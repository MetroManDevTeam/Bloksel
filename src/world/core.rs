use crate::{
    world::{
        BlockId,
        BlockMaterial,
        MaterialModifiers,
        BlockRegistry,
        ChunkManager,
        SerializedChunk,
        ChunkPool,
        PoolStats,
        SpatialPartition,
        QuadTree,
    },
    config::WorldConfig,
    render::pipeline::ChunkRenderer,
};

pub struct World {
    pub chunk_manager: ChunkManager,
    pub block_registry: BlockRegistry,
    pub spatial_partition: SpatialPartition,
}

impl World {
    pub fn new(config: &WorldConfig, renderer: ChunkRenderer, block_registry: BlockRegistry) -> Self {
        Self {
            chunk_manager: ChunkManager::new(config.clone(), renderer, block_registry.clone()),
            block_registry,
            spatial_partition: SpatialPartition::new(config),
        }
    }
}
