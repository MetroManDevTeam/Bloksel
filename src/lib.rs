pub mod assets;
pub mod config;
pub mod player;
pub mod render;
pub mod ui;
pub mod utils;
pub mod world;

// Re-exports for easier access
pub use config::{
    chunksys::ChunkSysConfig, core::EngineConfig, gameplay::GameplayConfig,
    worldgen::WorldGenConfig,
};
pub use player::{
    input::PlayerInput,
    physics::{Player, PlayerState},
};
pub use render::{pipeline::RenderPipeline, shaders::ShaderProgram};
pub use utils::{
    error::BlockError,
    math::{Plane, Ray, ViewFrustum},
};
pub use world::{
    block_id::BlockRegistry,
    chunk::{Chunk, ChunkCoord, SerializedChunk},
    generator::terrain::TerrainGenerator,
    spatial::SpatialPartition,
};
