use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockFacing {
    North = 0,
    South = 1,
    East = 2,
    West = 3,
    Up = 4,
    Down = 5,
}

impl BlockFacing {
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::North),
            1 => Some(Self::South),
            2 => Some(Self::East),
            3 => Some(Self::West),
            4 => Some(Self::Up),
            5 => Some(Self::Down),
            _ => None,
        }
    }

    pub fn to_index(&self) -> usize {
        *self as usize
    }

    pub fn all() -> [Self; 6] {
        [
            Self::North,
            Self::South,
            Self::East,
            Self::West,
            Self::Up,
            Self::Down,
        ]
    }

    pub fn opposite(&self) -> Self {
        match self {
            Self::North => Self::South,
            Self::South => Self::North,
            Self::East => Self::West,
            Self::West => Self::East,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}
