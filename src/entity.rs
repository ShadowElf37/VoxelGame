extern crate glam;

use glam::{
    f32::{Mat3, Vec2, Vec3},
    Mat4,
};

const deg_to_rad: f32 = 0.0174532925;

#[derive(Clone)]
pub struct Entity {
    pub pos: Vec3,
    pub vel: Vec3,
    pub facing: Vec3,

    pub move_speed: f32,
    pub desired_movement: DesiredMovement,
    
    pub flying: bool,
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
static NoMovement: DesiredMovement = DesiredMovement{FORWARD: false,
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
            facing: Vec3::new(0.0, -1.0, 0.0).normalize(),

            move_speed: 3.0,
            desired_movement: NoMovement.clone(),

            flying: true,
        }
    }

    pub fn facing_in_degrees(&self) -> Vec2 {
        Vec2::new(
            self.facing.z.asin() / deg_to_rad, // vertical
            self.facing.x.acos() / deg_to_rad, // horizontal
        )
    }
    pub fn get_rightward_vector(&self) -> Vec3 {
        Vec3::Z.cross(self.facing).normalize()
    }

    pub fn turn_horizontal(&mut self, amount_deg: f32) {
        self.facing = Mat3::from_rotation_z(amount_deg * deg_to_rad) * self.facing;
    }
    pub fn turn_vertical(&mut self, amount_deg: f32) {
        let new_facing = Mat3::from_axis_angle(self.get_rightward_vector(), -amount_deg * deg_to_rad) * self.facing;
        if new_facing.dot(Vec3::Z).abs() < 0.999 {
            self.facing = new_facing;
        }
    }

    pub fn get_desired_velocity(&self) -> Vec3{
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
        self.desired_movement = NoMovement.clone();
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