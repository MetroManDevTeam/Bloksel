use crate::world::BlockOrientation;
use crate::world::block_facing::BlockFacing;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct ConnectedDirections: u8 {
        const NORTH = 0b00000001;
        const SOUTH = 0b00000010;
        const EAST = 0b00000100;
        const WEST = 0b00001000;
        const UP = 0b00010000;
        const DOWN = 0b00100000;
    }
}

impl ConnectedDirections {
    pub fn new() -> Self {
        Self::empty()
    }

    pub fn set(&mut self, facing: BlockFacing, connected: bool) {
        match facing {
            BlockFacing::PosZ => self.set(ConnectedDirections::NORTH, connected),
            BlockFacing::NegZ => self.set(ConnectedDirections::SOUTH, connected),
            BlockFacing::PosX => self.set(ConnectedDirections::EAST, connected),
            BlockFacing::NegX => self.set(ConnectedDirections::WEST, connected),
            BlockFacing::PosY => self.set(ConnectedDirections::UP, connected),
            BlockFacing::NegY => self.set(ConnectedDirections::DOWN, connected),
            BlockFacing::None => (),
        }
    }

    pub fn get(&self, facing: BlockFacing) -> bool {
        match facing {
            BlockFacing::PosZ => self.contains(ConnectedDirections::NORTH),
            BlockFacing::NegZ => self.contains(ConnectedDirections::SOUTH),
            BlockFacing::PosX => self.contains(ConnectedDirections::EAST),
            BlockFacing::NegX => self.contains(ConnectedDirections::WEST),
            BlockFacing::PosY => self.contains(ConnectedDirections::UP),
            BlockFacing::NegY => self.contains(ConnectedDirections::DOWN),
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
