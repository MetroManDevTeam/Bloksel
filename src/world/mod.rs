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
pub mod generator;
pub mod pool;
pub mod spatial;
pub mod storage;

// Re-export commonly used types
pub use block::Block;
pub use block_error::BlockError;
pub use block_facing::BlockFacing;
pub use block_flags::BlockFlags;
pub use block_id::{BlockCategory, BlockData, BlockDefinition, BlockId, BlockRegistry};
pub use block_material::BlockMaterial;
pub use block_orientation::BlockOrientation;
pub use block_tech::BlockPhysics;
pub use block_visual::ConnectedDirections;
pub use blocks_data::BLOCKS;
pub use chunk::{Chunk, SerializedChunk};
pub use chunk_coord::ChunkCoord;
pub use generator::TerrainGenerator;
pub use pool::ChunkPool;
pub use spatial::SpatialIndex;
pub use storage::ChunkStorage;

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
