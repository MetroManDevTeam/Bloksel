use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord(pub IVec3);

impl ChunkCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    pub fn from_world_pos(world_pos: Vec3, chunk_size: i32) -> Self {
        let x = (world_pos.x / chunk_size as f32).floor() as i32;
        let y = (world_pos.y / chunk_size as f32).floor() as i32;
        let z = (world_pos.z / chunk_size as f32).floor() as i32;
        Self::new(x, y, z)
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

    pub fn to_world_pos(&self, chunk_size: i32) -> Vec3 {
        Vec3::new(
            self.x() as f32 * chunk_size as f32,
            self.y() as f32 * chunk_size as f32,
            self.z() as f32 * chunk_size as f32,
        )
    }

    pub fn distance_squared(&self, other: &ChunkCoord) -> i32 {
        let dx = self.x() - other.x();
        let dy = self.y() - other.y();
        let dz = self.z() - other.z();
        dx * dx + dy * dy + dz * dz
    }

    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let file_name = path.file_name().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid chunk file name")
        })?;

        let file_name = file_name.to_str().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid chunk file name encoding",
            )
        })?;

        let parts: Vec<&str> = file_name.split('_').collect();
        if parts.len() != 4 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid chunk file name format",
            ));
        }

        let x = parts[1].parse::<i32>().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid x coordinate in chunk file name",
            )
        })?;

        let y = parts[2].parse::<i32>().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid y coordinate in chunk file name",
            )
        })?;

        let z = parts[3].split('.').next().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid chunk file name format",
            )
        })?;

        let z = z.parse::<i32>().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid z coordinate in chunk file name",
            )
        })?;

        Ok(Self::new(x, y, z))
    }
}

impl From<IVec3> for ChunkCoord {
    fn from(vec: IVec3) -> Self {
        Self(vec)
    }
}

impl From<ChunkCoord> for IVec3 {
    fn from(coord: ChunkCoord) -> Self {
        coord.0
    }
}
