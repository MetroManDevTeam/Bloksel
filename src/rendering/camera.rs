use glam::{Mat4, Vec3};

pub struct Camera {
    pub position: Vec3,
    pub view_matrix: Mat4,
}

impl Camera {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            view_matrix: Mat4::IDENTITY,
        }
    }
}
