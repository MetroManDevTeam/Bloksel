use std::collections::HashMap;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha12Rng;
use noise::{NoiseFn, Perlin, Seedable, utils::{NoiseMapBuilder, PlaneMapBuilder}};
use crate::block::{Block, BlockPhysics, BlockIntegrity, BlockOrientation, BlockRegistry};
use crate::world::chunk::{Chunk, ChunkCoord};

const CHUNK_SIZE: usize = 32;
const SUB_RESOLUTION: usize = 4;
const SEA_LEVEL: i32 = 62;
const BASE_TERRAIN_HEIGHT: f64 = 58.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BiomeType {
    Plains,
    Mountains,
    Desert,
    Forest,
    Ocean,
}

pub struct TerrainGenerator {
    seed: u32,
    block_registry: BlockRegistry,
    noise_layers: HashMap<String, Perlin>,
    biome_map: Vec<BiomeType>,
    height_cache: HashMap<(i32, i32), f64>,
}

impl TerrainGenerator {
    pub fn new(seed: u32, block_registry: BlockRegistry) -> Self {
        let mut generator = Self {
            seed,
            block_registry,
            noise_layers: HashMap::new(),
            biome_map: Vec::new(),
            height_cache: HashMap::new(),
        };

        generator.initialize_noise_layers();
        generator
    }

    fn initialize_noise_layers(&mut self) {
        // Primary terrain noise
        self.add_noise_layer("terrain", 0, 4.0, 0.5, 6);
        
        // Detail noise
        self.add_noise_layer("detail", 1, 32.0, 0.8, 3);
        
        // Biome noise
        self.add_noise_layer("biome", 2, 0.5, 1.0, 1);
        
        // Cave noise
        self.add_noise_layer("caves", 3, 16.0, 0.7, 4);
    }

    fn add_noise_layer(&mut self, name: &str, seed_offset: u32, frequency: f64, persistence: f64, octaves: usize) {
        let mut perlin = Perlin::new();
        perlin = perlin.set_seed(self.seed.wrapping_add(seed_offset));
        
        let mut noise = Perlin::new();
        noise.set_seed(perlin.seed());
        noise.frequency = frequency;
        noise.persistence = persistence;
        noise.octaves = octaves;
        
        self.noise_layers.insert(name.to_string(), noise);
    }

