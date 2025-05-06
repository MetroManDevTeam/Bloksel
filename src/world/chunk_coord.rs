use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord(pub IVec3);

impl ChunkCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    pub fn from_world_pos(pos: Vec3, chunk_size: i32) -> Self {
        let x = (pos.x / chunk_size as f32).floor() as i32;
        let y = (pos.y / chunk_size as f32).floor() as i32;
        let z = (pos.z / chunk_size as f32).floor() as i32;
        Self::new(x, y, z)
    }

    pub fn from_world(position: Vec3) -> Self {
        Self::from_world_pos(position, 16) // Using standard chunk size of 16
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
            self.0.x as f32 * chunk_size as f32,
            self.0.y as f32 * chunk_size as f32,
            self.0.z as f32 * chunk_size as f32,
        )
    }

    pub fn to_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "chunks/{}/{}/{}.chunk",
            self.0.x, self.0.y, self.0.z
        ))
    }

    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let file_name = path
            .file_name()
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid chunk file name")
            })?
            .to_str()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid UTF-8 in chunk file name",
                )
            })?;

        if !file_name.ends_with(".chunk") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Not a chunk file",
            ));
        }

        let coords: Vec<&str> = file_name.trim_end_matches(".chunk").split('_').collect();
        if coords.len() != 3 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid chunk coordinates",
            ));
        }

        let x = coords[0].parse::<i32>().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid x coordinate")
        })?;
        let y = coords[1].parse::<i32>().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid y coordinate")
        })?;
        let z = coords[2].parse::<i32>().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid z coordinate")
        })?;

        Ok(Self::new(x, y, z))
    }

    pub fn get_neighbors(&self) -> Vec<Self> {
        let mut neighbors = Vec::with_capacity(26);
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if x == 0 && y == 0 && z == 0 {
                        continue;
                    }
                    neighbors.push(Self(self.0 + IVec3::new(x, y, z)));
                }
            }
        }
        neighbors
    }

    pub fn manhattan_distance(&self, other: &Self) -> i32 {
        (self.0.x - other.0.x).abs() + (self.0.y - other.0.y).abs() + (self.0.z - other.0.z).abs()
    }

    pub fn distance(&self, other: &Self) -> f32 {
        let dx = (self.0.x - other.0.x) as f32;
        let dy = (self.0.y - other.0.y) as f32;
        let dz = (self.0.z - other.0.z) as f32;
        (dx * dx + dy * dy + dz * dz).sqrt()
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
