extern crate glam;

use crate::entity;

use glam::{
    f32::{Mat3, Vec2, Vec3},
    Mat4,
};

const deg_to_rad: f32 = 0.0174532925;

pub struct Camera {
    pub fov: f32,
    pub look_sensitivity: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            fov: std::f32::consts::FRAC_PI_2,
            look_sensitivity: 0.01,
        }
    }

    pub fn set_fov(&mut self, fov_deg: f32) {
        self.fov = fov_deg * deg_to_rad;
    }

    pub fn get_proj_mat(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov, aspect_ratio, 0.001, 100.0)
    }
    pub fn get_view_mat(&self, entity: &Option<entity::Entity>) -> Mat4 {
        let e = entity.as_ref().unwrap();
        Mat4::look_to_rh(e.pos+Vec3::Z+Vec3::Z, e.facing, -Vec3::Z) // negative because glsl is retarded
    }
}