    pub fn generate_chunk(&mut self, coord: ChunkCoord) -> Chunk {
        let mut chunk = Chunk::new(CHUNK_SIZE, SUB_RESOLUTION);
        let mut rng = ChaCha12Rng::seed_from_u64(self.seed as u64 + coord.0 as u64 * 341873128712 + coord.1 as u64 * 132897987541);

        // Pre-calculate biome for chunk area
        let biome = self.calculate_biome(coord);
        
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_x = coord.0 * CHUNK_SIZE as i32 + x as i32;
                let world_z = coord.1 * CHUNK_SIZE as i32 + z as i32;
                
                let height = self.calculate_height(world_x, world_z, biome);
                let (base_block, top_block) = self.get_biome_blocks(biome);

                for y in 0..CHUNK_SIZE {
                    let world_y = y as i32;
                    let mut block_id = 0; // Air by default

                    if world_y <= height {
                        block_id = self.get_block_for_depth(
                            world_y, 
                            height,
                            base_block,
                            top_block,
                            biome
                        );

                        // Apply caves and overhangs
                        if self.should_add_cave(world_x, world_y, world_z) {
                            block_id = 0;
                        }
                    }

                    // Water in ocean biomes below sea level
                    if biome == BiomeType::Ocean && world_y <= SEA_LEVEL && block_id == 0 {
                        block_id = self.block_registry.get_id_by_name("water").unwrap_or(0);
                    }

                    if block_id != 0 {
                        let mut block = self.create_block(block_id, biome, &mut rng);
                        self.add_strata_details(&mut block, world_y, &mut rng);
                        chunk.set_block(x, y, z, block);
                    }
                }
            }
        }

        chunk
    }

    fn calculate_height(&mut self, x: i32, z: i32, biome: BiomeType) -> i32 {
        let cache_key = (x, z);
        if let Some(h) = self.height_cache.get(&cache_key) {
            return *h as i32;
        }

        let base_noise = self.sample_noise("terrain", x, z);
        let detail_noise = self.sample_noise("detail", x, z);
        let biome_mod = self.biome_height_modifier(biome);

        let height = BASE_TERRAIN_HEIGHT 
            + (base_noise * 24.0).abs()
            + (detail_noise * 6.0)
            + biome_mod;

        let final_height = height.clamp(SEA_LEVEL as f64 - 8.0, 256.0) as i32;
        self.height_cache.insert(cache_key, final_height as f64);
        final_height
    }

    fn biome_height_modifier(&self, biome: BiomeType) -> f64 {
        match biome {
            BiomeType::Mountains => 24.0,
            BiomeType::Plains => 2.0,
            BiomeType::Desert => 4.0,
            BiomeType::Forest => 6.0,
            BiomeType::Ocean => -12.0,
        }
    }

    fn calculate_biome(&mut self, coord: ChunkCoord) -> BiomeType {
        let x = coord.0 as f64 * 0.1;
        let z = coord.1 as f64 * 0.1;
        
        let temp = self.sample_noise("biome", coord.0, coord.1);
        let moisture = self.sample_noise("biome", coord.0 + 1000, coord.1 + 1000);

        match (temp, moisture) {
            (t, _) if t < -0.5 => BiomeType::Mountains,
            (t, m) if t > 0.5 && m < 0.0 => BiomeType::Desert,
            (t, m) if t > 0.3 && m > 0.4 => BiomeType::Forest,
            (_, m) if m > 0.7 => BiomeType::Ocean,
            _ => BiomeType::Plains,
        }
    }

    fn get_biome_blocks(&self, biome: BiomeType) -> (u32, u32) {
        match biome {
            BiomeType::Plains => (
                self.block_registry.get_id_by_name("dirt").unwrap_or(3),
                self.block_registry.get_id_by_name("grass").unwrap_or(2),
            ),
            BiomeType::Mountains => (
                self.block_registry.get_id_by_name("stone").unwrap_or(1),
                self.block_registry.get_id_by_name("snow").unwrap_or(5),
            ),
            BiomeType::Desert => (
                self.block_registry.get_id_by_name("sand").unwrap_or(4),
                self.block_registry.get_id_by_name("sand").unwrap_or(4),
            ),
            BiomeType::Forest => (
                self.block_registry.get_id_by_name("dirt").unwrap_or(3),
                self.block_registry.get_id_by_name("grass").unwrap_or(2),
            ),
            BiomeType::Ocean => (
                self.block_registry.get_id_by_name("sand").unwrap_or(4),
                self.block_registry.get_id_by_name("gravel").unwrap_or(6),
            ),
        }
    }

    fn get_block_for_depth(&self, y: i32, height: i32, base: u32, top: u32, biome: BiomeType) -> u32 {
        match biome {
            BiomeType::Ocean if y <= SEA_LEVEL - 8 => self.block_registry.get_id_by_name("stone").unwrap_or(1),
            _ if y == height => top,
            _ if y > height - 4 => base,
            _ => self.block_registry.get_id_by_name("stone").unwrap_or(1),
        }
    }

    fn create_block(&self, block_id: u32, biome: BiomeType, rng: &mut ChaCha12Rng) -> Block {
        let mut block = Block::uniform(
            block_id,
            SUB_RESOLUTION as u8,
            self.block_registry.get(block_id).unwrap()
        );

        // Add random variations
        match biome {
            BiomeType::Forest if block_id == self.block_registry.get_id_by_name("grass").unwrap_or(2) => {
                if rng.gen_ratio(1, 10) {
                    block.place_sub_block(
                        rng.gen_range(0..SUB_RESOLUTION as u8),
                        SUB_RESOLUTION as u8 - 1,
                        rng.gen_range(0..SUB_RESOLUTION as u8),
                        Block {
                            id: self.block_registry.get_id_by_name("flower").unwrap_or(7),
                            ..block.clone()
                        }
                    );
                }
            }
            BiomeType::Desert if block_id == self.block_registry.get_id_by_name("sand").unwrap_or(4) => {
                if rng.gen_ratio(1, 20) {
                    block.place_sub_block(
                        rng.gen_range(0..SUB_RESOLUTION as u8),
                        SUB_RESOLUTION as u8 - 1,
                        rng.gen_range(0..SUB_RESOLUTION as u8),
                        Block {
                            id: self.block_registry.get_id_by_name("dead_bush").unwrap_or(8),
                            ..block.clone()
                        }
                    );
                }
            }
            _ => {}
        }

        block
    }

    fn add_strata_details(&self, block: &mut Block, world_y: i32, rng: &mut ChaCha12Rng) {
        if world_y < SEA_LEVEL - 8 && rng.gen_ratio(1, 10) {
            let ore_type = match rng.gen_range(0..100) {
                0..=5 => "coal_ore",
                6..=8 => "iron_ore",
                9..=10 => "gold_ore",
                _ => "stone",
            };
            
            for _ in 0..rng.gen_range(1..=3) {
                block.place_sub_block(
                    rng.gen_range(0..SUB_RESOLUTION as u8),
                    rng.gen_range(0..SUB_RESOLUTION as u8),
                    rng.gen_range(0..SUB_RESOLUTION as u8),
                    Block {
                        id: self.block_registry.get_id_by_name(ore_type).unwrap_or(1),
                        integrity: BlockIntegrity::Full,
                        ..block.clone()
                    }
                );
            }
        }
    }

    fn should_add_cave(&self, x: i32, y: i32, z: i32) -> bool {
        let cave_noise = self.sample_noise("caves", x, z);
        let y_factor = 1.0 - (y as f64 / 128.0).abs();
        (cave_noise * y_factor).abs() > 0.6
    }

    fn sample_noise(&self, layer: &str, x: i32, z: i32) -> f64 {
        let noise = self.noise_layers.get(layer).unwrap();
        let scale = match layer {
            "terrain" => 0.01,
            "detail" => 0.05,
            "biome" => 0.005,
            "caves" => 0.1,
            _ => 0.1,
        };

        noise.get([
            x as f64 * scale,
            z as f64 * scale
        ])
    }

    pub fn generate_tree(&self, pos: (i32, i32, i32)) -> Vec<Block> {
        let mut blocks = Vec::new();
        let trunk_id = self.block_registry.get_id_by_name("wood").unwrap_or(9);
        let leaves_id = self.block_registry.get_id_by_name("leaves").unwrap_or(10);

        // Generate trunk
        for y in 0..5 {
            blocks.push(Block::uniform(
                trunk_id,
                SUB_RESOLUTION as u8,
                self.block_registry.get(trunk_id).unwrap()
            ));
        }

        // Generate leaves
        for dx in -2..=2 {
            for dz in -2..=2 {
                for dy in -1..=1 {
                    if dx*dx + dz*dz + dy*dy <= 4 {
                        blocks.push(Block::uniform(
                            leaves_id,
                            SUB_RESOLUTION as u8,
                            self.block_registry.get(leaves_id).unwrap()
                        ));
                    }
                }
            }
        }

        blocks
    }
}

// Extension methods for noise sampling
trait NoiseSampling {
    fn sample_3d(&self, x: f64, y: f64, z: f64) -> f64;
}

impl NoiseSampling for Perlin {
    fn sample_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        self.get([x, y, z])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biome_generation() {
        let registry = BlockRegistry::load_from_file("blocks.json").unwrap();
        let mut generator = TerrainGenerator::new(12345, registry);
        
        let coord = (0, 0);
        let biome = generator.calculate_biome(coord);
        assert!(matches!(biome, BiomeType::Plains | BiomeType::Forest | BiomeType::Mountains));
    }

    #[test]
    fn test_height_calculation() {
        let registry = BlockRegistry::load_from_file("blocks.json").unwrap();
        let mut generator = TerrainGenerator::new(12345, registry);
        
        let height = generator.calculate_height(0, 0, BiomeType::Plains);
        assert!(height >= SEA_LEVEL - 8 && height <= 256);
    }
}
