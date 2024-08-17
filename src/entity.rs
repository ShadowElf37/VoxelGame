extern crate glam;

use crate::block::BlockID;
use glam::{
    Mat3, Vec2, Vec3,
};

use crate::world;

const DEG_TO_RAD: f32 = 0.0174532925;

#[derive(Clone)]
pub struct DesiredMovement {
    pub FORWARD: bool,
    pub BACKWARD: bool,
    pub RIGHT: bool,
    pub LEFT: bool,
    pub UP: bool,
    pub DOWN: bool,
}
const NO_MOVEMENT: DesiredMovement = DesiredMovement {
    FORWARD: false,
    BACKWARD: false,
    RIGHT: false,
    LEFT: false,
    UP: false,
    DOWN: false,
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
    pub fn new(pos: glam::f32::Vec3) -> Self {
        Self {
            pos: pos,
            vel: Vec3::new(0.0, 0.0, 0.0),
            acc: Vec3::new(0.0, 0.0, 0.0),
            facing: Vec3::new(0.0, 1.0, 0.0),

            eye_height: 1.6,
            height: 1.8,
            width: 0.1,

            move_speed: 4.3,
            jump_height: 1.3,
            acc_rate: 130.0,
            gravity: 9.8*2.5,
            desired_movement: NO_MOVEMENT,

            flying: false,
            in_air: true,
        }
    }

    pub fn get_block_looking_at(&self, world: &world::World) -> (Vec3, Vec3, BlockID) {
        world.cast_ray_to_first_non_air_block(self.pos + Vec3::Z * self.eye_height, self.facing, 4.0)
    }
    /*pub fn get_block_just_before_looking_at(&self, world: &world::World) -> Vec3 {
        let (entered_pos, _) = self.get_block_looking_at(world);
        let delta = self.facing * 0.1;
        let just_before = entered_pos - delta;
    }*/

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
        // these rot matrices can be cached because we only receive integer (or integer * look_sensitivity) inputs from the mouse events
        self.facing = Mat3::from_rotation_z(-amount_deg * DEG_TO_RAD) * self.facing;
    }
    pub fn turn_vertical(&mut self, amount_deg: f32) {
        // these rot matrices can be cached because we only receive integer (or integer * look_sensitivity) inputs from the mouse events
        let new_facing = Mat3::from_axis_angle(self.get_rightward_vector(), -amount_deg * DEG_TO_RAD) * self.facing;
        if new_facing.dot(Vec3::Z).abs() < 0.999 {
            self.facing = new_facing;
        }
    }

    pub fn update_time_independent_acceleration(&mut self){
        self.acc = Vec3::ZERO;

        if self.in_air {
            self.acc_rate = 10.0;
        } else {
            self.acc_rate = 150.0;
        }

        if self.desired_movement.FORWARD {
            self.acc += self.get_moving_forward_xy(1.0);
        }
        if self.desired_movement.BACKWARD {
            self.acc += self.get_moving_forward_xy(-1.0);
        }
        if self.desired_movement.RIGHT {
            self.acc += self.get_moving_rightward(1.0);
        }
        if self.desired_movement.LEFT {
            self.acc += self.get_moving_rightward(-1.0);
        }

        if self.flying {
            if self.desired_movement.UP {
                self.acc += self.get_moving_up(1.0);
            }
            if self.desired_movement.DOWN {
                self.acc += self.get_moving_up(-1.0);
            }
        } else {
            if self.in_air {
                self.acc.z -= self.gravity;
            } else if self.desired_movement.UP {
                self.vel.z = (self.jump_height * self.gravity * 2.0).sqrt(); //sqrt(9.8*2) = 4.427188
            }
        }
    }

    pub fn get_moving_forward(&self, fac: f32) -> Vec3 {
        fac * self.acc_rate * self.facing
    }
    pub fn get_moving_forward_xy(&self, fac: f32) -> Vec3 {
        // moves in the xy plane only
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