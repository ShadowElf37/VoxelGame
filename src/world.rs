use crate::block::BlockID;
use std::time::SystemTime;
use std::sync::RwLock;
use crate::camera;
use crate::entity::*;
use crate::geometry;

const ENTITY_LIMIT: usize = 128;
const RENDER_DISTANCE: usize = 4;
const RENDER_VOLUME: usize = 8*RENDER_DISTANCE*RENDER_DISTANCE*RENDER_DISTANCE;

use glam::f32::{Vec3};
use crate::block;
use crate::memarena::Arena;
use crate::chunk::{Chunk, CHUNK_SIZE_F};

pub struct World {
    pub chunks: Arena<Chunk>,
    pub entities: Arena<Entity>,

    pub block_properties: block::BlockProtoSet,

    pub spawn_point: Vec3,
    pub sky_color: [f32; 4],
    pub player: RwLock<Entity>,

    pub need_block_update: bool,
}

impl World {
    pub fn new() -> Self {
        return Self {
            // render distance changing is easy. `chunks = Arena::from_iter(chunks.iter())`. then, ensure Arena::drop() works.
            chunks: Arena::<Chunk>::new(RENDER_VOLUME),//Vec::<block::Chunk>::with_capacity(RENDER_AREA), 
            entities: Arena::<Entity>::new(ENTITY_LIMIT),

            block_properties: block::BlockProtoSet::from_toml("config/blocks.toml"),

            spawn_point: Vec3::new(0.0, 0.0, 0.0),
            player: RwLock::new(Entity::new(Vec3::new(0.0, 0.0, 5.0))),
            sky_color: [155./255., 230./255., 255./255., 1.0],

            need_block_update: true,
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
            block_id = self.get_block_id_at(ray_pos.x, ray_pos.y, ray_pos.z);
        }

        (ray_pos-midpoint_offset, last_ray_pos-midpoint_offset, block_id)
    }

    pub fn get_chunk_at(&self, x: f32, y: f32, z: f32) -> Option<&RwLock<Chunk>> {
        for chunk in self.chunks.iter() {
            let chunk_rw = chunk.read().unwrap();
            if chunk_rw.check_inside_me(x, y, z) {
                return Some(chunk)
            }
        }
        None
    }

    pub fn get_block_id_at(&self, x: f32, y: f32, z: f32) -> BlockID {
        // returns 0 if the chunk isn't loaded
        match self.get_chunk_at(x, y, z) {
            Some(chunk) => {
                chunk.read().unwrap().get_block_id_at(x, y, z)
            }
            None => 0
        }
    }
    pub fn set_block_id_at(&mut self, x: f32, y: f32, z: f32, id: BlockID) -> Option<()> {
        // returns None and noops if the chunk isn't loaded
        match self.get_chunk_at(x, y, z) {
            Some(chunk) => {
                chunk.write().unwrap().set_block_id_at(x, y, z, id)
            }
            None => return None
        }
        self.need_block_update = true;
        Some(())
    }

    pub fn generate_all_chunks_around_player(&mut self) {
        for x in -(RENDER_DISTANCE as isize)..RENDER_DISTANCE as isize {
            for y in -(RENDER_DISTANCE as isize)..RENDER_DISTANCE as isize {
                for z in -(RENDER_DISTANCE as isize)..RENDER_DISTANCE as isize {
                    println!("Chunk generated at {} {} {}", x, y, z);
                    let mut new_chunk = Chunk::new(x as f32 * CHUNK_SIZE_F, y as f32 * CHUNK_SIZE_F, z as f32 * CHUNK_SIZE_F);
                    new_chunk.generate_planet();
                    self.chunks.create(new_chunk).unwrap();
                }
            }
        }
    }
    
    pub fn get_all_chunk_meshes(&mut self) -> (Vec<geometry::Vertex>, Vec<u32>) {
        let mut vertices = Vec::<geometry::Vertex>::new();
        let mut indices = Vec::<u32>::new();
        let mut indices_offset = 0u32;

        for chunk in self.chunks.iter() {
            let (v, i) = chunk.read().unwrap().get_mesh(indices_offset, &self.block_properties);
            indices_offset += v.len() as u32;
            vertices.extend(v);
            indices.extend(i);
        }

        (vertices, indices)
    }

    fn do_physics(&self, dt: f32, e: &RwLock<Entity>) {
        let mut e = e.write().unwrap();
        let mut dx = Vec3::ZERO;
        let mut dv = Vec3::ZERO;
        let (x, y, z) = (e.pos.x, e.pos.y, e.pos.z);

        if z < -32.0 {
            e.pos = Vec3::new(0.0, 0.0, 10.0);
            e.vel.z = 0.0;
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
        self.do_physics(dt, &self.player);
        for e in self.entities.iter() {
            self.do_physics(dt, e);
        }
    }
}

