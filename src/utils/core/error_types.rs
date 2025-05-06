use thiserror::Error;
use std::io;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Asset loading error: {0}")]
    AssetError(String),

    #[error("Shader compilation error: {0}")]
    ShaderError(String),

    #[error("Render error: {0}")]
    RenderError(String),

    #[error("World generation error: {0}")]
    WorldGenError(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;
