use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use glam::{IVec3, Vec3};
use noise::{NoiseFn, Perlin, Seedable};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha12Rng;
use serde::{Serialize, Deserialize};

use crate::block::{Block, BlockId, BlockRegistry};
use crate::chunk::{Chunk, ChunkCoord};

const CHUNK_SIZE: usize = 32;
const SUB_RESOLUTION: usize = 4;
const SEA_LEVEL: i32 = 62;
const BASE_TERRAIN_HEIGHT: f64 = 58.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BiomeType {
    Plains,
    Mountains,
    Desert,
    Forest,
    Ocean,
    Tundra,
    Swamp,
}

#[derive(Serialize, Deserialize)]
pub struct TerrainConfig {
    pub seed: u32,
    pub world_scale: f64,
    pub terrain_amplitude: f64,
    pub cave_threshold: f64,
}

pub struct TerrainGenerator {
    config: TerrainConfig,
    block_registry: Arc<BlockRegistry>,
    noise_layers: RwLock<HashMap<String, Perlin>>,
    height_cache: RwLock<HashMap<(i32, i32), f64>>,
    biome_cache: RwLock<HashMap<(i32, i32), BiomeType>>,
    chunk_cache: RwLock<HashMap<ChunkCoord, Arc<Chunk>>>,
}

impl TerrainGenerator {
    pub fn new(seed: u32, block_registry: Arc<BlockRegistry>) -> Self {
        let config = TerrainConfig {
            seed,
            world_scale: 0.01,
            terrain_amplitude: 24.0,
            cave_threshold: 0.6,
        };

        let mut generator = Self {
            config,
            block_registry,
            noise_layers: RwLock::new(HashMap::new()),
            height_cache: RwLock::new(HashMap::new()),
            biome_cache: RwLock::new(HashMap::new()),
            chunk_cache: RwLock::new(HashMap::new()),
        };

        generator.initialize_noise_layers();
        generator
    }

    fn initialize_noise_layers(&self) {
        let mut layers = self.noise_layers.write();
        
        // Primary terrain noise
        layers.insert("terrain".into(), self.create_noise_layer(0, 4.0, 0.5, 6));
        
        // Detail noise
        layers.insert("detail".into(), self.create_noise_layer(1, 32.0, 0.8, 3));
        
        // Biome noise
        layers.insert("biome".into(), self.create_noise_layer(2, 0.5, 1.0, 1));
        
        // Cave noise
        layers.insert("caves".into(), self.create_noise_layer(3, 16.0, 0.7, 4));
    }

    fn create_noise_layer(&self, seed_offset: u32, frequency: f64, persistence: f64, octaves: usize) -> Perlin {
        let mut perlin = Perlin::new();
        perlin.set_seed(self.config.seed.wrapping_add(seed_offset));
        perlin.frequency = frequency;
        perlin.persistence = persistence;
        perlin.octaves = octaves;
        perlin
    }

    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        {
            let cache = self.chunk_cache.read();
            if let Some(chunk) = cache.get(&coord) {
                return Some(chunk.clone());
            }
        }

