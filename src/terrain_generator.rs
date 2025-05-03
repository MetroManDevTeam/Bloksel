use std::collections::HashMap;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use noise::{NoiseFn, Perlin};

use crate::block::{BlockData, Orientation, Integrity};

pub type ChunkCoord = (i32, i32);

pub struct Chunk {
    pub blocks: Vec<Vec<Vec<Option<BlockData>>>>,
    pub chunk_size: usize,
    pub sub_resolution: usize,
}

impl Chunk {
    pub fn new(chunk_size: usize, sub_resolution: usize) -> Self {
        let blocks = vec![vec![vec![None; chunk_size]; chunk_size]; chunk_size];
        Chunk {
            blocks,
            chunk_size,
            sub_resolution,
        }
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: BlockData) {
        if x < self.chunk_size && y < self.chunk_size && z < self.chunk_size {
            self.blocks[x][y][z] = Some(block);
        }
    }
}

pub struct TerrainGenerator {
    seed: u32,
    noise: Perlin,
    chunk_size: usize,
    sub_resolution: usize,
}

impl TerrainGenerator {
    pub fn new(seed: u32, chunk_size: usize, sub_resolution: usize) -> Self {
        TerrainGenerator {
            seed,
            noise: Perlin::new(),
            chunk_size,
            sub_resolution,
        }
    }

    pub fn generate_into_chunk(&self, chunk: &mut Chunk, coord: ChunkCoord) {
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed as u64 + (coord.0 as u64) * 341873128712 + (coord.1 as u64) * 132897987541);

        for x in 0..self.chunk_size {
            for z in 0..self.chunk_size {
                let wx = coord.0 * self.chunk_size as i32 + x as i32;
                let wz = coord.1 * self.chunk_size as i32 + z as i32;

                // Basic terrain height using noise
                let height = self.get_height(wx as f64, wz as f64);

                for y in 0..self.chunk_size {
                    let wy = y as i32;

                    let block_id = if wy > height {
                        0 // air
                    } else if wy == height {
                        2 // grass
                    } else if wy >= height - 3 {
                        3 // dirt
                    } else if wy < 5 {
                        5 // water
                    } else {
                        1 // stone
                    };

                    if block_id == 0 {
                        continue;
                    }

                    let mut block = BlockData::new(block_id);

                    for sx in 0..self.sub_resolution {
                        for sy in 0..self.sub_resolution {
                            for sz in 0..self.sub_resolution {
                                block.grid.insert(
                                    (sx, sy, sz),
                                    BlockData {
                                        id: block_id,
                                        integrity: Integrity::Full,
                                        orientation: Orientation::default(),
                                        ..Default::default()
                                    },
                                );
                            }
                        }
                    }

                    chunk.set_block(x, y, z, block);

                    // Pebble logic: randomly place a 1/4 stone block on grass
                    if block_id == 2 && rng.gen_bool(0.25) {
                        let mut pebble = BlockData::new(1); // stone block

                        // Only fill a quarter sub-block randomly
                        let sx = rng.gen_range(0..self.sub_resolution / 2);
                        let sy = rng.gen_range(0..self.sub_resolution / 2);
                        let sz = rng.gen_range(0..self.sub_resolution / 2);

                        pebble.grid.insert(
                            (sx as u8, sy as u8, sz as u8),
                            BlockData {
                                id: 1,
                                integrity: Integrity::Partial,
                                orientation: Orientation::default(),
                                ..Default::default()
                            },
                        );

                        chunk.set_block(x, y + 1, z, pebble);
                    }
                }
            }
        }
    }

    fn get_height(&self, x: f64, z: f64) -> i32 {
        let scale = 0.05;
        let noise_val = self.noise.get([x * scale, z * scale]);
        let base = 10.0;
        let height = (noise_val * 10.0 + base).round() as i32;
        height.clamp(1, (self.chunk_size - 1) as i32)
    }

    /// Placeholder for integrating real topographic data later
    #[allow(dead_code)]
    pub fn override_with_topographic(&self, _chunk: &mut Chunk, _topo_data: &[u8]) {
        // Future hook for DEM or other real-world terrain data
    }
}
