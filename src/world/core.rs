use crate::world::BlockFacing;
use crate::world::BlockOrientation;
use crate::world::block::Block;
use crate::world::block_id::BlockId;
use crate::world::block_material::BlockMaterial;
use crate::world::block_material::MaterialModifiers;
use crate::world::blocks_data::BlockRegistry;
use crate::world::chunk::Chunk;
use crate::world::chunk::ChunkManager;
use crate::world::chunk::SerializedChunk;
use crate::world::chunk_coord::ChunkCoord;
use crate::world::generator::terrain::{TerrainGenerator, WorldGenConfig};
use crate::world::pool::ChunkPool;
use crate::world::pool::PoolStats;
use crate::world::spatial::QuadTree;
use crate::world::spatial::SpatialPartition;
use crate::world::storage::core::{ChunkStorage, MemoryStorage};
use crate::world::storage::file::FileChunkStorage;
use crate::{config::core::EngineConfig, render::pipeline::ChunkRenderer};
use glam::Vec3;
use std::path::Path;
use std::sync::Arc;

pub struct World {
    generator: TerrainGenerator,
    storage: Box<dyn ChunkStorage>,
    pool: ChunkPool,
    config: EngineConfig,
}

impl World {
    pub fn new(config: EngineConfig, world_config: WorldGenConfig) -> Self {
        let storage = Box::new(FileChunkStorage::new("world"));
        let pool = ChunkPool::new(1000); // Maximum 1000 chunks in pool
        let generator = TerrainGenerator::new(world_config, Arc::new(Default::default()));

        Self {
            generator,
            storage,
            pool,
            config,
        }
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<&Block> {
        let chunk_coord = ChunkCoord::from_world_pos(Vec3::new(x as f32, y as f32, z as f32), 32);
        let (local_x, local_y, local_z) = self.get_local_coords(x, y, z);

        if let Some(chunk) = self.storage.get_chunk(&chunk_coord) {
            chunk.get_block(local_x as u32, local_y as u32, local_z as u32)
        } else {
            None
        }
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: Option<Block>) {
        let chunk_coord = ChunkCoord::from_world_pos(Vec3::new(x as f32, y as f32, z as f32), 32);
        let (local_x, local_y, local_z) = self.get_local_coords(x, y, z);

        if let Some(chunk) = self.storage.get_chunk_mut(&chunk_coord) {
            chunk.set_block(local_x as u32, local_y as u32, local_z as u32, block);
        }
    }

    fn get_local_coords(&self, x: i32, y: i32, z: i32) -> (i32, i32, i32) {
        let chunk_size = 32;
        let local_x = ((x % chunk_size) + chunk_size) % chunk_size;
        let local_y = ((y % chunk_size) + chunk_size) % chunk_size;
        let local_z = ((z % chunk_size) + chunk_size) % chunk_size;
        (local_x, local_y, local_z)
    }

    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<&Chunk> {
        self.storage.get_chunk(coord).map(|arc| arc.as_ref())
    }

    pub fn get_chunk_mut(&mut self, coord: ChunkCoord) -> Option<&mut Chunk> {
        self.storage.get_chunk_mut(coord).and_then(Arc::get_mut)
    }

    pub fn has_chunk(&self, coord: ChunkCoord) -> bool {
        self.storage.get_chunk(coord).is_some()
    }

    pub fn set_chunk(&mut self, coord: ChunkCoord, chunk: Chunk) {
        self.storage.set_chunk(coord, Arc::new(chunk));
    }

    pub fn generate_chunk(&mut self, coord: ChunkCoord) {
        if !self.storage.has_chunk(&coord) {
            let chunk = self.generator.generate_chunk(coord);
            self.storage.set_chunk(coord, chunk);
        }
    }
}
