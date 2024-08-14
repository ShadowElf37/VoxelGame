extern crate glam;

use glam::{
    Mat3, Vec2, Vec3,
};

const DEG_TO_RAD: f32 = 0.0174532925;

#[derive(Clone)]
pub struct Entity {
    pub pos: Vec3,
    pub vel: Vec3,
    pub facing: Vec3,

    pub move_speed: f32,
    pub desired_movement: DesiredMovement,
    
    pub flying: bool,
    pub in_air: bool,
}

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

impl Entity {
    pub fn new(pos: glam::f32::Vec3) -> Self {
        Self {
            pos: pos,
            vel: Vec3::new(0.0, 0.0, 0.0),
            facing: Vec3::new(0.0, 1.0, 0.0),

            move_speed: 3.0,
            desired_movement: NO_MOVEMENT,

            flying: true,
            in_air: true,
        }
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

    pub fn get_desired_velocity(&mut self) -> Vec3{
        let mut desired_vel = Vec3::ZERO;

        if self.desired_movement.FORWARD {
            desired_vel += self.get_moving_forward_xy(1.0);
        }
        if self.desired_movement.BACKWARD {
            desired_vel += self.get_moving_forward_xy(-1.0);
        }
        if self.desired_movement.RIGHT {
            desired_vel += self.get_moving_rightward(1.0);
        }
        if self.desired_movement.LEFT {
            desired_vel += self.get_moving_rightward(-1.0);
        }
        if self.flying {
            if self.desired_movement.UP {
                desired_vel += self.get_moving_up(1.0);
            }
            if self.desired_movement.DOWN {
                desired_vel += self.get_moving_up(-1.0);
            }
        } else if !self.in_air {
            desired_vel += self.get_moving_up(2.0);
        }

        desired_vel
    }

    pub fn get_moving_forward(&self, fac: f32) -> Vec3 {
        fac * self.move_speed * self.facing
    }
    pub fn get_moving_forward_xy(&self, fac: f32) -> Vec3 {
        // moves in the xy plane only
        fac * self.move_speed * self.facing.with_z(0f32).normalize()
    }
    pub fn get_moving_rightward(&self, fac: f32) -> Vec3 {
        fac * self.move_speed * self.get_rightward_vector()
    }
    pub fn get_moving_up(&self, fac: f32) -> Vec3 {
        fac * self.move_speed * Vec3::Z

    }
    pub fn clear_moving(&mut self) {
        self.desired_movement = NO_MOVEMENT;
    }

    // DEPRECATED: manual movement
    /*
    pub fn move_forward(&mut self, amount: f32) {
        self.pos += amount * self.facing;
    }
    pub fn move_forward_xy(&mut self, amount: f32) {
        // moves in the xy plane only
        self.pos += amount * self.facing.with_z(0f32).normalize();
    }
    pub fn move_rightward(&mut self, amount: f32) {
        self.pos += amount * self.get_rightward_vector();
    }
    pub fn move_up(&mut self, amount: f32) {
        self.pos += amount * Vec3::Z;
    }
    */
}