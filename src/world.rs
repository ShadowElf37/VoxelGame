use std::time::SystemTime;

use crate::camera;
use crate::entity::*;
use crate::geometry;

const ENTITY_LIMIT: usize = 128;
const RENDER_DISTANCE: usize = 2;
const RENDER_AREA: usize = 4*RENDER_DISTANCE*RENDER_DISTANCE;

use glam::f32::{Vec3};
use crate::block;
use crate::memarena::Arena;

pub struct World {
    pub chunks: Arena<block::Chunk>,
    pub entities: Arena<Entity>,

    pub spawn_point: Vec3,
    pub sky_color: [f32; 4],
    pub player: Entity,
}

impl World {
    pub fn new() -> Self {
        return Self {
            // render distance changing is easy. `chunks = Arena::from_iter(chunks.iter())`. then, ensure Arena::drop() works.
            chunks: Arena::<block::Chunk>::new(RENDER_AREA),//Vec::<block::Chunk>::with_capacity(RENDER_AREA), 
            entities: Arena::<Entity>::new(ENTITY_LIMIT),

            spawn_point: Vec3::new(0.0, 0.0, 0.0),
            player: Entity::new(Vec3::new(0.0, 0.0, 5.0)),
            sky_color: [0.58, 0.93, 0.95, 1.0],
        };
    }

    pub fn generate_all_chunks_around_player(&mut self) {
        for x in -(RENDER_DISTANCE as i64)..RENDER_DISTANCE as i64 {
            for y in -(RENDER_DISTANCE as i64)..RENDER_DISTANCE as i64 {
                let mut new_chunk = block::Chunk::new(x as f32 * block::CHUNK_SIZE_F, y as f32 * block::CHUNK_SIZE_F, 0.0);
                new_chunk.generate_flat();
                println!("chunk size 1 {}", std::mem::size_of::<block::Chunk>());
                println!("chunk size 2 {}", std::mem::size_of_val::<block::Chunk>(&new_chunk));
                //self.chunks.push(new_chunk);
                self.chunks.create(new_chunk).unwrap();
            }
        }
    }
    pub fn get_all_chunk_meshes(&mut self) -> (Vec<geometry::Vertex>, Vec<u32>) {
        let mut vertices = Vec::<geometry::Vertex>::new();
        let mut indices = Vec::<u32>::new();
        let mut indices_offset = 0u32;

        for chunk in self.chunks.iter() {
            let (v, i) = chunk.read().unwrap().get_mesh(indices_offset);
            indices_offset += v.len() as u32;
            vertices.extend(v);
            indices.extend(i);
        }

        (vertices, indices)
    }

    fn do_physics(dt: f32, e: &mut Entity) {
        let mut dp: Vec3 = Vec3::ZERO;
        if e.flying || !e.in_air {
            dp += e.get_desired_velocity() * dt;
        }
        dp += e.vel * dt;
        e.pos += dp;
    }

    pub fn physics_step(&mut self, dt: f32) {
        Self::do_physics(dt, &mut self.player);
        /*for opt_e in self.entities.iter_mut() {
            match opt_e {
                Some(e) => {
                    // physics
                    Self::do_physics(dt, e);
                }
                None => ()
            }
        }*/
    }
}

