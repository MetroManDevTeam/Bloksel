pub mod terrain;

pub use terrain::{BiomeType, Generator, Terrain, WorldType};

use crate::world::block::Block;
use crate::world::block_id::BlockId;
use crate::world::chunk::Chunk;
use crate::world::chunk_coord::ChunkCoord;
use crate::world::generator::terrain::TerrainGenerator;
use glam::Vec3;
use rand::Rng;
use std::sync::Arc;

pub trait WorldGenerator: Send + Sync {
    fn generate_chunk(&self, coord: ChunkCoord) -> Chunk;
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block;
}

pub struct ChunkGenerator {
    terrain_generator: TerrainGenerator,
}

impl ChunkGenerator {
    pub fn new(terrain_generator: TerrainGenerator) -> Self {
        Self { terrain_generator }
    }

    pub fn generate_chunk(&mut self, coord: ChunkCoord) -> Chunk {
        self.terrain_generator.generate_chunk(coord)
    }

    fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        if y < 0 {
            Block::new(BlockId::new(1, 0, 0)) // Stone
        } else if y == 0 {
            Block::new(BlockId::new(2, 0, 0)) // Grass
        } else {
            Block::new(BlockId::new(0, 0, 0)) // Air
        }
    }
}
