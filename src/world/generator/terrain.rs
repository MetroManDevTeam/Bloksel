use crate::world::BlockFacing;
use crate::world::BlockOrientation;
use crate::world::block::Block;
use crate::world::block::SubBlock;
use crate::world::block_id::BlockId;
use crate::world::block_registry::BlockRegistry;
use crate::world::block_visual::ConnectedDirections;
use crate::world::chunk::Chunk;
use crate::world::chunk_coord::ChunkCoord;
use glam::IVec3;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use parking_lot::RwLock;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::Arc;

// Constants
const CHUNK_SIZE: usize = 16;
const SUB_RESOLUTION: usize = 4;
const SEA_LEVEL: i32 = 64;
const BASE_TERRAIN_HEIGHT: f64 = 64.0;
const FLAT_WORLD_HEIGHT: i32 = 64;

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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WorldType {
    Normal,
    Flat,
    Amplified,
    LargeBiomes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainConfig {
    pub seed: u32,
    pub world_type: WorldType,
    pub terrain_amplitude: f64,
    pub world_scale: f64,
    pub cave_threshold: f64,
    pub flat_world_layers: Vec<(BlockId, u32)>,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            world_type: WorldType::Normal,
            terrain_amplitude: 32.0,
            world_scale: 0.01,
            cave_threshold: 0.7,
            flat_world_layers: Vec::new(),
        }
    }
}

pub struct TerrainGenerator {
    config: TerrainConfig,
    block_registry: Arc<BlockRegistry>,
    noise_layers: RwLock<HashMap<String, Fbm<Perlin>>>,
    height_cache: RwLock<HashMap<(i32, i32), f64>>,
    biome_cache: RwLock<HashMap<(i32, i32), BiomeType>>,
    chunk_cache: RwLock<HashMap<ChunkCoord, Arc<Chunk>>>,
}

