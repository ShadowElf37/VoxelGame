use crate::camera;
use crate::entity::*;

const ENTITY_LIMIT: usize = 128;

extern crate glam;
use glam::f32::{Vec3};
use crate::entity;

pub struct World {
    pub entities: Vec<Option<Entity>>,
    pub spawn_point: Vec3,
    pub sky_color: [f32; 4],
    pub player: Entity,
    ticks_per_second: u64,
}

impl World {
    pub fn new() -> Self {
        return Self {
            entities: vec![None as Option<Entity>; ENTITY_LIMIT],
            spawn_point: Vec3::new(0.5, -1.0, -1.0),
            player: Entity::new(Vec3::new(0.5, -1.0, -1.0)),

            sky_color: [0.58, 0.93, 0.95, 1.0],
            ticks_per_second: 100,
        };
    }

    fn do_physics(dt: f32, e: &mut entity::Entity) {
        let mut dp: Vec3 = Vec3::ZERO;
        if e.flying || !e.in_air {
            dp += e.get_desired_velocity() * dt;
        }
        dp += e.vel * dt;
        e.pos += dp;
    }

    pub fn physics_step(&mut self, dt: f32) {
        Self::do_physics(dt, &mut self.player);
        for opt_e in self.entities.iter_mut() {
            match opt_e {
                Some(e) => {
                    // physics
                    Self::do_physics(dt, e);
                }
                None => ()
            }
        }
    }

    pub fn spawn_entity_at_spawnpoint(&mut self) -> Option<usize> {
        self.spawn_entity(self.spawn_point)
    }

    pub fn spawn_entity(&mut self, pos: glam::f32::Vec3) -> Option<usize> {
        for (i, mut entity) in self.entities.iter_mut().enumerate() {
            if entity.is_none() {
                *entity = Some(Entity::new(pos));
                return Some(i);
            }
        }
        return None; // out of memory!
    }
    pub fn kill_entity(&mut self, i: usize) -> bool {
        if self.entities[i].is_some() {return false;};
        self.entities[i] = None;
        return true;
    }
    
}

