use std::time::SystemTime;
use std::sync::{Arc, Mutex, RwLock};
use std::collections::VecDeque;
use glam::f32::Vec3;
use rayon::prelude::*;
use crate::block;
use crate::block::BlockID;
use crate::chunk::{Chunk, CHUNK_SIZE_F};
use crate::entity::*;
use crate::memarena::{Arena, ArenaHandle};

const ENTITY_LIMIT: usize = 128;
pub const RENDER_DISTANCE: usize = 10;
pub const RENDER_VOLUME: usize = 8*RENDER_DISTANCE*RENDER_DISTANCE*RENDER_DISTANCE;

const LOAD_RADIUS: f32 = 2.0;

pub struct World {
    pub chunks: Arena<Arc<RwLock<Chunk>>>,
    pub entities: Arena<Arc<RwLock<Entity>>>,

    pub block_properties: block::BlockProtoSet,

    pub spawn_point: Vec3,
    pub sky_color: [f32; 4],
    pub player: ArenaHandle<Arc<RwLock<Entity>>>,
    last_player_chunk_coords: Option<[i32; 3]>,

    pub need_mesh_update: Mutex<VecDeque<ArenaHandle<Arc<RwLock<Chunk>>>>>,
    pub need_generation_update: Mutex<VecDeque<ArenaHandle<Arc<RwLock<Chunk>>>>>,
    thread_pool: Arc<rayon::ThreadPool>,
}

impl Clone for World {
    fn clone(&self) -> Self {
        World {
            chunks: self.chunks.clone(),
            entities: self.entities.clone(),
            block_properties: self.block_properties.clone(),
            spawn_point: self.spawn_point.clone(),
            sky_color: self.sky_color.clone(),
            player: self.player.clone(),
            last_player_chunk_coords: self.last_player_chunk_coords.clone(),
            need_mesh_update: Mutex::new(self.need_mesh_update.lock().unwrap().clone()),
            need_generation_update: Mutex::new(self.need_generation_update.lock().unwrap().clone()),
            thread_pool: self.thread_pool.clone(),
        }
    }
}

impl World {
    pub fn new() -> Self {
        let mut entities = Arena::<Arc<RwLock<Entity>>>::new(ENTITY_LIMIT);
        let player = entities.create(Entity::new(Vec3::new(0.0, 0.0, 32.0))).unwrap();
        let thread_pool = Arc::new(rayon::ThreadPoolBuilder::new().build().unwrap());
        println!("Created thread pool with {} threads", thread_pool.current_num_threads());
        Self {
            chunks: Arena::new(RENDER_VOLUME),
            entities,
            block_properties: block::BlockProtoSet::new(),
            spawn_point: Vec3::new(0.0, 0.0, 32.0),
            sky_color: [0.5, 0.7, 1.0, 1.0],
            player,
            last_player_chunk_coords: None,
            need_mesh_update: Mutex::new(VecDeque::new()),
            need_generation_update: Mutex::new(VecDeque::new()),
            thread_pool,
        }
    }

    // return the first non air block you hit (where you want to destroy a block), the last air block you hit (where you want to place a block), and the block id at that spot
    pub fn cast_ray_to_first_non_air_block(&self, start_pos: Vec3, facing: Vec3, max_distance: f32) -> (Vec3, Vec3, BlockID) {
        let midpoint_offset = Vec3::new(0.5, 0.5, 0.5);
        let max_distance = max_distance*max_distance;
        let dirs_manhattan = {
            let facing_manhattan = facing.signum();
            [facing_manhattan.x*Vec3::X, facing_manhattan.y*Vec3::Y, facing_manhattan.z*Vec3::Z]
        };

        let mut block_id: BlockID = 0;
        let mut ray_pos = start_pos.floor()+midpoint_offset;
        let mut ray = ray_pos-start_pos;
        let mut last_ray_pos = start_pos;

        while block_id == 0 && ray.length_squared() < max_distance {
            let candidates = [ray + dirs_manhattan[0], ray + dirs_manhattan[1], ray + dirs_manhattan[2]];
            let errors = candidates.iter().map(|cand| cand.cross(facing).length_squared());
            let best_index = errors.enumerate().min_by(|(_, a), (_, b)| a.total_cmp(b)).map(|(index, _)| index).unwrap();

            ray = candidates[best_index];
            last_ray_pos = ray_pos;
            ray_pos = start_pos+ray;
            block_id = self.get_block_id_at(ray_pos.x, ray_pos.y, ray_pos.z);
        }

        (ray_pos-midpoint_offset, last_ray_pos-midpoint_offset, block_id)
    }

