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

impl World {
    pub fn new() -> Self {
        let mut entities = Arena::<Arc<RwLock<Entity>>>::new(ENTITY_LIMIT);
        let player = entities.create(Arc::new(RwLock::new(Entity::new(Vec3::new(0.0, 0.0, 32.0))))).unwrap();
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

    pub fn get_chunk_at(&self, x: f32, y: f32, z: f32) -> Option<ArenaHandle<Chunk>> {
        for handle in self.chunks.iter() {
            if self.chunks.read_lock(handle).unwrap().check_inside_me(x, y, z) {
                return Some(handle)
            }
        }
        None
    }

    pub fn get_block_id_at(&self, x: f32, y: f32, z: f32) -> BlockID {
        // returns 0 if the chunk isn't loaded
        match self.get_chunk_at(x, y, z) {
            Some(handle) => {
                self.chunks.read_lock(handle).unwrap().get_block_id_at(x, y, z)
            }
            None => 0
        }
    }
    pub fn set_block_id_at(&mut self, x: f32, y: f32, z: f32, id: BlockID) -> Option<()> {
        // returns None and noops if the chunk isn't loaded
        match self.get_chunk_at(x, y, z) {
            Some(handle) => {
                self.chunks.write_lock(handle).unwrap().set_block_id_at(x, y, z, id);
                self.queue_mesh_update(handle);
            }
            None => return None
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
                    let mut wlock = chunk.write().unwrap();
                    wlock.generate_planet();
                    drop(wlock);
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
                    let block_properties = &self.block_properties;
                    let tp = &self.thread_pool;
                    let mut wlock = chunk.write().unwrap();
                    wlock.make_mesh(block_properties, tp);
                    wlock.ready_to_display = true;
                }
            }
        }
    }

    pub fn generate_chunk(&mut self, x: f32, y: f32, z: f32) {
        let new_chunk = Arc::new(RwLock::new(Chunk::new(x, y, z)));
        let handle = self.chunks.create(new_chunk).unwrap();
        self.queue_chunk_update(handle);
    }

    pub fn generate_all_chunks_around_player(&mut self) {
        let (px, py, pz) = self.get_player_chunk_coords();
        let chunk_coords: Vec<(i32, i32, i32)> = (px - RENDER_DISTANCE as i32..px + RENDER_DISTANCE as i32)
            .flat_map(|x| (py - RENDER_DISTANCE as i32..py + RENDER_DISTANCE as i32)
                .flat_map(move |y| (pz - RENDER_DISTANCE as i32..pz + RENDER_DISTANCE as i32)
                    .map(move |z| (x, y, z))))
            .collect();

        chunk_coords.par_iter().for_each(|&(x, y, z)| {
            if !self.is_chunk_loaded(x, y, z) {
                self.generate_chunk(x as f32, y as f32, z as f32);
            }
        });
    }
    
    pub fn get_all_chunk_meshes(&mut self, device: &wgpu::Device) {
        for handle in self.chunks.iter() {
            let chunk = self.chunks.fetch_lock(handle).unwrap();
            if chunk.read().unwrap().vertex_buffer.is_none() {
                self.thread_pool.install(||{
                    chunk.write().unwrap().make_vertex_buffer(device);
                });
            }
        }
    }

    fn get_player_chunk_coords(&self) -> (i32, i32, i32) {
        let player_pos = self.entities.read_lock(self.player).unwrap().pos;
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

        let chunks_to_unload: Vec<ArenaHandle<Chunk>> = self.chunks.iter()
            .filter(|handle| {
                let chunk = self.chunks.read_lock(*handle).unwrap();
                let (cx, cy, cz) = chunk.integer_chunk_coords();
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

        chunk_coords.par_iter().for_each(|&(x, y, z)| {
            if !self.is_chunk_loaded(x, y, z) {
                self.generate_chunk(x as f32, y as f32, z as f32);
            }
        });

        let end_time = SystemTime::now();
        println!("Chunk generation took {:?}", end_time.duration_since(start_time).unwrap());
    }

    fn is_chunk_loaded(&self, x: i32, y: i32, z: i32) -> bool {
        self.chunks.iter().any(|handle| {
            let chunk = self.chunks.read_lock(handle).unwrap();
            chunk.integer_chunk_coords() == (x, y, z)
        })
    }

    fn do_physics(&self, dt: f32, e: ArenaHandle<Entity>) {
        let mut e = self.entities.write_lock(e).unwrap();
        let mut dx = Vec3::ZERO;
        let mut dv = Vec3::ZERO;
        let (x, y, z) = (e.pos.x, e.pos.y, e.pos.z);

        if e.vel.z.abs() > 100.0 {
            e.pos = Vec3::new(0.0, 0.0, 10.0);
            e.vel = Vec3::ZERO;
            e.facing = Vec3::new(0.0, 1.0, 0.0);
        }

        //let entity_chunk = self.get_chunk_at(x, y, z);

        e.update_time_independent_acceleration();

        if true { // !e.in_air {
            let decel = e.vel.with_z(0.0)*e.acc_rate/e.move_speed;
            e.acc -= decel;
        }
        dv += e.acc * dt;
        dx += (e.vel+dv) * dt;
        
        //let dx_dir = dx.normalize()*0.1;

        // this will break for high dx (high v)
        let future_pos = e.pos+dx+dx.signum()*Vec3::new(e.width, e.width, 0.0);
        let (fx, fy, fz) = (future_pos.x, future_pos.y, future_pos.z);

        if self.block_properties.by_id(self.get_block_id_at(fx, y, z)).solid || self.block_properties.by_id(self.get_block_id_at(fx, y, z+1.0)).solid {
            dx = dx.with_x(0.0);
            dv = dv.with_x(-e.vel.x);
        }
        if self.block_properties.by_id(self.get_block_id_at(x, fy, z)).solid || self.block_properties.by_id(self.get_block_id_at(x, fy, z+1.0)).solid {
            dx = dx.with_y(0.0);
            dv = dv.with_y(-e.vel.y);
        }
        if self.block_properties.by_id(self.get_block_id_at(x, y, fz)).solid {
            dx = dx.with_z(0.0);
            dv = dv.with_z(-e.vel.z);
            e.in_air = false;
        } else {
            e.in_air = true;
        }
        if self.block_properties.by_id(self.get_block_id_at(x, y, fz+e.height)).solid {
            dx = dx.with_z(0.0);
            dv = dv.with_z(-e.vel.z);
        }

        e.vel += dv;
        e.pos += dx;
    }

    pub fn physics_step(&mut self, dt: f32) {
        for e in self.entities.iter() {
            self.do_physics(dt, e);
        }
    }
}

