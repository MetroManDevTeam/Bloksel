use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockOrientation {
    None,
    Wall,
    Floor,
    Ceiling,
    Corner,
    Edge,
    Custom(u8),
}

impl BlockOrientation {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Wall),
            2 => Some(Self::Floor),
            3 => Some(Self::Ceiling),
            4 => Some(Self::Corner),
            5 => Some(Self::Edge),
            n => Some(Self::Custom(n)),
        }
    }
}

impl Default for BlockOrientation {
    fn default() -> Self {
        Self::None
    }
}
