use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub fn from_facing(facing: crate::world::block_facing::BlockFacing) -> Self {
        match facing {
            crate::world::block_facing::BlockFacing::PosZ => Self::North,
            crate::world::block_facing::BlockFacing::NegZ => Self::South,
            crate::world::block_facing::BlockFacing::PosX => Self::East,
            crate::world::block_facing::BlockFacing::NegX => Self::West,
            crate::world::block_facing::BlockFacing::PosY => Self::Up,
            crate::world::block_facing::BlockFacing::NegY => Self::Down,
            crate::world::block_facing::BlockFacing::None => Self::None,
        }
    }

    pub fn to_facing(self) -> crate::world::block_facing::BlockFacing {
        match self {
            Self::North => crate::world::block_facing::BlockFacing::PosZ,
            Self::South => crate::world::block_facing::BlockFacing::NegZ,
            Self::East => crate::world::block_facing::BlockFacing::PosX,
            Self::West => crate::world::block_facing::BlockFacing::NegX,
            Self::Up => crate::world::block_facing::BlockFacing::PosY,
            Self::Down => crate::world::block_facing::BlockFacing::NegY,
            Self::None => crate::world::block_facing::BlockFacing::None,
        }
    }
}

impl Default for BlockOrientation {
    fn default() -> Self {
        Self::None
    }
}
