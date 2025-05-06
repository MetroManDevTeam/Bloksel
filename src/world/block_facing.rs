use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockFacing {
    Wall,
    Floor,
    Ceiling,
    Corner,
    Edge,
    Custom(u8),
    None,
}

impl BlockFacing {
    pub fn from_normal(normal: glam::Vec3) -> Self {
        if normal.x > 0.0 {
            Self::PosX
        } else if normal.x < 0.0 {
            Self::NegX
        } else if normal.y > 0.0 {
            Self::PosY
        } else if normal.y < 0.0 {
            Self::NegY
        } else if normal.z > 0.0 {
            Self::PosZ
        } else if normal.z < 0.0 {
            Self::NegZ
        } else {
            Self::None
        }
    }

    pub fn to_normal(self) -> glam::Vec3 {
        match self {
            Self::PosX => glam::Vec3::X,
            Self::NegX => -glam::Vec3::X,
            Self::PosY => glam::Vec3::Y,
            Self::NegY => -glam::Vec3::Y,
            Self::PosZ => glam::Vec3::Z,
            Self::NegZ => -glam::Vec3::Z,
            Self::None => glam::Vec3::ZERO,
        }
    }
}
