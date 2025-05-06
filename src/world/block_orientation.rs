use crate::world::block_facing::BlockFacing;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockOrientation {
    North,
    South,
    East,
    West,
    Up,
    Down,
    None,
}

impl BlockOrientation {
    pub fn from_facing(facing: BlockFacing) -> Self {
        match facing {
            BlockFacing::PosZ => Self::North,
            BlockFacing::NegZ => Self::South,
            BlockFacing::PosX => Self::East,
            BlockFacing::NegX => Self::West,
            BlockFacing::PosY => Self::Up,
            BlockFacing::NegY => Self::Down,
            BlockFacing::None => Self::None,
        }
    }

    pub fn to_facing(self) -> BlockFacing {
        match self {
            Self::North => BlockFacing::PosZ,
            Self::South => BlockFacing::NegZ,
            Self::East => BlockFacing::PosX,
            Self::West => BlockFacing::NegX,
            Self::Up => BlockFacing::PosY,
            Self::Down => BlockFacing::NegY,
            Self::None => BlockFacing::None,
        }
    }
}

impl Default for BlockOrientation {
    fn default() -> Self {
        Self::None
    }
}
