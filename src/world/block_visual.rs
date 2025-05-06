use crate::world::block_facing::BlockFacing;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectedDirections {
    pub north: bool,
    pub south: bool,
    pub east: bool,
    pub west: bool,
    pub up: bool,
    pub down: bool,
}

impl ConnectedDirections {
    pub fn new() -> Self {
        Self {
            north: false,
            south: false,
            east: false,
            west: false,
            up: false,
            down: false,
        }
    }

    pub fn set(&mut self, facing: BlockFacing, connected: bool) {
        match facing {
            BlockFacing::North => self.north = connected,
            BlockFacing::South => self.south = connected,
            BlockFacing::East => self.east = connected,
            BlockFacing::West => self.west = connected,
            BlockFacing::Up => self.up = connected,
            BlockFacing::Down => self.down = connected,
            BlockFacing::None => (),
        }
    }

    pub fn get(&self, facing: BlockFacing) -> bool {
        match facing {
            BlockFacing::North => self.north,
            BlockFacing::South => self.south,
            BlockFacing::East => self.east,
            BlockFacing::West => self.west,
            BlockFacing::Up => self.up,
            BlockFacing::Down => self.down,
            BlockFacing::None => false,
        }
    }
}

impl BlockFacing {
    pub fn opposite(&self) -> Self {
        match self {
            BlockFacing::North => BlockFacing::South,
            BlockFacing::South => BlockFacing::North,
            BlockFacing::East => BlockFacing::West,
            BlockFacing::West => BlockFacing::East,
            BlockFacing::Up => BlockFacing::Down,
            BlockFacing::Down => BlockFacing::Up,
            _ => BlockFacing::None,
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
}

impl BlockOrientation {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::North),
            2 => Some(Self::South),
            3 => Some(Self::East),
            4 => Some(Self::West),
            5 => Some(Self::Up),
            6 => Some(Self::Down),
            _ => None,
        }
    }
}

impl Default for BlockFacing {
    fn default() -> Self {
        BlockFacing::None
    }
}
