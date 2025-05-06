
#[derive(Debug, Error)]
pub enum BlockError {
    #[error("Duplicate block ID: {0:?}")]
    DuplicateId(BlockId),
    
    #[error("Duplicate block name: {0}")]
    DuplicateName(String),
    
    #[error("Invalid variant data")]
    InvalidVariant,
    
    #[error("Serialization failed")]
    SerializationFailed,
    
    #[error("Deserialization failed")]
    DeserializationFailed,
    
    #[error("Texture not found: {0}")]
    TextureNotFound(String),
}
