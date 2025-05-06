use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord(pub IVec3);

impl ChunkCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    pub fn x(&self) -> i32 {
        self.0.x
    }

    pub fn y(&self) -> i32 {
        self.0.y
    }

    pub fn z(&self) -> i32 {
        self.0.z
    }

    pub fn distance_squared(&self, other: &ChunkCoord) -> i32 {
        let dx = self.x() - other.x();
        let dy = self.y() - other.y();
        let dz = self.z() - other.z();
        dx * dx + dy * dy + dz * dz
    }

    pub fn from_world_pos(world_pos: Vec3, chunk_size: u32) -> Self {
        let x = (world_pos.x / chunk_size as f32).floor() as i32;
        let y = (world_pos.y / chunk_size as f32).floor() as i32;
        let z = (world_pos.z / chunk_size as f32).floor() as i32;
        Self::new(x, y, z)
    }
}
