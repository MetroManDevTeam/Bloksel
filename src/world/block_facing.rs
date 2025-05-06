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
        if normal.y > 0.0 {
            Self::Ceiling
        } else if normal.y < 0.0 {
            Self::Floor
        } else if normal.x != 0.0 || normal.z != 0.0 {
            Self::Wall
        } else {
            Self::None
        }
    }

    pub fn to_normal(self) -> glam::Vec3 {
        match self {
            Self::Wall => glam::Vec3::new(1.0, 0.0, 0.0),
            Self::Floor => glam::Vec3::new(0.0, -1.0, 0.0),
            Self::Ceiling => glam::Vec3::new(0.0, 1.0, 0.0),
            Self::Corner => glam::Vec3::new(1.0, 1.0, 1.0).normalize(),
            Self::Edge => glam::Vec3::new(1.0, 0.0, 1.0).normalize(),
            Self::Custom(_) => glam::Vec3::ZERO,
            Self::None => glam::Vec3::ZERO,
        }
    }
}
