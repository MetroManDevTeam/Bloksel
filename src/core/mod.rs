//! Core voxel systems
pub mod block;
pub mod chunk;
pub mod world;

// Selective re-exports
pub use block::{Block, BlockProperties, BlockError};
pub use chunk::{Chunk, CHUNK_SIZE, CHUNK_HEIGHT};
pub use world::{World, WorldError, ChunkPos};

/// Thread-safe chunk handle
pub type ChunkHandle = Arc<RwLock<Chunk>>;
