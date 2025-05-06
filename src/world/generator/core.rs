pub mod terrain;

pub use terrain::{BiomeType, Generator, Terrain, WorldType};

use crate::world::BlockFacing;
use crate::world::BlockOrientation;
use crate::world::block::Block;
use crate::world::chunk::Chunk;
use crate::world::chunk_coord::ChunkCoord;
use std::sync::Arc;

pub trait WorldGenerator: Send + Sync {
    fn generate_chunk(&self, coord: ChunkCoord) -> Chunk;
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block;
}

pub struct SimpleGenerator;

impl WorldGenerator for SimpleGenerator {
    fn generate_chunk(&self, coord: ChunkCoord) -> Chunk {
        let mut chunk = Chunk::new(coord);
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let block = self.get_block(
                        coord.x() * 16 + x as i32,
                        coord.y() * 16 + y as i32,
                        coord.z() * 16 + z as i32,
                    );
                    chunk.set_block(x, y, z, block);
                }
            }
        }
        chunk
    }

    fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        if y < 0 {
            Block::new(1) // Stone
        } else if y == 0 {
            Block::new(2) // Grass
        } else {
            Block::new(0) // Air
        }
    }
}
