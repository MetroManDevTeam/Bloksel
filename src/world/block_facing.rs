use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockFacing {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
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
            Self::Wall
            | Self::Floor
            | Self::Ceiling
            | Self::Corner
            | Self::Edge
            | Self::Custom(_)
            | Self::None => glam::Vec3::ZERO,
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Wall,
            1 => Self::Floor,
            2 => Self::Ceiling,
            3 => Self::Corner,
            4 => Self::Edge,
            n => Self::Custom(n),
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            Self::Wall => 0,
            Self::Floor => 1,
            Self::Ceiling => 2,
            Self::Corner => 3,
            Self::Edge => 4,
            Self::Custom(n) => n,
            _ => 0,
        }
    }

    pub fn opposite(&self) -> Self {
        match self {
            Self::PosX => Self::NegX,
            Self::NegX => Self::PosX,
            Self::PosY => Self::NegY,
            Self::NegY => Self::PosY,
            Self::PosZ => Self::NegZ,
            Self::NegZ => Self::PosZ,
            _ => Self::None,
        }
    }
}
