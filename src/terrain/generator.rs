use noise::{NoiseFn, Perlin};
use crate::core::{Chunk, Block, BlockProperties};
use crate::registry::BlockRegistry;
use glam::Vec3;

pub struct TerrainGenerator {
    noise: Perlin,
    stone_noise: Perlin,
    seed: u32,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            noise: Perlin::new(seed),
            stone_noise: Perlin::new(seed.wrapping_add(1)),
            seed,
        }
    }

    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32, registry: &BlockRegistry) -> Chunk {
        let mut chunk = Chunk::new(chunk_x, chunk_z);
        let block_size = 1.0;

        for x in 0..Chunk::SIZE {
            for z in 0..Chunk::SIZE {
                // World coordinates
                let world_x = (chunk_x * Chunk::SIZE as i32) + x as i32;
                let world_z = (chunk_z * Chunk::SIZE as i32) + z as i32;

                // Base terrain height (50-70 blocks)
                let height = self.get_base_height(world_x, world_z);

                // Stone layer starts 5 blocks below surface
                let stone_level = height - 5;

                for y in 0..height {
                    let block = if y >= stone_level {
                        // Surface layers
                        self.get_surface_block(world_x, y, world_z, height, registry)
                    } else {
                        // Underground - stone with random deposits
                        self.get_stone_block(world_x, y, world_z, registry)
                    };

                    chunk.set_block(x as usize, y as usize, z as usize, block);
                }
            }
        }

        chunk
    }

    fn get_base_height(&self, x: i32, z: i32) -> i32 {
        let xf = x as f64 * 0.03;
        let zf = z as f64 * 0.03;
        
        // Base height between 50-70
        let base = 60.0;
        let variation = self.noise.get([xf, zf]) * 10.0;
        
        (base + variation) as i32
    }

    fn get_surface_block(&self, x: i32, y: i32, z: i32, height: i32, registry: &BlockRegistry) -> Block {
        let xf = x as f64 * 0.1;
        let zf = z as f64 * 0.1;
        
        // Slope detection (1 in 4 chance to be half block on slopes)
        let slope = self.noise.get([xf, zf]).abs();
        let is_slope = slope > 0.3;
        let is_half_block = is_slope && (x + z + y) % 4 == 0;

        if y == height - 1 {
            // Top layer - grass
            if is_half_block {
                registry.create_block("3HNM1.0S", (x, y, z)).unwrap() // Half grass
            } else {
                registry.create_block("3FNM1.0S", (x, y, z)).unwrap() // Full grass
            }
        } else if y >= height - 4 {
            // Subsurface - dirt
            registry.create_block("2FNM1.0S", (x, y, z)).unwrap()
        } else {
            // Stone layer
            registry.create_block("1FNM1.0S", (x, y, z)).unwrap()
        }
    }

    fn get_stone_block(&self, x: i32, y: i32, z: i32, registry: &BlockRegistry) -> Block {
        // 25% chance for stone deposit variant
        let deposit_value = self.stone_noise.get([
            x as f64 * 0.2, 
            y as f64 * 0.2, 
            z as f64 * 0.2
        ]);

        if deposit_value > 0.75 {
            registry.create_block("1FNM1.5S", (x, y, z)).unwrap() // Denser stone
        } else {
            registry.create_block("1FNM1.0S", (x, y, z)).unwrap() // Regular stone
        }
    }
}
