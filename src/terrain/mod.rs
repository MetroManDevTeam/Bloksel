//! World generation
pub mod generator;
pub mod loader;

pub use generator::{TerrainGenerator, SimpleTerrainGenerator};
pub use loader::ChunkLoader;

/// World seed type
pub type WorldSeed = u64;
