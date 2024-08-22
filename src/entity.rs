extern crate glam;

use crate::block::BlockID;
use glam::{Mat3, Vec2, Vec3};
use crate::world;
use std::sync::{Arc, RwLock};

const DEG_TO_RAD: f32 = 0.0174532925;

#[derive(Clone)]
pub struct DesiredMovement {
    pub forward: bool,
    pub backward: bool,
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
    pub sprint: bool,
}

const NO_MOVEMENT: DesiredMovement = DesiredMovement {
    forward: false,
    backward: false,
    right: false,
    left: false,
    up: false,
    down: false,
    sprint: false,
};

#[derive(Clone)]
pub struct Entity {
    pub pos: Vec3,
    pub vel: Vec3,
    pub acc: Vec3,
    pub facing: Vec3,

    pub eye_height: f32,
    pub height: f32,
    pub width: f32,

    pub move_speed: f32,
    pub jump_height: f32,
    pub acc_rate: f32,
    pub gravity: f32,
    pub desired_movement: DesiredMovement,

    pub flying: bool,
    pub in_air: bool,
}

impl Entity {
    pub fn new(pos: glam::f32::Vec3) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            pos,
            vel: Vec3::new(0.0, 0.0, 0.0),
            acc: Vec3::new(0.0, 0.0, 0.0),
            facing: Vec3::new(0.0, 1.0, 0.0).normalize(),

            eye_height: 1.6,
            height: 1.8,
            width: 0.1,

            move_speed: 4.3,
            jump_height: 1.3,
            acc_rate: 130.0,
            gravity: 9.8 * 2.5,
            desired_movement: NO_MOVEMENT,

            flying: true,
            in_air: true,
        }))
    }

    pub fn get_block_looking_at(&self, world: &world::World) -> (Vec3, Vec3, BlockID) {
        world.cast_ray_to_first_non_air_block(self.pos + Vec3::Z * self.eye_height, self.facing, 4.0)
    }

    pub fn facing_in_degrees(&self) -> Vec2 {
        Vec2::new(
            self.facing.z.asin() / DEG_TO_RAD, // vertical
            self.facing.y.atan2(self.facing.x) / DEG_TO_RAD, // horizontal
        )
    }

    pub fn get_rightward_vector(&self) -> Vec3 {
        self.facing.cross(Vec3::Z).normalize()
    }

    pub fn turn_horizontal(&mut self, amount_deg: f32) {
        self.facing = Mat3::from_rotation_z(-amount_deg * DEG_TO_RAD) * self.facing;
    }

    pub fn turn_vertical(&mut self, amount_deg: f32) {
        let new_facing = Mat3::from_axis_angle(self.get_rightward_vector(), -amount_deg * DEG_TO_RAD) * self.facing;
        if new_facing.dot(Vec3::Z).abs() < 0.999 {
            self.facing = new_facing;
        }
    }

    pub fn update_time_independent_acceleration(&mut self) {
        self.acc = Vec3::ZERO;

        if self.in_air && !self.flying {
            self.acc_rate = 10.0;
        } else {
            self.acc_rate = 150.0;
        }

        let sprint_factor = if self.desired_movement.sprint { 2.0 } else { 1.0 };

        if self.desired_movement.forward {
            self.acc += self.get_moving_forward_xy(1.0) * sprint_factor;
        }
        if self.desired_movement.backward {
            self.acc += self.get_moving_forward_xy(-1.0) * sprint_factor;
        }
        if self.desired_movement.right {
            self.acc += self.get_moving_rightward(1.0) * sprint_factor;
        }
        if self.desired_movement.left {
            self.acc += self.get_moving_rightward(-1.0) * sprint_factor;
        }

        if self.flying {
            if self.desired_movement.up {
                self.acc += self.get_moving_up(1.0);
            }
            if self.desired_movement.down {
                self.acc += self.get_moving_up(-1.0);
            }
        } else {
            if self.in_air {
                self.acc.z -= self.gravity;
            } else if self.desired_movement.up {
                self.vel.z = (self.jump_height * self.gravity * 2.0).sqrt();
            }
        }
    }

    pub fn get_moving_forward(&self, fac: f32) -> Vec3 {
        fac * self.acc_rate * self.facing
    }

    pub fn get_moving_forward_xy(&self, fac: f32) -> Vec3 {
        fac * self.acc_rate * self.facing.with_z(0f32).normalize()
    }

    pub fn get_moving_rightward(&self, fac: f32) -> Vec3 {
        fac * self.acc_rate * self.get_rightward_vector()
    }

    pub fn get_moving_up(&self, fac: f32) -> Vec3 {
        fac * self.acc_rate * Vec3::Z
    }

    pub fn clear_moving(&mut self) {
        self.desired_movement = NO_MOVEMENT;
    }
}