    pub fn get_chunk_at(&self, x: f32, y: f32, z: f32) -> Option<ArenaHandle<Arc<RwLock<Chunk>>>> {
        for handle in self.chunks.iter() {
            if let Ok(chunk_guard) = self.chunks.read_lock(handle) {
                let chunk = chunk_guard.read().unwrap(); // Dereference the Arc and RwLock
                let chunk_read = chunk.read().unwrap(); // Store the result of chunk_guard.read().unwrap() in a separate variable
                if chunk_read.check_inside_me(x, y, z) {
                    return Some(handle);
                }
            }
        }
        None
    }

    pub fn get_block_id_at(&self, x: f32, y: f32, z: f32) -> BlockID {
        // returns 0 if the chunk isn't loaded
        match self.get_chunk_at(x, y, z) {
            Some(handle) => {
                if let Ok(chunk_guard) = self.chunks.read_lock(handle) {
                    let chunk = chunk_guard.read().unwrap(); // Dereference the Arc and RwLock
                    let chunk_read = chunk.read().unwrap(); // Store the result of chunk_guard.read().unwrap() in a separate variable
                    chunk_read.get_block_id_at(x, y, z)
                } else {
                    0
                }
            }
            None => 0,
        }
    }

    pub fn set_block_id_at(&mut self, x: f32, y: f32, z: f32, id: BlockID) -> Option<()> {
        // returns None and noops if the chunk isn't loaded
        match self.get_chunk_at(x, y, z) {
            Some(handle) => {
                if let Ok(chunk_guard) = self.chunks.write_lock(handle) {
                    let chunk = chunk_guard.write().unwrap(); // Dereference the Arc and RwLock
                    chunk.write().unwrap().set_block_id_at(x, y, z, id);
                    self.queue_mesh_update(handle);
                }
            }
            None => return None,
        }
        Some(())
    }

    pub fn queue_mesh_update(&mut self, handle: ArenaHandle<Arc<RwLock<Chunk>>>) {
        self.need_mesh_update.lock().unwrap().push_back(handle)
    }

    pub fn queue_chunk_update(&mut self, handle: ArenaHandle<Arc<RwLock<Chunk>>>) {
        self.need_generation_update.lock().unwrap().push_back(handle)
    }

    pub fn spawn_chunk_updates(&self) {
        let mut gen_update_lock = self.need_generation_update.lock().unwrap();
        loop {
            match gen_update_lock.pop_front() {
                None => break,
                Some(handle) => {
                    let chunk = self.chunks.fetch_lock(handle).unwrap();
                    let mesh_update = &self.need_mesh_update;
                    {
                        let wlock = chunk.write().unwrap();
                        let mut chunk_write = wlock.write().unwrap();
                        chunk_write.generate_planet();
                    } // The write lock is dropped here
                    mesh_update.lock().unwrap().push_back(handle);
                }
            }
        }
    }

    pub fn spawn_mesh_updates(&self) -> bool {
        let mut got_any_updates = false;
        let mut mesh_update_lock = self.need_mesh_update.lock().unwrap();
        loop {
            match mesh_update_lock.pop_front() {
                None => return got_any_updates,
                Some(handle) => {
                    got_any_updates = true;
                    let chunk = self.chunks.fetch_lock(handle).unwrap();
                    let chunk_write = chunk.write().unwrap();
                    let block_properties = &self.block_properties;
                    let tp = &self.thread_pool;
                    chunk_write.write().unwrap().make_mesh(block_properties, tp);
                    chunk_write.write().unwrap().ready_to_display = true;
                }
            }
        }
    }

    pub fn generate_chunk(&mut self, x: f32, y: f32, z: f32) {
        let new_chunk = Arc::new(RwLock::new(Chunk::new(x, y, z)));
        let handle = self.chunks.create(new_chunk).unwrap();
        self.queue_chunk_update(handle);
    }

