use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use glam::{IVec3, Vec3};
use noise::{NoiseFn, Perlin, Seedable};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha12Rng;
use serde::{Serialize, Deserialize};
use noise::Fbm;


use crate::block::{Block, BlockId, BlockRegistry, BlockPhysics, SubBlock, BlockFacing, BlockOrientation };
use crate::chunk::{terrain_generator::Chunk,  terrain_generator::ChunkCoord};

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

pub struct Chunk {
    pub blocks: Vec<Vec<Vec<Option<Block>>>>,
    pub sub_resolution: usize, 
    pub coord: ChunkCoord,
    pub chunk_size: usize,
    pub mesh: ChunkMesh,
}

#[derive(Default)]
pub struct ChunkMesh {
    pub vertex_data: Vec<f32>,
    pub index_data: Vec<u32>,
    pub vao: u32,
    pub vbo: u32,
    pub ebo: u32,
    pub index_count: i32,
    pub needs_upload: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn from_world(pos: Vec3, chunk_size: f32) -> Self {
        Self {
            x: (pos.x / chunk_size).floor() as i32,
            y: (pos.y / chunk_size).floor() as i32,
            z: (pos.z / chunk_size).floor() as i32,
        }
    }
}

impl ChunkMesh {
    pub fn new() -> Self {
        Self {
            vertex_data: Vec::new(),
            index_data: Vec::new(),
            vao: 0,
            vbo: 0,
            ebo: 0,
            index_count: 0,
            needs_upload: true,
        }
    }
}

impl Chunk {
        
    pub fn transform_matrix(&self) -> glam::Mat4 {
        let chunk_size = self.blocks.len() as f32;
        glam::Mat4::from_translation(glam::Vec3::new(
            self.coord.x as f32 * chunk_size,
            self.coord.y as f32 * chunk_size,
            self.coord.z as f32 * chunk_size
        ))
    }

 pub fn new(size: usize, sub_resolution: usize, coord: ChunkCoord) -> Self {
    let mut blocks = vec![vec![vec![None; size]; size]; size]; // Fixed 3D initialization
    Chunk {
        blocks,
        sub_resolution,
        coord,         
        chunk_size: size,
        mesh: ChunkMesh::new(),  
    }
  }

      pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: Option<Block>) {
        if x < self.blocks.len() 
            && y < self.blocks[0].len() 
            && z < self.blocks[0][0].len() 
        {
            self.blocks[x][y][z] = block;
        } else {
            log::warn!("Attempted to set block at out-of-bounds position ({}, {}, {})", x, y, z);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub id: BlockId,
    pub grid: HashMap<(u8, u8, u8), SubBlock>,
    pub physics: BlockPhysics,
    pub integrity: f32,
    pub orientation: Orientation,
    pub metadata: u32,
    pub temperature: f32,
    pub custom_data: Option<Vec<u8>>,
}

impl Default for BlockData {
    fn default() -> Self {
        Self {
            id: BlockId::AIR,
            grid: HashMap::new(),
            physics: BlockPhysics::default(),
            integrity: 1.0,
            orientation: Orientation::default(),
            metadata: 0,
            temperature: 293.15, 
            custom_data: None,
        }
    }
}

impl BlockData {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn with_physics(id: BlockId, physics: BlockPhysics) -> Self {
        Self {
            id,
            physics,
            ..Default::default()
        }
    }

    pub fn is_air(&self) -> bool {
        self.id == BlockId::AIR
    }

    pub fn is_solid(&self) -> bool {
        !self.physics.passable
    }

    pub fn is_liquid(&self) -> bool {
        self.physics.passable && self.physics.dynamic
    }

    pub fn is_opaque(&self) -> bool {
        !self.is_air() && !self.is_liquid()
    }

    pub fn light_level(&self) -> u8 {
        self.physics.light_level
    }

    pub fn rotate(&mut self, new_orientation: Orientation) {
        self.orientation = new_orientation;
    }

    pub fn damage(&mut self, amount: f32) {
        self.integrity = (self.integrity - amount).max(0.0);
    }

    pub fn heal(&mut self, amount: f32) {
        self.integrity = (self.integrity + amount).min(1.0);
    }
}

#[derive(Serialize, Deserialize)]
pub struct TerrainConfig {
    pub seed: u32,
    pub world_scale: f64,
    pub terrain_amplitude: f64,
    pub cave_threshold: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Orientation {
    North,
    South,
    East,
    West,
    Up,
    Down,
    Custom(f32, f32, f32, f32),
    None,
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::North
    }
}

impl Orientation {
    pub fn to_matrix(&self) -> glam::Mat4 {
        match self {
            Orientation::North => glam::Mat4::IDENTITY,
            Orientation::South => glam::Mat4::from_rotation_y(std::f32::consts::PI),
            Orientation::East => glam::Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Orientation::West => glam::Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_2),
            Orientation::Up => glam::Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            Orientation::Down => glam::Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2),
            Orientation::Custom(x, y, z, w) => glam::Mat4::from_quat(glam::Quat::from_xyzw(*x, *y, *z, *w)),
            Orientation::None => glam::Mat4::IDENTITY,
        }
    }

    pub fn facing(&self) -> glam::Vec3 {
        match self {
            Orientation::North => glam::Vec3::NEG_Z,
            Orientation::South => glam::Vec3::Z,
            Orientation::East => glam::Vec3::X,
            Orientation::West => glam::Vec3::NEG_X,
            Orientation::Up => glam::Vec3::Y,
            Orientation::Down => glam::Vec3::NEG_Y,
            Orientation::Custom(x, y, z, w) => {
                let quat = glam::Quat::from_xyzw(*x, *y, *z, *w);
                quat.mul_vec3(glam::Vec3::NEG_Z)
            }
            Orientation::None => glam::Vec3::ZERO,
        }
    }
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
    fn initialize_noise_layers(&mut self) {  // Changed to &mut self
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
    octaves: usize
) -> Fbm<Perlin> {
    let mut fbm = Fbm::<Perlin>::new(self.config.seed + seed_offset);
    fbm.set_octaves(octaves)
        .set_frequency(frequency)
        .set_persistence(persistence)
        .set_lacunarity(2.0);
    fbm
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
            // Add default case for any new biomes
            _ => 0.0,
        }
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
                        block_id = self.block_registry.get_by_name("water").map(|def| def.id) .unwrap_or(BlockId::from(10));
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
                self.block_registry.get_by_name("dirt").map(|def| def.id) .unwrap_or(BlockId::from(10)),
                self.block_registry.get_by_name("grass").map(|def| def.id) .unwrap_or(BlockId::from(10)),
            ),
            BiomeType::Mountains | BiomeType::Tundra => (
                self.block_registry.get_by_name("stone").map(|def| def.id) .unwrap_or(BlockId::from(10)),
                self.block_registry.get_by_name("snow").map(|def| def.id) .unwrap_or(BlockId::from(10)),
            ),
            BiomeType::Desert => (
                self.block_registry.get_by_name("sand").map(|def| def.id) .unwrap_or(BlockId::from(10)),
                self.block_registry.get_by_name("sand").map(|def| def.id) .unwrap_or(BlockId::from(10)),
            ),
            BiomeType::Forest => (
                self.block_registry.get_by_name("dirt").map(|def| def.id) .unwrap_or(BlockId::from(10)),
                self.block_registry.get_by_name("grass").map(|def| def.id) .unwrap_or(BlockId::from(10)),
            ),
            BiomeType::Ocean => (
                self.block_registry.get_by_name("sand").map(|def| def.id) .unwrap_or(BlockId::from(10)),
                self.block_registry.get_by_name("gravel").map(|def| def.id) .unwrap_or(BlockId::from(10)),
            ),
        }
    }

    fn get_block_for_depth(&self, y: i32, height: i32, base: BlockId, top: BlockId, biome: BiomeType) -> BlockId {
        match biome {
            BiomeType::Ocean if y <= SEA_LEVEL - 8 => 
                self.block_registry.get_by_name("stone").map(|def| def.id) .unwrap_or(BlockId::from(10)),
            _ if y == height => top,
            _ if y > height - 4 => base,
            _ => self.block_registry.get_by_name("stone").map(|def| def.id) .unwrap_or(BlockId::from(10)),
        }
    }

    fn create_block(&self, id: BlockId, biome: BiomeType, rng: &mut ChaCha12Rng) -> Block {
        let mut block = Block::new(id, SUB_RESOLUTION as u8);

        // Add biome-specific features
        match biome {
            BiomeType::Forest if id == self.block_registry.get_by_name("grass").map(|def| def.id) .unwrap_or(BlockId::from(10)) => {
                if rng.gen_ratio(1, 10) {
                  block.place_sub_block(
    rng.gen_range(0..SUB_RESOLUTION as u8),
    rng.gen_range(0..SUB_RESOLUTION as u8),
    rng.gen_range(0..SUB_RESOLUTION as u8),
    SubBlock {
        id: self.block_registry.get_by_name("grass").map(|def| def.id) .unwrap_or(BlockId::from(10)),
        metadata: 0, // Set appropriate metadata if needed
        facing: BlockFacing::None, // Set appropriate facing if needed
        orientation: BlockOrientation::Wall, // Set appropriate orientation if needed
    }
);
                }
            },
            BiomeType::Swamp if id == self.block_registry.get_by_name("water").map(|def| def.id) .unwrap_or(BlockId::from(10)) => {
               block.place_sub_block(
    rng.gen_range(0..SUB_RESOLUTION as u8),
    rng.gen_range(0..SUB_RESOLUTION as u8),
    rng.gen_range(0..SUB_RESOLUTION as u8),
    SubBlock {
        id: self.block_registry.get_by_name("water").map(|def| def.id) .unwrap_or(BlockId::from(10)),
        metadata: 0, // Set appropriate metadata if needed
        facing: BlockFacing::None, // Set appropriate facing if needed
        orientation: BlockOrientation::Wall, // Set appropriate orientation if needed
    }
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
                SubBlock {
                    id: self.block_registry.get_by_name(ore_type).map(|def| def.id).unwrap_or(BlockId::from(10)),
                    metadata: 0,
                    facing: BlockFacing::None,
                    orientation: BlockOrientation::Wall,
                    ..SubBlock::default()
                }
            );
        }
    }
}
    
    fn should_add_cave(&self, x: i32, y: i32, z: i32) -> bool {
    // Only generate caves below surface level
    if y > SEA_LEVEL + 10 {
        return false;
    }
    
    let cave_noise = self.sample_noise("caves", x, z);
    let vertical_factor = 1.0 - (y as f64 / 128.0).abs().powi(2);
    let threshold = self.config.cave_threshold * vertical_factor;
    
    cave_noise.abs() > threshold
}

    fn sample_noise(&self, layer: &str, x: i32, z: i32) -> f64 {
        let layers = self.noise_layers.read();
        let noise = layers.get(layer).unwrap();
        noise.get([x as f64 * self.config.world_scale, z as f64 * self.config.world_scale])
    }

    pub fn generate_tree(&self, base_pos: IVec3) -> HashMap<IVec3, Block> {
    let mut blocks = HashMap::new();
    let trunk_id = self.get_block_id_safe("log");
    let leaves_id = self.get_block_id_safe("leaves");
    let height = 4 + (base_pos.x.abs() % 3) as i32;

    // Generate trunk
    for y in 0..height {
        let pos = IVec3::new(base_pos.x, base_pos.y + y, base_pos.z);
        blocks.insert(pos, Block::new(trunk_id, SUB_RESOLUTION as u8));
    }

    // Generate leaves
    let center = IVec3::new(base_pos.x, base_pos.y + height - 2, base_pos.z);
    for dx in -2..=2 {
        for dz in -2..=2 {
            for dy in -1..=1 {
                if dx*dx + dz*dz + dy*dy <= 4 {
                    let pos = center + IVec3::new(dx, dy, dz);
                    blocks.insert(pos, Block::new(leaves_id, SUB_RESOLUTION as u8));
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
    
    let coord = ChunkCoord::new(0, 0, 0); // Added this line
    let chunk = generator.generate_chunk(coord);
    assert!(chunk.blocks.iter().flatten().flatten().any(|b| b.is_some()));
}
}
