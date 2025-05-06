use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockFacing {
    North,
    South,
    East,
    West,
    Up,
    Down,
    None,
}

impl BlockFacing {
    pub fn from_normal(normal: glam::Vec3) -> Self {
        if normal.y > 0.0 {
            Self::Up
        } else if normal.y < 0.0 {
            Self::Down
        } else if normal.x > 0.0 {
            Self::East
        } else if normal.x < 0.0 {
            Self::West
        } else if normal.z > 0.0 {
            Self::North
        } else if normal.z < 0.0 {
            Self::South
        } else {
            Self::None
        }
    }

    pub fn to_normal(self) -> glam::Vec3 {
        match self {
            Self::North => glam::Vec3::new(0.0, 0.0, 1.0),
            Self::South => glam::Vec3::new(0.0, 0.0, -1.0),
            Self::East => glam::Vec3::new(1.0, 0.0, 0.0),
            Self::West => glam::Vec3::new(-1.0, 0.0, 0.0),
            Self::Up => glam::Vec3::new(0.0, 1.0, 0.0),
            Self::Down => glam::Vec3::new(0.0, -1.0, 0.0),
            Self::None => glam::Vec3::ZERO,
        }
    }

    pub fn opposite(&self) -> Self {
        match self {
            Self::North => Self::South,
            Self::South => Self::North,
            Self::East => Self::West,
            Self::West => Self::East,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::None => Self::None,
        }
    }
}
