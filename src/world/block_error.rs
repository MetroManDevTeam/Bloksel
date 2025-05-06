use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockError {
    InvalidIdFormat,
    DuplicateId(u32),
    DuplicateName(String),
    SerializationFailed,
    DeserializationFailed,
}

impl std::fmt::Display for BlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdFormat => write!(f, "Invalid block ID format"),
            Self::DuplicateId(id) => write!(f, "Duplicate block ID: {}", id),
            Self::DuplicateName(name) => write!(f, "Duplicate block name: {}", name),
            Self::SerializationFailed => write!(f, "Failed to serialize block data"),
            Self::DeserializationFailed => write!(f, "Failed to deserialize block data"),
        }
    }
}

impl std::error::Error for BlockError {}
