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
        PoolStats, QuadTree, SerializedChunk, SpatialPartition, TerrainGenerator,
    },
};
use std::sync::Arc;

pub struct World {
    pub generator: TerrainGenerator,
    pub storage: SpatialPartition,
    pub pool: ChunkPool,
    pub config: WorldGenConfig,
}

impl World {
    pub fn new(config: WorldGenConfig, block_registry: Arc<BlockRegistry>) -> Self {
        Self {
            generator: TerrainGenerator::new(config.clone(), block_registry),
            storage: SpatialPartition::new(),
            pool: ChunkPool::new(),
            config,
        }
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<Block> {
        let chunk_coord = ChunkCoord::from_world_pos(x, y, z);
        let (local_x, local_y, local_z) = chunk_coord.get_local_coords(x, y, z);

        if let Some(chunk) = self.storage.get_chunk(&chunk_coord) {
            chunk
                .get_block(local_x.into(), local_y.into(), local_z.into())
                .cloned()
        } else {
            None
        }
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: Option<Block>) {
        let chunk_coord = ChunkCoord::from_world_pos(x, y, z);
        let (local_x, local_y, local_z) = chunk_coord.get_local_coords(x, y, z);

        if let Some(chunk) = self.storage.get_chunk_mut(&chunk_coord) {
            chunk.set_block(local_x.into(), local_y.into(), local_z.into(), block);
        }
    }

    pub fn get_chunk(&self, coord: &ChunkCoord) -> Option<Arc<Chunk>> {
        self.storage.get_chunk(coord)
    }

    pub fn get_chunk_mut(&mut self, coord: &ChunkCoord) -> Option<&mut Chunk> {
        self.storage.get_chunk_mut(coord)
    }

    pub fn generate_chunk(&mut self, coord: ChunkCoord) {
        if !self.storage.has_chunk(&coord) {
            let chunk = self.generator.generate_chunk(coord);
            self.storage.set_chunk(coord, chunk);
        }
    }
}
