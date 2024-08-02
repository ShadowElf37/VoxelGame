use crate::camera;
use crate::entity::*;

const ENTITY_LIMIT: usize = 128;

extern crate glam;
use glam::f32::{Vec3};
use crate::entity;

pub struct GameState {
    pub paused: bool,
    pub in_game: bool,
}


pub struct World {
    pub game_state: GameState,

    pub entities: Vec<Option<Entity>>,
    pub spawn_point: Vec3,

    camera_hook: usize,
    pub camera: camera::Camera,

    ticks_per_second: u64,
}

impl World {
    pub fn new() -> Self {
        return Self {
            game_state: GameState {
                paused: false,
                in_game: true,
            },

            entities: vec![None as Option<Entity>; ENTITY_LIMIT],
            spawn_point: Vec3::new(1.0, 1.0, 0.0),

            camera_hook: 0,
            camera: camera::Camera::new(),
            ticks_per_second: 100,
        };
    }

    pub fn get_camera_entity(&self) -> &Option<entity::Entity> {
        &self.entities[self.camera_hook]
    }

    pub fn physics_step(&mut self, dt: f32) {
        for opt_e in self.entities.iter_mut() {
            match opt_e {
                Some(e) => {
                    // physics
                    let dp = e.get_desired_velocity() * dt;
                    e.pos += dp;
                }
                None => ()
            }
        }
    }

    pub fn spawn_at_sp(&mut self) -> Option<usize> {
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

