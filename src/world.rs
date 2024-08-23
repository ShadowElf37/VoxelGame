use ndarray::NdIndex;
use std::mem::MaybeUninit;
use std::sync::RwLock;
use std::sync::Arc;
use crate::block::BlockID;
use std::time::SystemTime;
use std::sync::Mutex;
use crate::camera;
use crate::entity::*;
use crate::geometry;
use std::collections::VecDeque;
use glam::f32::{Vec3};
use crate::block;
use crate::memarena::{Arena, ArenaHandle};
use crate::chunk::{Chunk, CHUNK_SIZE_F};
use crate::chunkset::{ChunkSet, ChunkCoord};
use ndarray::prelude::*;
use ndarray::{Array3};


const ENTITY_LIMIT: usize = 128;
pub const RENDER_DISTANCE: usize = 10;


pub struct World {
    pub chunks: ChunkSet,

    pub entities: Arena<Entity>,

    pub block_properties: block::BlockProtoSet,

    pub spawn_point: Vec3,
    pub sky_color: [f32; 4],
    pub player: ArenaHandle<Entity>,
    last_player_chunk_coords: Option<ChunkCoord>,

    pub need_mesh_update: Mutex<VecDeque<ArenaHandle<Chunk>>>,
    pub need_generation_update: Mutex<VecDeque<ArenaHandle<Chunk>>>,
    thread_pool: rayon::ThreadPool,
}

impl World {
    pub fn new() -> Self {
        let spawn_pos = Vec3::new(0.0, 0.0, 32.0);
        let mut entities = Arena::<Entity>::new(ENTITY_LIMIT);
        let player = entities.create(Entity::new(spawn_pos)).unwrap();
        let thread_pool = rayon::ThreadPoolBuilder::new().build().unwrap();
        println!("Created threadpool with {} threads", thread_pool.current_num_threads());
        return Self {
            // render distance changing is easy. `chunks = Arena::from_iter(chunks.iter())`. then, ensure Arena::drop() works.
            
            chunks: ChunkSet::new((0, 0, 32), RENDER_DISTANCE),
            entities,

            block_properties: block::BlockProtoSet::from_toml("config/blocks.toml"),

            spawn_point: Vec3::new(0.0, 0.0, 0.0),
            player,
            sky_color: [155./255., 230./255., 255./255., 1.0],
            last_player_chunk_coords: None,

            need_mesh_update: Mutex::new(VecDeque::new()),
            need_generation_update: Mutex::new(VecDeque::new()),
            thread_pool,
        };
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
            block_id = self.get_block_id_at(ray_pos);
        }

        (ray_pos-midpoint_offset, last_ray_pos-midpoint_offset, block_id)
    }

    

    pub fn get_block_id_at(&self, pos: Vec3) -> BlockID {
        // returns 0 if the chunk isn't loaded
        match self.get_chunk_at(pos) {
            Some(lock) => {
                match lock.read() {
                    Ok(chunk) => chunk.get_block_id_at(pos),
                    Err(_) => 0
                }
            }
            None => 0
        }
    }
    pub fn set_block_id_at(&mut self, pos: Vec3, id: BlockID, device: &wgpu::Device) -> Option<()> {
        // returns None and noops if the chunk isn't loaded
        match self.get_chunk_at(pos) {
            Some(lock) => {
                self.thread_pool.install(||{
                    let mut chunk = lock.write().unwrap();
                    chunk.set_block_id_at(pos, id);
                    chunk.make_mesh(&self.block_properties, &self.thread_pool);
                    chunk.make_vertex_buffer(device);
                    drop(chunk);
                })
                
                //self.queue_mesh_update(handle);
            }
            None => return None
        }
        Some(())
    }

    pub fn get_chunk_at(&self, pos: Vec3) -> Option<&RwLock<Chunk>> {
        self.chunks.get_chunk_at_world_coords(pos)
    }

    fn get_player_chunk_coords(&self) -> (isize, isize, isize) {
        self.chunks.world_to_chunk_coords(self.entities.read_lock(self.player).unwrap().pos)
    }

    pub fn update_loaded_chunks(&mut self, device: &wgpu::Device) {

        let pcp = self.get_player_chunk_coords();
        self.chunks.recenter(pcp);

        if self.last_player_chunk_coords.is_some() && pcp == self.last_player_chunk_coords.unwrap() {
            return
        }
        //let delta = 
        self.last_player_chunk_coords = Some(pcp);

        // GENERATE ANY CHUNKS THAT HAVEN'T BEEN LOADED YET
        let mut to_unload = Vec::<ChunkCoord>::new();
        for lock in self.chunks.iter() {
            let cp = self.chunks.world_to_chunk_coords(lock.read().unwrap().pos);
            if !self.chunks.check_in_bounds(cp) {
                to_unload.push(cp);
            }
        }

        for cp in to_unload.into_iter() {
            self.chunks.mark_unloaded(cp);
        }

        //let mut i = 0;
        for x in (pcp.0 - RENDER_DISTANCE as isize)..(pcp.0 + RENDER_DISTANCE as isize) {
            for y in (pcp.1 - RENDER_DISTANCE as isize)..(pcp.1 + RENDER_DISTANCE as isize) {
                for z in (pcp.2 - RENDER_DISTANCE as isize)..(pcp.2 + RENDER_DISTANCE as isize) {
                    if self.chunks.is_unloaded((x,y,z)) {
                        //i += 1;
                        self.chunks.generate_chunk((x, y, z), &self.thread_pool, &self.block_properties, device);
                    }
                }
            }
        }
        //println!("Created {} chunks", i);
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

        if self.block_properties.by_id(self.get_block_id_at(Vec3::new(fx, y, z))).solid || self.block_properties.by_id(self.get_block_id_at(Vec3::new(fx, y, z+1.0))).solid {
            dx = dx.with_x(0.0);
            dv = dv.with_x(-e.vel.x);
        }
        if self.block_properties.by_id(self.get_block_id_at(Vec3::new(x, fy, z))).solid || self.block_properties.by_id(self.get_block_id_at(Vec3::new(x, fy, z+1.0))).solid {
            dx = dx.with_y(0.0);
            dv = dv.with_y(-e.vel.y);
        }
        if self.block_properties.by_id(self.get_block_id_at(Vec3::new(x, y, fz))).solid {
            dx = dx.with_z(0.0);
            dv = dv.with_z(-e.vel.z);
            e.in_air = false;
        } else {
            e.in_air = true;
        }
        if self.block_properties.by_id(self.get_block_id_at(Vec3::new(x, y, fz+e.height))).solid {
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

