use crate::world::BlockOrientation;
use crate::world::block_facing::BlockFacing;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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

    pub fn set_direction(&mut self, facing: BlockFacing, connected: bool) {
        match facing {
            BlockFacing::PosZ => {
                if connected {
                    *self |= ConnectedDirections::NORTH
                } else {
                    *self &= !ConnectedDirections::NORTH
                }
            }
            BlockFacing::NegZ => {
                if connected {
                    *self |= ConnectedDirections::SOUTH
                } else {
                    *self &= !ConnectedDirections::SOUTH
                }
            }
            BlockFacing::PosX => {
                if connected {
                    *self |= ConnectedDirections::EAST
                } else {
                    *self &= !ConnectedDirections::EAST
                }
            }
            BlockFacing::NegX => {
                if connected {
                    *self |= ConnectedDirections::WEST
                } else {
                    *self &= !ConnectedDirections::WEST
                }
            }
            BlockFacing::PosY => {
                if connected {
                    *self |= ConnectedDirections::UP
                } else {
                    *self &= !ConnectedDirections::UP
                }
            }
            BlockFacing::NegY => {
                if connected {
                    *self |= ConnectedDirections::DOWN
                } else {
                    *self &= !ConnectedDirections::DOWN
                }
            }
            _ => {}
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
            _ => false,
        }
    }

    pub fn from_u8(value: u8) -> Self {
        Self::from_bits_truncate(value)
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