        let chunk = self.generate_chunk(coord);
        let mut cache = self.chunk_cache.write();
        cache.insert(coord, chunk.clone());
        Some(chunk)
    }

    pub fn generate_chunk(&self, coord: ChunkCoord) -> Arc<Chunk> {
        let mut chunk = Chunk::new(CHUNK_SIZE, SUB_RESOLUTION, coord);
        let mut rng = ChaCha12Rng::seed_from_u64(
            self.config.seed as u64 + 
            coord.x as u64 * 341873128712 + 
            coord.z as u64 * 132897987541
        );

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_x = coord.x * CHUNK_SIZE as i32 + x as i32;
                let world_z = coord.z * CHUNK_SIZE as i32 + z as i32;
                
                let biome = self.calculate_biome(world_x, world_z);
                let height = self.calculate_height(world_x, world_z, biome);
                let (base_block, top_block) = self.get_biome_blocks(biome);

                for y in 0..CHUNK_SIZE {
                    let world_y = coord.y * CHUNK_SIZE as i32 + y as i32;
                    let mut block_id = BlockId::AIR;

                    if world_y <= height {
                        block_id = self.get_block_for_depth(
                            world_y, 
                            height,
                            base_block,
                            top_block,
                            biome
                        );

                        if self.should_add_cave(world_x, world_y, world_z) {
                            block_id = BlockId::AIR;
                        }
                    }

                    if biome == BiomeType::Ocean && world_y <= SEA_LEVEL && block_id == BlockId::AIR {
                        block_id = self.block_registry.get_id_by_name("water").unwrap_or(BlockId::AIR);
                    }

                    if block_id != BlockId::AIR {
                        let mut block = self.create_block(block_id, biome, &mut rng);
                        self.add_strata_details(&mut block, world_y, &mut rng);
                        chunk.set_block(x, y, z, Some(block));
                    }
                }
            }
        }

        Arc::new(chunk)
    }

    fn calculate_height(&self, x: i32, z: i32, biome: BiomeType) -> i32 {
        let cache_key = (x, z);
        {
            let cache = self.height_cache.read();
            if let Some(h) = cache.get(&cache_key) {
                return *h as i32;
            }
        }

        let base_noise = self.sample_noise("terrain", x, z);
        let detail_noise = self.sample_noise("detail", x, z);
        let biome_mod = self.biome_height_modifier(biome);

        let height = BASE_TERRAIN_HEIGHT 
            + (base_noise * self.config.terrain_amplitude).abs()
            + (detail_noise * 6.0)
            + biome_mod;

        let final_height = height.clamp(SEA_LEVEL as f64 - 8.0, 256.0) as i32;
        self.height_cache.write().insert(cache_key, final_height as f64);
        final_height
    }

    fn calculate_biome(&self, x: i32, z: i32) -> BiomeType {
        let cache_key = (x, z);
        {
            let cache = self.biome_cache.read();
            if let Some(b) = cache.get(&cache_key) {
                return *b;
            }
        }

        let temp = self.sample_noise("biome", x, z);
        let moisture = self.sample_noise("biome", x + 1000, z + 1000);

        let biome = match (temp, moisture) {
            (t, _) if t < -0.5 => BiomeType::Mountains,
            (t, m) if t > 0.5 && m < 0.0 => BiomeType::Desert,
            (t, m) if t > 0.3 && m > 0.4 => BiomeType::Forest,
            (_, m) if m > 0.7 => BiomeType::Ocean,
            (t, _) if t < -0.3 => BiomeType::Tundra,
            (_, m) if m > 0.5 => BiomeType::Swamp,
            _ => BiomeType::Plains,
        };

        self.biome_cache.write().insert(cache_key, biome);
        biome
    }

    fn get_biome_blocks(&self, biome: BiomeType) -> (BlockId, BlockId) {
        match biome {
            BiomeType::Plains | BiomeType::Swamp => (
                self.block_registry.get_id_by_name("dirt").unwrap_or(BlockId::from(3)),
                self.block_registry.get_id_by_name("grass").unwrap_or(BlockId::from(2)),
            ),
            BiomeType::Mountains | BiomeType::Tundra => (
                self.block_registry.get_id_by_name("stone").unwrap_or(BlockId::from(1)),
                self.block_registry.get_id_by_name("snow").unwrap_or(BlockId::from(5)),
            ),
            BiomeType::Desert => (
                self.block_registry.get_id_by_name("sand").unwrap_or(BlockId::from(4)),
                self.block_registry.get_id_by_name("sand").unwrap_or(BlockId::from(4)),
            ),
            BiomeType::Forest => (
                self.block_registry.get_id_by_name("dirt").unwrap_or(BlockId::from(3)),
                self.block_registry.get_id_by_name("grass").unwrap_or(BlockId::from(2)),
            ),
            BiomeType::Ocean => (
                self.block_registry.get_id_by_name("sand").unwrap_or(BlockId::from(4)),
                self.block_registry.get_id_by_name("gravel").unwrap_or(BlockId::from(6)),
            ),
        }
    }

    fn get_block_for_depth(&self, y: i32, height: i32, base: BlockId, top: BlockId, biome: BiomeType) -> BlockId {
        match biome {
            BiomeType::Ocean if y <= SEA_LEVEL - 8 => 
                self.block_registry.get_id_by_name("stone").unwrap_or(BlockId::from(1)),
            _ if y == height => top,
            _ if y > height - 4 => base,
            _ => self.block_registry.get_id_by_name("stone").unwrap_or(BlockId::from(1)),
        }
    }

    fn create_block(&self, id: BlockId, biome: BiomeType, rng: &mut ChaCha12Rng) -> Block {
        let mut block = Block::new(id, SUB_RESOLUTION as u8);

        // Add biome-specific features
        match biome {
            BiomeType::Forest if id == self.block_registry.get_id_by_name("grass").unwrap_or(BlockId::from(2)) => {
                if rng.gen_ratio(1, 10) {
                    block.place_sub_block(
                        rng.gen_range(0..SUB_RESOLUTION as u8),
                        SUB_RESOLUTION as u8 - 1,
                        rng.gen_range(0..SUB_RESOLUTION as u8),
                        Block::new(
                            self.block_registry.get_id_by_name("flower").unwrap_or(BlockId::from(7)),
                            SUB_RESOLUTION as u8
                        )
                    );
                }
            },
            BiomeType::Swamp if id == self.block_registry.get_id_by_name("water").unwrap_or(BlockId::from(11)) => {
                block.place_sub_block(
                    rng.gen_range(0..SUB_RESOLUTION as u8),
                    0,
                    rng.gen_range(0..SUB_RESOLUTION as u8),
                    Block::new(
                        self.block_registry.get_id_by_name("lily_pad").unwrap_or(BlockId::from(12)),
                        SUB_RESOLUTION as u8
                    )
                );
            },
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
                11..=12 => "diamond_ore",
                _ => "stone",
            };
            
            for _ in 0..rng.gen_range(1..=3) {
                block.place_sub_block(
                    rng.gen_range(0..SUB_RESOLUTION as u8),
                    rng.gen_range(0..SUB_RESOLUTION as u8),
                    rng.gen_range(0..SUB_RESOLUTION as u8),
                    Block::new(
                        self.block_registry.get_id_by_name(ore_type).unwrap_or(BlockId::from(1)),
                        SUB_RESOLUTION as u8
                    )
                );
            }
        }
    }

    fn should_add_cave(&self, x: i32, y: i32, z: i32) -> bool {
        let cave_noise = self.sample_noise("caves", x, z);
        let y_factor = 1.0 - (y as f64 / 128.0).abs();
        (cave_noise * y_factor).abs() > self.config.cave_threshold
    }

    fn sample_noise(&self, layer: &str, x: i32, z: i32) -> f64 {
        let layers = self.noise_layers.read();
        let noise = layers.get(layer).unwrap();
        noise.get([x as f64 * self.config.world_scale, z as f64 * self.config.world_scale])
    }

    pub fn generate_tree(&self, pos: IVec3) -> Vec<Block> {
        let mut blocks = Vec::new();
        let trunk_id = self.block_registry.get_id_by_name("log").unwrap_or(BlockId::from(9));
        let leaves_id = self.block_registry.get_id_by_name("leaves").unwrap_or(BlockId::from(10));

        // Generate trunk (4-6 blocks tall)
        let height = 4 + (pos.x % 3) as usize;
        for y in 0..height {
            blocks.push(Block::new(trunk_id, SUB_RESOLUTION as u8));
        }

        // Generate leaves canopy
        let center = IVec3::new(pos.x, pos.y + height as i32 - 2, pos.z);
        for dx in -2..=2 {
            for dz in -2..=2 {
                for dy in -1..=1 {
                    if dx*dx + dz*dz + dy*dy <= 4 {
                        blocks.push(Block::new(leaves_id, SUB_RESOLUTION as u8));
                    }
                }
            }
        }

        blocks
    }

    pub fn clear_cache(&self) {
        self.height_cache.write().clear();
        self.biome_cache.write().clear();
        self.chunk_cache.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockRegistry;

    #[test]
    fn test_biome_generation() {
        let registry = Arc::new(BlockRegistry::initialize_default());
        let generator = TerrainGenerator::new(12345, registry);
        
        let biome = generator.calculate_biome(0, 0);
        assert!(matches!(
            biome,
            BiomeType::Plains | BiomeType::Forest | 
            BiomeType::Mountains | BiomeType::Ocean
        ));
    }

    #[test]
    fn test_height_calculation() {
        let registry = Arc::new(BlockRegistry::initialize_default());
        let generator = TerrainGenerator::new(12345, registry);
        
        let height = generator.calculate_height(0, 0, BiomeType::Plains);
        assert!(height >= SEA_LEVEL - 8 && height <= 256);
    }

    #[test]
    fn test_chunk_generation() {
        let registry = Arc::new(BlockRegistry::initialize_default());
        let generator = TerrainGenerator::new(12345, registry);
        
        let chunk = generator.generate_chunk(ChunkCoord::new(0, 0, 0));
        assert!(chunk.blocks.iter().flatten().flatten().any(|b| b.is_some()));
    }
            }
