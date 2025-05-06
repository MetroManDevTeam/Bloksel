use glam::IVec3;
<<<<<<< Updated upstream

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
=======
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
>>>>>>> Stashed changes
pub struct ChunkCoord(pub IVec3);

impl ChunkCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }
<<<<<<< Updated upstream
=======

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
>>>>>>> Stashed changes
}