    pub fn generate_all_chunks_around_player(&self) {
        let (px, py, pz) = self.get_player_chunk_coords();
        let chunk_coords: Vec<(i32, i32, i32)> = ((px - RENDER_DISTANCE as i32)..(px + RENDER_DISTANCE as i32))
            .flat_map(|x| ((py - RENDER_DISTANCE as i32)..(py + RENDER_DISTANCE as i32))
                .flat_map(move |y| ((pz - RENDER_DISTANCE as i32)..(pz + RENDER_DISTANCE as i32))
                    .map(move |z| (x, y, z))))
            .collect();
    
        let world_arc = Arc::new(RwLock::new(self.clone())); // Use RwLock for thread-safe access
        let thread_pool = self.thread_pool.clone();
        
        thread_pool.install(|| {
            chunk_coords.into_par_iter().for_each(move |(x, y, z)| {
                let world = world_arc.clone();
                let mut world = world.write().unwrap();
                if !world.is_chunk_loaded(x, y, z) {
                    world.generate_chunk(x as f32, y as f32, z as f32);
                }
            });
        });
    }
    
    pub fn get_all_chunk_meshes(&mut self, device: &wgpu::Device) {
        for handle in self.chunks.iter() {
            let chunk = self.chunks.fetch_lock(handle).unwrap();
            if chunk.read().unwrap().read().unwrap().vertex_buffer.is_none() {
                self.thread_pool.install(||{
                    chunk.write().unwrap().write().unwrap().make_vertex_buffer(device);
                });
            }
        }
    }    
    fn get_player_chunk_coords(&self) -> (i32, i32, i32) {
        let player_pos = {
            let player_entity = self.entities.read_lock(self.player).unwrap();
            let player = player_entity.read().unwrap();
            let player_inner = player.read().unwrap();
            player_inner.pos
        };
        (
            (player_pos.x / CHUNK_SIZE_F).floor() as i32,
            (player_pos.y / CHUNK_SIZE_F).floor() as i32,
            (player_pos.z / CHUNK_SIZE_F).floor() as i32,
        )
    }

    pub fn update_loaded_chunks(&mut self) {
        let (px, py, pz) = self.get_player_chunk_coords();
        if self.last_player_chunk_coords.is_some() && [px, py, pz] == self.last_player_chunk_coords.unwrap() {
            return;
        }
        self.last_player_chunk_coords = Some([px, py, pz]);

        let start_time = SystemTime::now();

        let chunks_to_unload: Vec<ArenaHandle<Arc<RwLock<Chunk>>>> = self.chunks.iter()
            .filter(|handle| {
                let chunk = self.chunks.read_lock(*handle).unwrap();
                let (cx, cy, cz) = chunk.read().unwrap().read().unwrap().integer_chunk_coords();
                let dist = ((cx - px).pow(2) + (cy - py).pow(2) + (cz - pz).pow(2)) as f32;
                dist > (RENDER_DISTANCE as f32).powi(2)
            })
            .collect();

        for handle in chunks_to_unload {
            self.chunks.destroy(handle).unwrap();
        }

        let chunk_coords: Vec<(i32, i32, i32)> = (px - RENDER_DISTANCE as i32..px + RENDER_DISTANCE as i32)
            .flat_map(|x| (py - RENDER_DISTANCE as i32..py + RENDER_DISTANCE as i32)
                .flat_map(move |y| (pz - RENDER_DISTANCE as i32..pz + RENDER_DISTANCE as i32)
                    .map(move |z| (x, y, z))))
            .collect();

        let world_arc = Arc::new(RwLock::new(self.clone())); // Use RwLock for thread-safe access
        chunk_coords.into_par_iter().for_each(|(x, y, z)| {
            let world = world_arc.clone();
            let mut world = world.write().unwrap();
            if !world.is_chunk_loaded(x, y, z) {
                world.generate_chunk(x as f32, y as f32, z as f32);
            }
        });

        let end_time = SystemTime::now();
        println!("Chunk generation took {:?}", end_time.duration_since(start_time).unwrap());
    }

