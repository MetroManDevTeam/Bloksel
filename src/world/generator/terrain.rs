use crate::world::BlockOrientation;
use crate::world::block::{Block, SubBlock};
use crate::world::block_facing::BlockFacing;
use crate::world::block_id::BlockId;
use crate::world::block_visual::ConnectedDirections;
use crate::world::blocks_data::BlockRegistry;
use crate::world::chunk::Chunk;
use crate::world::chunk_coord::ChunkCoord;
use glam::IVec3;
use noise::{NoiseFn, Perlin};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldType {
    Normal,
    Flat,
    Superflat,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldGenConfig {
    pub world_seed: u64,
    pub terrain_height: i32,
    pub water_level: i32,
    pub biome_scale: f64,
    pub noise_scale: f64,
    pub octaves: u32,
    pub persistence: f64,
    pub lacunarity: f64,
    pub height_multiplier: f64,
    pub world_type: WorldType,
    pub terrain_amplitude: f64,
    pub cave_threshold: f64,
    pub flat_world_layers: Vec<(BlockId, i32)>,
}

impl Default for WorldGenConfig {
    fn default() -> Self {
        Self {
            world_seed: 0,
            terrain_height: 128,
            water_level: 62,
            biome_scale: 0.01,
            noise_scale: 0.01,
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
            height_multiplier: 1.0,
            world_type: WorldType::Normal,
            terrain_amplitude: 1.0,
            cave_threshold: 0.5,
            flat_world_layers: vec![
                (BlockId::new(1), 1), // Bedrock
                (BlockId::new(2), 3), // Stone
                (BlockId::new(3), 1), // Dirt
                (BlockId::new(4), 1), // Grass
            ],
        }
    }
}

pub struct TerrainGenerator {
    config: WorldGenConfig,
    block_registry: Arc<BlockRegistry>,
    noise: Perlin,
    rng: ChaCha12Rng,
}

impl TerrainGenerator {
    pub fn new(config: WorldGenConfig, block_registry: Arc<BlockRegistry>) -> Self {
        let noise = Perlin::new(config.world_seed as u32);
        let rng = ChaCha12Rng::seed_from_u64(config.world_seed);
        Self {
            config,
            block_registry,
            noise,
            rng,
        }
    }

    pub fn generate_chunk(&self, coord: ChunkCoord) -> Chunk {
        let mut chunk = Chunk::new(coord);
        let mut rng = rand::thread_rng();

        // Base terrain generation
        for x in 0..16 {
            for z in 0..16 {
                let height = self.get_height(coord.x * 16 + x, coord.z * 16 + z);
                for y in 0..16 {
                    let block_y = coord.y * 16 + y;
                    if block_y < height {
                        let block = if block_y < 5 {
                            Block::new(BlockId::new(1, 0, 0)) // Stone
                        } else if block_y < height - 1 {
                            Block::new(BlockId::new(2, 0, 0)) // Dirt
                        } else {
                            Block::new(BlockId::new(3, 0, 0)) // Grass
                        };
                        chunk.set_block(x, y, z, Some(block));
                    }
                }
            }
        }

        chunk
    }

    fn generate_normal_chunk(&self, chunk: &mut Chunk, coord: ChunkCoord) {
        let base_x = coord.x() * 32;
        let base_y = coord.y() * 32;
        let base_z = coord.z() * 32;

        for x in 0..32 {
            for y in 0..32 {
                for z in 0..32 {
                    let world_x = base_x + x as i32;
                    let world_y = base_y + y as i32;
                    let world_z = base_z + z as i32;

                    let height = self.get_height(world_x, world_z);
                    if world_y < height {
                        chunk.set_block(
                            x as u32,
                            y as u32,
                            z as u32,
                            Some(Block::new(BlockId::new(1))),
                        ); // Stone
                    } else if world_y == height {
                        chunk.set_block(
                            x as u32,
                            y as u32,
                            z as u32,
                            Some(Block::new(BlockId::new(2))),
                        ); // Grass
                    } else {
                        chunk.set_block(
                            x as u32,
                            y as u32,
                            z as u32,
                            Some(Block::new(BlockId::new(0))),
                        ); // Air
                    }
                }
            }
        }
    }

    fn generate_flat_chunk(&self, chunk: &mut Chunk, coord: ChunkCoord) {
        let base_y = coord.y() * 32;
        for x in 0..32 {
            for y in 0..32 {
                for z in 0..32 {
                    let world_y = base_y + y as i32;
                    if world_y < self.config.terrain_height {
                        chunk.set_block(
                            x as u32,
                            y as u32,
                            z as u32,
                            Some(Block::new(BlockId::new(1))),
                        ); // Stone
                    } else if world_y == self.config.terrain_height {
                        chunk.set_block(
                            x as u32,
                            y as u32,
                            z as u32,
                            Some(Block::new(BlockId::new(2))),
                        ); // Grass
                    } else {
                        chunk.set_block(
                            x as u32,
                            y as u32,
                            z as u32,
                            Some(Block::new(BlockId::new(0))),
                        ); // Air
                    }
                }
            }
        }
    }

    fn generate_superflat_chunk(&self, chunk: &mut Chunk, coord: ChunkCoord) {
        let base_y = coord.y() * 32;
        let mut current_height = 0;

        for (block_id, thickness) in &self.config.flat_world_layers {
            for y in 0..32 {
                let world_y = base_y + y as i32;
                if world_y >= current_height && world_y < current_height + thickness {
                    for x in 0..32 {
                        for z in 0..32 {
                            chunk.set_block(
                                x as u32,
                                y as u32,
                                z as u32,
                                Some(Block::new(*block_id)),
                            );
                        }
                    }
                }
            }
            current_height += thickness;
        }

        // Fill remaining space with air
        for x in 0..32 {
            for y in 0..32 {
                for z in 0..32 {
                    let world_y = base_y + y as i32;
                    if world_y >= current_height {
                        chunk.set_block(
                            x as u32,
                            y as u32,
                            z as u32,
                            Some(Block::new(BlockId::new(0))),
                        ); // Air
                    }
                }
            }
        }
    }

    fn get_height(&self, x: i32, z: i32) -> i32 {
        let mut amplitude = 1.0;
        let mut frequency = self.config.noise_scale;
        let mut height = 0.0;

        for _ in 0..self.config.octaves {
            let nx = x as f64 * frequency;
            let nz = z as f64 * frequency;
            height += self.noise.get([nx, nz]) * amplitude;
            amplitude *= self.config.persistence;
            frequency *= self.config.lacunarity;
        }

        (height * self.config.height_multiplier) as i32 + self.config.terrain_height
    }

    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        let chunk = self.generate_chunk(coord);
        Some(Arc::new(chunk))
    }

    fn calculate_height(&self, x: i32, z: i32, biome: BiomeType) -> i32 {
        let base_noise = self.sample_noise("terrain", x, z);
        let detail_noise = self.sample_noise("detail", x, z);
        let biome_mod = self.biome_height_modifier(biome);

        let height = match self.config.world_type {
            WorldType::Superflat => {
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
                if rng.gen_ratio(1, 10) {
                    block.place_sub_block(
                        rng.gen_range(0..SUB_RESOLUTION as u8),
                        rng.gen_range(0..SUB_RESOLUTION as u8),
                        rng.gen_range(0..SUB_RESOLUTION as u8),
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
                    rng.gen_range(0..SUB_RESOLUTION as u8),
                    rng.gen_range(0..SUB_RESOLUTION as u8),
                    rng.gen_range(0..SUB_RESOLUTION as u8),
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
        let noise = self.noise.get([
            x as f64 / self.config.noise_scale,
            z as f64 / self.config.noise_scale,
        ]);
        noise
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
}
