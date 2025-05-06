pub mod block;
pub mod block_error;
pub mod block_facing;
pub mod block_flags;
pub mod block_id;
pub mod block_material;
pub mod block_orientation;
pub mod block_tech;
pub mod block_visual;
pub mod blocks_data;
pub mod chunk;
pub mod chunk_coord;
pub mod chunk_mesh;
pub mod core;
pub mod generator;
pub mod pool;
pub mod spatial;
pub mod storage;

pub use block::{Block, SubBlock};
pub use block_error::BlockError;
pub use block_facing::BlockFacing;
pub use block_flags::BlockFlags;
pub use block_id::BlockId;
pub use block_material::{BlockMaterial, MaterialModifiers};
pub use block_orientation::BlockOrientation;
pub use block_tech::*;
pub use block_visual::ConnectedDirections;
pub use blocks_data::BlockRegistry;
pub use chunk::*;
pub use chunk::{Chunk, ChunkMesh};
pub use chunk_coord::ChunkCoord;
pub use core::World;
pub use generator::*;
pub use pool::{ChunkPool, PoolStats};
pub use spatial::*;
pub use storage::*;
pub use storage::core::MemoryStorage;
pub use storage::file::FileChunkStorage;

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
