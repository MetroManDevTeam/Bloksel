use glam::{Vec2, Vec3};
use std::sync::Arc;

pub struct MeshData {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub tex_coords: Vec<Vec2>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            normals: Vec::new(),
            tex_coords: Vec::new(),
            indices: Vec::new(),
        }
    }
}

pub struct Mesh {
    pub data: Arc<MeshData>,
    pub material_index: u32,
}
