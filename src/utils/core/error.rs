use thiserror::Error;
use crate::world::block_id::BlockId;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Duplicate block ID: {0:?}")]
    DuplicateId(BlockId),

    #[error("Duplicate block name: {0}")]
    DuplicateName(String),

    #[error("Invalid variant data")]
    InvalidVariant,

    #[error("Serialization failed")]
    SerializationError,

    #[error("Deserialization failed")]
    DeserializationError,

    #[error("Texture not found: {0}")]
    TextureNotFound(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