    fn is_chunk_loaded(&self, x: i32, y: i32, z: i32) -> bool {
        self.chunks.iter().any(|handle| {
            let chunk = self.chunks.read_lock(handle).unwrap();
            let result = chunk.read().unwrap().read().unwrap().integer_chunk_coords() == (x, y, z);
            std::mem::drop(chunk);
            result
        })
    }

    fn do_physics(&self, dt: f32, e: ArenaHandle<Arc<RwLock<Entity>>>) {
        let e = self.entities.write_lock(e).unwrap();
        let mut dx = Vec3::ZERO;
        let mut dv = Vec3::ZERO;
        let (x, y, z) = {
            let e_inner = e.read().unwrap();
            let entity = e_inner.read().unwrap();
            (entity.pos.x, entity.pos.y, entity.pos.z)
        };

        if e.read().unwrap().read().unwrap().vel.z.abs() > 100.0 {
            if let Ok(entity_guard) = e.write() {
                let mut entity = entity_guard.write().unwrap(); // Acquire a mutable reference to the inner Entity struct
                entity.pos = Vec3::new(0.0, 0.0, 10.0); // Access the field on the Entity
            }
            (*e.write().unwrap().write().unwrap()).vel = Vec3::ZERO;
            if let Ok(entity_guard) = e.write() {
                let mut entity = entity_guard.write().unwrap();
                entity.facing = Vec3::new(0.0, 1.0, 0.0);
            }
        }

        //let entity_chunk = self.get_chunk_at(x, y, z);

        let entity = e.read().unwrap();
        let mut inner_entity = entity.write().unwrap();
        inner_entity.update_time_independent_acceleration();

        if true { // !e.in_air {
            let decel = e.read().unwrap().read().unwrap().vel.with_z(0.0)*e.read().unwrap().read().unwrap().acc_rate/e.read().unwrap().read().unwrap().move_speed;
            if let Ok(entity_guard) = e.write() {
                entity_guard.write().unwrap().acc -= decel;
            }
        }
        dv += e.read().unwrap().read().unwrap().acc * dt;
        dx += (e.read().unwrap().read().unwrap().vel+dv) * dt;
        
        //let dx_dir = dx.normalize()*0.1;

        // this will break for high dx (high v)
        let future_pos = e.read().unwrap().read().unwrap().pos+dx+dx.signum()*Vec3::new(e.read().unwrap().read().unwrap().width, e.read().unwrap().read().unwrap().width, 0.0);
        let (fx, fy, fz) = (future_pos.x, future_pos.y, future_pos.z);

        if self.block_properties.by_id(self.get_block_id_at(fx, y, z)).solid || self.block_properties.by_id(self.get_block_id_at(fx, y, z+1.0)).solid {
            dx = dx.with_x(0.0);
            dv = dv.with_x(-e.read().unwrap().read().unwrap().vel.x);
        }
        if self.block_properties.by_id(self.get_block_id_at(x, fy, z)).solid || self.block_properties.by_id(self.get_block_id_at(x, fy, z+1.0)).solid {
            dx = dx.with_y(0.0);
            dv = dv.with_y(-e.read().unwrap().read().unwrap().vel.y);
        }
        let mut entity_guard = None;
        if let Ok(guard) = e.write() {
            entity_guard = Some(guard);
        }
        if let Some(entity_guard) = entity_guard {
            let mut entity = entity_guard.write().unwrap(); // Acquire a mutable reference to the inner Entity struct
        
            if self.block_properties.by_id(self.get_block_id_at(x, y, fz)).solid {
                dx = dx.with_z(0.0);
                dv = dv.with_z(-entity.vel.z); // Access the field on the Entity
                entity.in_air = false; // Access the field on the Entity
            } else {
                entity.in_air = true; // Access the field on the Entity
            }
            if self.block_properties.by_id(self.get_block_id_at(x, y, fz + entity.height)).solid {
                dx = dx.with_z(0.0);
                dv = dv.with_z(-entity.vel.z); // Access the field on the Entity
            }
        
            entity.vel += dv; // Access the field on the Entity
            entity.pos += dx; // Access the field on the Entity
        }
    }

    pub fn physics_step(&mut self, dt: f32) {
        for e in self.entities.iter() {
            self.do_physics(dt, e);
        }
    }
}

