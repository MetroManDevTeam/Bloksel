use crate::world::BlockFacing;
use crate::world::BlockOrientation;
use crate::world::block::Block;
use crate::world::chunk::Chunk;
use crate::world::chunk_coord::ChunkCoord;
use crate::world::generator::WorldGenerator;
use crate::world::storage::ChunkStorage;
use crate::{
    config::WorldGenConfig,
    render::pipeline::ChunkRenderer,
    world::{
        BlockId, BlockMaterial, BlockRegistry, ChunkManager, ChunkPool, MaterialModifiers,
        PoolStats, QuadTree, SerializedChunk, SpatialPartition,
    },
};
use std::sync::Arc;

pub struct World {
    pub chunk_manager: ChunkManager,
    pub block_registry: BlockRegistry,
    pub spatial_partition: SpatialPartition,
    generator: Arc<dyn WorldGenerator>,
    storage: Arc<dyn ChunkStorage>,
    pool: Arc<ChunkPool>,
}

impl World {
    pub fn new(
        config: WorldGenConfig,
        renderer: Arc<Renderer>,
        block_registry: BlockRegistry,
    ) -> Self {
        let block_registry = Arc::new(block_registry);
        Self {
            chunk_manager: ChunkManager::new(config.clone(), renderer, block_registry.clone()),
            spatial_partition: SpatialPartition::new(&config.engine),
            block_registry,
            config,
        }
    }

    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        if let Some(chunk) = self.storage.get_chunk(coord) {
            return Some(chunk);
        }

        let chunk = self.generator.generate_chunk(coord);
        let chunk = Arc::new(chunk);
        self.storage.set_chunk(coord, chunk.clone());
        Some(chunk)
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        let chunk_x = x.div_euclid(16);
        let chunk_y = y.div_euclid(16);
        let chunk_z = z.div_euclid(16);
        let coord = ChunkCoord::new(chunk_x, chunk_y, chunk_z);

        if let Some(chunk) = self.get_chunk(coord) {
            let local_x = x.rem_euclid(16) as u8;
            let local_y = y.rem_euclid(16) as u8;
            let local_z = z.rem_euclid(16) as u8;
            chunk.get_block(local_x, local_y, local_z)
        } else {
            self.generator.get_block(x, y, z)
        }
    }
}