impl TerrainGenerator {
    pub fn new(seed: u32, block_registry: Arc<BlockRegistry>, world_type: WorldType) -> Self {
        let mut config = TerrainConfig {
            seed,
            world_type,
            ..Default::default()
        };

        // Adjust parameters based on world type
        match world_type {
            WorldType::Amplified => {
                config.terrain_amplitude = 48.0;
                config.world_scale = 0.005;
            }
            WorldType::LargeBiomes => {
                config.world_scale = 0.002;
            }
            _ => {}
        }

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

    fn initialize_noise_layers(&mut self) {
        let mut layers = self.noise_layers.write();

        // Primary terrain noise
        layers.insert("terrain".into(), self.create_noise_layer(0, 0.01, 0.5, 6));

        // Detail noise
        layers.insert("detail".into(), self.create_noise_layer(1, 0.05, 0.8, 3));

        // Biome noise
        layers.insert("biome".into(), self.create_noise_layer(2, 0.001, 1.0, 1));

        // Cave noise
        layers.insert("caves".into(), self.create_noise_layer(3, 0.03, 0.7, 4));
    }

    fn create_noise_layer(
        &self,
        seed_offset: u32,
        frequency: f64,
        persistence: f64,
        octaves: usize,
    ) -> Fbm<Perlin> {
        Fbm::<Perlin>::new(self.config.seed + seed_offset)
            .set_octaves(octaves)
            .set_frequency(frequency)
            .set_persistence(persistence)
            .set_lacunarity(2.0)
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
        let mut chunk = Chunk::new(coord);

        match self.config.world_type {
            WorldType::Normal => self.generate_normal_chunk(&mut chunk, coord),
            WorldType::Flat => self.generate_flat_chunk(&mut chunk, coord),
            WorldType::Amplified => self.generate_amplified_chunk(&mut chunk, coord),
            WorldType::LargeBiomes => self.generate_large_biomes_chunk(&mut chunk, coord),
        }

        Arc::new(chunk)
    }

    fn generate_normal_chunk(&self, chunk: &mut Chunk, coord: ChunkCoord) {
        let mut rng = ChaCha12Rng::seed_from_u64(
            self.config.seed as u64
                + coord.x() as u64 * 341873128712
                + coord.z() as u64 * 132897987541,
        );

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_x = coord.x() * CHUNK_SIZE as i32 + x as i32;
                let world_z = coord.z() * CHUNK_SIZE as i32 + z as i32;

                let biome = self.calculate_biome(world_x, world_z);
                let height = self.calculate_height(world_x, world_z, biome);
                let (base_block, top_block) = self.get_biome_blocks(biome);

                for y in 0..CHUNK_SIZE {
                    let world_y = coord.y() * CHUNK_SIZE as i32 + y as i32;
                    let mut block_id = BlockId::new(0);

                    if world_y <= height {
                        block_id =
                            self.get_block_for_depth(world_y, height, base_block, top_block, biome);

                        if self.should_add_cave(world_x, world_y, world_z) {
                            block_id = BlockId::new(0);
                        }
                    }

                    if biome == BiomeType::Ocean
                        && world_y <= SEA_LEVEL
                        && block_id == BlockId::new(0)
                    {
                        block_id = BlockId::new(10);
                    }

                    if block_id != BlockId::new(0) {
                        let mut block = self.create_block(block_id, biome, &mut rng);
                        self.add_strata_details(&mut block, world_y, &mut rng);
                        chunk.set_block(x, y, z, Some(block));
                    }
                }
            }
        }
    }

    fn generate_flat_chunk(&self, chunk: &mut Chunk, coord: ChunkCoord) {
        let mut rng = ChaCha12Rng::seed_from_u64(
            self.config.seed as u64
                + coord.x() as u64 * 341873128712
                + coord.z() as u64 * 132897987541,
        );

        let mut current_height = FLAT_WORLD_HEIGHT;

        // Generate layers from top to bottom
        for (block_id, thickness) in &self.config.flat_world_layers {
            for _ in 0..*thickness {
                for x in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        let chunk_y = current_height - coord.y() * CHUNK_SIZE as i32;
                        if chunk_y >= 0 && chunk_y < CHUNK_SIZE as i32 {
                            let block = self.create_block(*block_id, BiomeType::Plains, &mut rng);
                            chunk.set_block(x, chunk_y as usize, z, Some(block));
                        }
                    }
                }
                current_height -= 1;
            }
        }

        // Fill with bedrock at the bottom
        if current_height > 0 {
            let bedrock_id = self
                .block_registry
                .get_by_name("bedrock")
                .map(|def| def.id)
                .unwrap_or(BlockId::from(10));
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let chunk_y = current_height - coord.y() * CHUNK_SIZE as i32;
                    if chunk_y >= 0 && chunk_y < CHUNK_SIZE as i32 {
                        let block = self.create_block(bedrock_id, BiomeType::Plains, &mut rng);
                        chunk.set_block(x, chunk_y as usize, z, Some(block));
                    }
                }
            }
        }
    }

    fn generate_amplified_chunk(&self, chunk: &mut Chunk, coord: ChunkCoord) {
        // Similar to normal but with more extreme terrain
        self.generate_normal_chunk(chunk, coord)
    }

    fn generate_large_biomes_chunk(&self, chunk: &mut Chunk, coord: ChunkCoord) {
        // Similar to normal but with larger biome areas
        self.generate_normal_chunk(chunk, coord)
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

        let height = match self.config.world_type {
            WorldType::Amplified => {
                BASE_TERRAIN_HEIGHT
                    + (base_noise * self.config.terrain_amplitude * 2.0).abs()
                    + (detail_noise * 12.0)
                    + biome_mod * 1.5
            }
            _ => {
                BASE_TERRAIN_HEIGHT
                    + (base_noise * self.config.terrain_amplitude).abs()
                    + (detail_noise * 6.0)
                    + biome_mod
            }
        };

        let final_height = height.clamp(SEA_LEVEL as f64 - 8.0, 256.0) as i32;
        self.height_cache
            .write()
            .insert(cache_key, final_height as f64);
        final_height
    }

    fn biome_height_modifier(&self, biome: BiomeType) -> f64 {
        match biome {
            BiomeType::Mountains => 15.0,
            BiomeType::Plains => 2.0,
            BiomeType::Desert => -3.0,
            BiomeType::Forest => 4.0,
            BiomeType::Ocean => -8.0,
            BiomeType::Tundra => 6.0,
            BiomeType::Swamp => -2.0,
        }
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
                self.block_registry
                    .get_by_name("dirt")
                    .map(|def| def.id)
                    .unwrap_or(BlockId::from(10)),
                self.block_registry
                    .get_by_name("grass")
                    .map(|def| def.id)
                    .unwrap_or(BlockId::from(10)),
            ),
            BiomeType::Mountains | BiomeType::Tundra => (
                self.block_registry
                    .get_by_name("stone")
                    .map(|def| def.id)
                    .unwrap_or(BlockId::from(10)),
                self.block_registry
                    .get_by_name("snow")
                    .map(|def| def.id)
                    .unwrap_or(BlockId::from(10)),
            ),
            BiomeType::Desert => (
                self.block_registry
                    .get_by_name("sand")
                    .map(|def| def.id)
                    .unwrap_or(BlockId::from(10)),
                self.block_registry
                    .get_by_name("sand")
                    .map(|def| def.id)
                    .unwrap_or(BlockId::from(10)),
            ),
            BiomeType::Forest => (
                self.block_registry
                    .get_by_name("dirt")
                    .map(|def| def.id)
                    .unwrap_or(BlockId::from(10)),
                self.block_registry
                    .get_by_name("grass")
                    .map(|def| def.id)
                    .unwrap_or(BlockId::from(10)),
            ),
            BiomeType::Ocean => (
                self.block_registry
                    .get_by_name("sand")
                    .map(|def| def.id)
                    .unwrap_or(BlockId::from(10)),
                self.block_registry
                    .get_by_name("gravel")
                    .map(|def| def.id)
                    .unwrap_or(BlockId::from(10)),
            ),
        }
    }

    fn get_block_for_depth(
        &self,
        y: i32,
        height: i32,
        base: BlockId,
        top: BlockId,
        biome: BiomeType,
    ) -> BlockId {
        match biome {
            BiomeType::Ocean if y <= SEA_LEVEL - 8 => self
                .block_registry
                .get_by_name("stone")
                .map(|def| def.id)
                .unwrap_or(BlockId::from(10)),
            _ if y == height => top,
            _ if y > height - 4 => base,
            _ => self
                .block_registry
                .get_by_name("stone")
                .map(|def| def.id)
                .unwrap_or(BlockId::from(10)),
        }
    }

    fn create_block(&self, id: BlockId, biome: BiomeType, rng: &mut ChaCha12Rng) -> Block {
        let mut block = Block::new(id.into());

        // Add biome-specific features
        match biome {
            BiomeType::Forest
                if id
                    == self
                        .block_registry
                        .get_by_name("grass")
                        .map(|def| def.id)
                        .unwrap_or(BlockId::from(10)) =>
            {
                if rng.random_ratio(1, 10) {
                    block.place_sub_block(
                        rng.random_range(0..SUB_RESOLUTION as u8),
                        rng.random_range(0..SUB_RESOLUTION as u8),
                        rng.random_range(0..SUB_RESOLUTION as u8),
                        SubBlock {
                            id: self
                                .block_registry
                                .get_by_name("grass")
                                .map(|def| def.id)
                                .unwrap_or(BlockId::from(10)),
                            metadata: 0,
                            facing: BlockFacing::None,
                            orientation: BlockOrientation::None,
                            connections: ConnectedDirections::empty(),
                        },
                    );
                }
            }
            BiomeType::Swamp
                if id
                    == self
                        .block_registry
                        .get_by_name("water")
                        .map(|def| def.id)
                        .unwrap_or(BlockId::from(10)) =>
            {
                block.place_sub_block(
                    rng.random_range(0..SUB_RESOLUTION as u8),
                    rng.random_range(0..SUB_RESOLUTION as u8),
                    rng.random_range(0..SUB_RESOLUTION as u8),
                    SubBlock {
                        id: self
                            .block_registry
                            .get_by_name("water")
                            .map(|def| def.id)
                            .unwrap_or(BlockId::from(10)),
                        metadata: 0,
                        facing: BlockFacing::None,
                        orientation: BlockOrientation::None,
                        connections: ConnectedDirections::empty(),
                    },
                );
            }
            _ => {}
        }

        block
    }

    fn add_strata_details(&self, block: &mut Block, world_y: i32, rng: &mut ChaCha12Rng) {
        if world_y < SEA_LEVEL - 8 && rng.random_ratio(1, 10) {
            let ore_type = match rng.random_range(0..100) {
                0..=5 => "coal_ore",
                6..=8 => "iron_ore",
                9..=10 => "gold_ore",
                11..=12 => "diamond_ore",
                _ => "stone",
            };

            for _ in 0..rng.random_range(1..=3) {
                block.place_sub_block(
                    rng.random_range(0..SUB_RESOLUTION as u8),
                    rng.random_range(0..SUB_RESOLUTION as u8),
                    rng.random_range(0..SUB_RESOLUTION as u8),
                    SubBlock {
                        id: self
                            .block_registry
                            .get_by_name(ore_type)
                            .map(|def| def.id)
                            .unwrap_or(BlockId::from(10)),
                        metadata: 0,
                        facing: BlockFacing::None,
                        orientation: BlockOrientation::None,
                        connections: ConnectedDirections::empty(),
                    },
                );
            }
        }
    }

    fn should_add_cave(&self, x: i32, y: i32, z: i32) -> bool {
        if self.config.world_type == WorldType::Flat {
            return false;
        }

        let cave_noise = self.sample_noise("caves", x, z);
        let y_factor = 1.0 - (y as f64 / 128.0).abs();
        (cave_noise * y_factor).abs() > self.config.cave_threshold
    }

    fn sample_noise(&self, layer: &str, x: i32, z: i32) -> f64 {
        let layers = self.noise_layers.read();
        let noise = layers.get(layer).unwrap();
        noise.get([
            x as f64 * self.config.world_scale,
            z as f64 * self.config.world_scale,
        ])
    }

    pub fn generate_tree(&self, base_pos: IVec3) -> HashMap<IVec3, Block> {
        let mut blocks = HashMap::new();
        let trunk_id = self
            .block_registry
            .get_by_name("log")
            .map(|def| def.id)
            .unwrap_or(BlockId::from(10));
        let leaves_id = self
            .block_registry
            .get_by_name("leaves")
            .map(|def| def.id)
            .unwrap_or(BlockId::from(10));
        let height = 4 + (base_pos.x.abs() % 3) as i32;

        // Generate trunk
        for y in 0..height {
            let pos = IVec3::new(base_pos.x, base_pos.y + y, base_pos.z);
            blocks.insert(pos, Block::new(trunk_id.into()));
        }

        // Generate leaves
        let center = IVec3::new(base_pos.x, base_pos.y + height - 2, base_pos.z);
        for dx in -2..=2 {
            for dz in -2..=2 {
                for dy in -1..=1 {
                    if dx * dx + dz * dz + dy * dy <= 4 {
                        let pos = center + IVec3::new(dx, dy, dz);
                        blocks.insert(pos, Block::new(leaves_id.into()));
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
