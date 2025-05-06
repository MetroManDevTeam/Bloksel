pub mod block;
pub mod block_coord;
pub mod block_facing;
pub mod block_flags;
pub mod block_id;
pub mod block_material;
pub mod block_tech;
pub mod block_visual;
pub mod blocks_data;
pub mod chunk;
pub mod chunk_coord;
pub mod generator;
pub mod spatial;
pub mod storage;

// Re-export commonly used types
pub use block::*;
pub use block_coord::*;
pub use block_facing::*;
pub use block_flags::*;
pub use block_id::BlockRegistry;
pub use block_material::*;
pub use block_tech::*;
pub use block_visual::*;
pub use blocks_data::*;
pub use chunk::*;
pub use chunk_coord::*;
pub use generator::*;
pub use spatial::*;
pub use storage::*;

// Re-export specific types that need to be public
pub use block_tech::BlockPhysics;
pub use chunk::Chunk;
pub use chunk_coord::ChunkCoord;
pub use generator::TerrainGenerator;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldType {
    Normal,
    Flat,
    Superflat,
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMeta {
    pub name: String,
    pub world_type: WorldType,
    pub seed: i64,
    pub difficulty: Difficulty,
    pub spawn_point: [f32; 3],
    pub last_played: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}
