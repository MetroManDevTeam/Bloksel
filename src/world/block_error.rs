use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockError {
    InvalidIdFormat,
    DuplicateName(String),
    InvalidBlockId,
    InvalidBlockData,
    BlockNotFound,
}

impl std::fmt::Display for BlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdFormat => write!(f, "Invalid block ID format"),
            Self::DuplicateName(name) => write!(f, "Duplicate block name: {}", name),
            Self::InvalidBlockId => write!(f, "Invalid block ID"),
            Self::InvalidBlockData => write!(f, "Invalid block data"),
            Self::BlockNotFound => write!(f, "Block not found"),
        }
    }
}

impl std::error::Error for BlockError {}
