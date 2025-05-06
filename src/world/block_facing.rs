use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockFacing {
    None,
    Wall,
    Floor,
    Ceiling,
    Corner,
    Edge,
    Custom(u8),
}

impl BlockFacing {
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

    pub fn opposite(&self) -> Self {
        match self {
            BlockFacing::North => BlockFacing::South,
            BlockFacing::South => BlockFacing::North,
            BlockFacing::East => BlockFacing::West,
            BlockFacing::West => BlockFacing::East,
            BlockFacing::Up => BlockFacing::Down,
            BlockFacing::Down => BlockFacing::Up,
            BlockFacing::None => BlockFacing::None,
        }
    }
}
