use crate::{
    config::WorldGenConfig,
    render::pipeline::ChunkRenderer,
    world::{
        BlockId, BlockMaterial, BlockRegistry, ChunkManager, ChunkPool, MaterialModifiers,
        PoolStats, QuadTree, SerializedChunk, SpatialPartition,
    },
};

pub struct World {
    pub chunk_manager: ChunkManager,
    pub block_registry: BlockRegistry,
    pub spatial_partition: SpatialPartition,
}

impl World {
    pub fn new(
        config: &WorldGenConfig,
        renderer: ChunkRenderer,
        block_registry: BlockRegistry,
    ) -> Self {
        Self {
            chunk_manager: ChunkManager::new(config.clone(), renderer, block_registry.clone()),
            block_registry,
            spatial_partition: SpatialPartition::new(config),
        }
    }
}
