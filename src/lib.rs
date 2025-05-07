pub mod config;
pub mod engine;
pub mod player;
pub mod render;
pub mod utils;
pub mod world;

// Re-export commonly used types
pub use config::chunksys::ChunkSysConfig;
pub use config::core::EngineConfig;
pub use config::gameplay::GameplayConfig;
pub use config::worldgen::WorldGenConfig;
pub use engine::VoxelEngine;
pub use player::Player;
pub use player::PlayerState;
pub use player::input::InputState;
pub use render::pipeline::RenderPipeline;
pub use render::shaders::ShaderProgram;
pub use utils::error::BlockError;
pub use utils::math::raycast::Ray;
pub use utils::math::{Plane, ViewFrustum};
pub use world::block_id::BlockRegistry;
pub use world::chunk::{Chunk, SerializedChunk};
pub use world::chunk_coord::ChunkCoord;
pub use world::generator::terrain::TerrainGenerator;
pub use world::spatial::SpatialPartition;
