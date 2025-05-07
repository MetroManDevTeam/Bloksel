use glam::{IVec3, Vec3};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord(pub IVec3);

impl Serialize for ChunkCoord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (self.0.x, self.0.y, self.0.z).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChunkCoord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (x, y, z) = <(i32, i32, i32)>::deserialize(deserializer)?;
        Ok(ChunkCoord(IVec3::new(x, y, z)))
    }
}

impl PartialOrd for ChunkCoord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChunkCoord {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.x.cmp(&other.0.x) {
            Ordering::Equal => match self.0.y.cmp(&other.0.y) {
                Ordering::Equal => self.0.z.cmp(&other.0.z),
                ord => ord,
            },
            ord => ord,
        }
    }
}

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

    pub fn to_world_center(&self, chunk_size: i32) -> Vec3 {
        let pos = self.to_world_pos(chunk_size);
        pos + Vec3::splat(chunk_size as f32 / 2.0)
    }

    pub fn to_path(&self) -> PathBuf {
        PathBuf::from(format!("chunk_{}_{}_{}.bin", self.0.x, self.0.y, self.0.z))
    }

    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let file_name = path
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))?
            .to_str()
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid filename")
            })?;

        if !file_name.starts_with("chunk_") || !file_name.ends_with(".bin") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid chunk file format",
            ));
        }

        let coords = file_name[6..file_name.len() - 4]
            .split('_')
            .map(|s| s.parse::<i32>())
            .collect::<Result<Vec<i32>, _>>()
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid coordinates")
            })?;

        if coords.len() != 3 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid coordinate count",
            ));
        }

        Ok(Self::new(coords[0], coords[1], coords[2]))
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
