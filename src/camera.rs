extern crate glam;

use crate::entity;

use glam::{
    Vec3,
    Mat4,
};

const DEG_TO_RAD: f32 = 0.0174532925;

pub struct Camera {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub look_sensitivity: f32,
    pub height: f32,
    pub proj_mat: Mat4,
}

impl Camera {
    pub fn new(aspect_ratio: f32) -> Self {
        let fov = std::f32::consts::FRAC_PI_2;
        Self {
            fov,
            aspect_ratio,
            look_sensitivity: if cfg!(target_os = "macos") {0.07} else {0.02},
            height: 1.6,
            proj_mat: Self::get_proj_mat(fov, aspect_ratio)
        }
    }

    pub fn set_fov(&mut self, fov_deg: f32) {
        self.fov = fov_deg * DEG_TO_RAD;
        self.proj_mat = Self::get_proj_mat(self.fov, self.aspect_ratio);
    }
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
        self.proj_mat = Self::get_proj_mat(self.fov, self.aspect_ratio);
    }

    fn get_proj_mat(fov: f32, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_infinite_rh(fov, aspect_ratio, 0.001)
    }
    fn get_view_mat(&self, entity: &entity::Entity) -> Mat4 {
        Mat4::look_to_rh(entity.pos + Vec3::Z * self.height, entity.facing, Vec3::Z)
    }
    pub fn get_projview(&self, entity: &entity::Entity) -> Mat4 {
        self.proj_mat * self.get_view_mat(entity)
    }
}
