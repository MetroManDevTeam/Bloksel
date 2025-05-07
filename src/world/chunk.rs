use crate::config::WorldGenConfig;
use crate::render::pipeline::ChunkRenderer;
use crate::world::BlockOrientation;
use crate::world::BlockRegistry;
use crate::world::block::{Block, SubBlock};
use crate::world::block_facing::BlockFacing;
use crate::world::block_id::BlockId;
use crate::world::block_material::BlockMaterial;
use crate::world::block_visual::ConnectedDirections;
use crate::world::blocks_data::get_block_registry;
use crate::world::chunk_coord::ChunkCoord;
use crate::world::generator::terrain::BiomeType;
use crate::world::storage::core::{CompressedBlock, CompressedSubBlock};
use bincode::{deserialize_from, serialize_into};
use gl::types::GLuint;
use glam::{IVec3, Mat4, Vec3};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;

pub const CHUNK_SIZE: u32 = 32;
pub const CHUNK_VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMesh {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
    #[serde(skip)]
    pub vao: GLuint,
    #[serde(skip)]
    pub vbo: GLuint,
    #[serde(skip)]
    pub ebo: GLuint,
}

impl ChunkMesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            vao: 0,
            vbo: 0,
            ebo: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub position: ChunkCoord,
    pub blocks: Vec<Option<Block>>,
    pub mesh: Option<ChunkMesh>,
}

impl Chunk {
    pub fn new(position: ChunkCoord) -> Self {
        Self {
            position,
            blocks: vec![None; CHUNK_VOLUME],
            mesh: None,
        }
    }

    pub fn empty() -> Self {
        Self::new(ChunkCoord::new(0, 0, 0))
    }

    pub fn get_block(&self, x: u32, y: u32, z: u32) -> Option<&Block> {
        let index = self.get_index(x, y, z);
        self.blocks[index].as_ref()
    }

    pub fn get_block_mut(&mut self, x: u32, y: u32, z: u32) -> Option<&mut Block> {
        let index = self.get_index(x, y, z);
        self.blocks[index].as_mut()
    }

    pub fn set_block(&mut self, x: u32, y: u32, z: u32, block: Option<Block>) {
        let index = self.get_index(x, y, z);
        self.blocks[index] = block;
    }

    fn get_index(&self, x: u32, y: u32, z: u32) -> usize {
        (x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE) as usize
    }

    pub fn get_block_at(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<&Block> {
        let local_x = (world_x.rem_euclid(CHUNK_SIZE as i32)) as u32;
        let local_y = (world_y.rem_euclid(CHUNK_SIZE as i32)) as u32;
        let local_z = (world_z.rem_euclid(CHUNK_SIZE as i32)) as u32;
        self.get_block(local_x, local_y, local_z)
    }

    pub fn get_subblock_at(
        &self,
        world_x: i32,
        world_y: i32,
        world_z: i32,
        sub_x: u8,
        sub_y: u8,
        sub_z: u8,
    ) -> Option<&SubBlock> {
        self.get_block_at(world_x, world_y, world_z)
            .and_then(|block| block.get_sub_block(&(sub_x, sub_y, sub_z)))
    }

    pub fn save_world(&self, world_dir: &Path) -> std::io::Result<()> {
        let chunk_file = world_dir.join(format!(
            "chunk_{}_{}_{}.bin",
            self.position.x(),
            self.position.y(),
            self.position.z()
        ));
        let file = File::create(chunk_file)?;
        self.save_to_writer(file)?;
        Ok(())
    }

    pub fn load_world(world_dir: &Path, coord: ChunkCoord) -> std::io::Result<Self> {
        let chunk_file = world_dir.join(format!(
            "chunk_{}_{}_{}.bin",
            coord.x(),
            coord.y(),
            coord.z()
        ));
        Self::load(&chunk_file)
    }

    pub fn save_to_writer(&self, mut writer: impl io::Write) -> io::Result<()> {
        bincode::serialize_into(&mut writer, self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::load_from_reader(reader)
    }

    pub fn load_from_reader(mut reader: impl io::Read) -> io::Result<Self> {
        bincode::deserialize_from(&mut reader).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    pub fn add_biome_features(&mut self, rng: &mut ChaCha12Rng, block_registry: &BlockRegistry) {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if let Some(block) = self.get_block_mut(x, y, z) {
                        if block.base_id() == 2 {
                            // Grass block
                            if rng.gen_bool(0.1) {
                                let sub_block = SubBlock::new(3) // Tall grass
                                    .with_facing(BlockFacing::PosZ)
                                    .with_orientation(BlockOrientation::North)
                                    .with_connections(ConnectedDirections::empty());
                                block.place_sub_block((x as u8, y as u8, z as u8), sub_block);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn generate_random(&mut self, probability: f64) {
        let mut rng = ChaCha12Rng::from_entropy();
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    if rng.gen_bool(probability) {
                        let block = Block::new(BlockId::new(1, 0, 0));
                        self.set_block(x, y, z, Some(block));
                    }
                }
            }
        }
    }

    pub fn transform(&self) -> Mat4 {
        let pos = Vec3::new(
            self.position.x() as f32 * CHUNK_SIZE as f32,
            self.position.y() as f32 * CHUNK_SIZE as f32,
            self.position.z() as f32 * CHUNK_SIZE as f32,
        );
        Mat4::from_translation(pos)
    }

    pub fn get_block_id_safe(&self, name: &str) -> BlockId {
        BlockId(
            get_block_registry()
                .borrow()
                .get_by_name(name)
                .map(|def| def.id.0)
                .unwrap_or(10),
        )
    }

    pub fn create_grass_block(&mut self) -> Block {
        let mut block = Block::new(self.get_block_id_safe("grass"));
        block.place_sub_block(
            (0, 1, 0),
            SubBlock {
                id: self.get_block_id_safe("grass").into(),
                facing: BlockFacing::PosZ,
                orientation: BlockOrientation::North,
                connections: ConnectedDirections::default(),
            },
        );
        block
    }

    pub fn create_tree_block(&mut self) -> Block {
        let mut block = Block::new(self.get_block_id_safe("tree"));
        block.place_sub_block(
            (0, 1, 0),
            SubBlock {
                id: self.get_block_id_safe("tree").into(),
                facing: BlockFacing::PosZ,
                orientation: BlockOrientation::North,
                connections: ConnectedDirections::default(),
            },
        );
        block
    }

    pub fn create_cactus_block(&mut self) -> Block {
        let mut block = Block::new(self.get_block_id_safe("cactus"));
        block.place_sub_block(
            (0, 1, 0),
            SubBlock {
                id: self.get_block_id_safe("cactus").into(),
                facing: BlockFacing::PosZ,
                orientation: BlockOrientation::North,
                connections: ConnectedDirections::default(),
            },
        );
        block
    }

    pub fn create_rock_block(&mut self) -> Block {
        let mut block = Block::new(self.get_block_id_safe("rock"));
        block.place_sub_block(
            (0, 1, 0),
            SubBlock {
                id: self.get_block_id_safe("rock").into(),
                facing: BlockFacing::PosZ,
                orientation: BlockOrientation::North,
                connections: ConnectedDirections::default(),
            },
        );
        block
    }

    pub fn create_coral_block(&mut self) -> Block {
        let mut block = Block::new(self.get_block_id_safe("coral"));
        block.place_sub_block(
            (0, 1, 0),
            SubBlock {
                id: self.get_block_id_safe("coral").into(),
                facing: BlockFacing::PosZ,
                orientation: BlockOrientation::North,
                connections: ConnectedDirections::default(),
            },
        );
        block
    }

    pub fn is_solid_at(&self, world_x: i32, world_y: i32, world_z: i32) -> bool {
        self.get_block_at(world_x, world_y, world_z)
            .map_or(false, |block| block.is_solid())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedChunk {
    pub coord: ChunkCoord,
    pub blocks: Vec<Option<Block>>,
}

impl SerializedChunk {
    pub fn from_chunk(coord: ChunkCoord, chunk: &Chunk) -> Self {
        Self {
            coord,
            blocks: chunk.blocks.clone(),
        }
    }
}

impl Chunk {
    pub fn from_serialized(serialized: SerializedChunk) -> Result<Self, std::io::Error> {
        Ok(Self {
            position: serialized.coord,
            blocks: serialized.blocks,
            mesh: None,
        })
    }
}

pub struct ChunkManager {
    chunks: std::collections::HashMap<ChunkCoord, Arc<Chunk>>,
    renderer: ChunkRenderer,
    world_config: WorldGenConfig,
    compressed_cache: HashMap<ChunkCoord, Vec<CompressedBlock>>,
    block_registry: Arc<BlockRegistry>,
}

impl ChunkManager {
    pub fn new(
        world_config: WorldGenConfig,
        renderer: ChunkRenderer,
        block_registry: Arc<BlockRegistry>,
    ) -> Self {
        Self {
            chunks: std::collections::HashMap::new(),
            renderer,
            world_config,
            compressed_cache: HashMap::new(),
            block_registry,
        }
    }

    pub fn add_chunk(&mut self, coord: ChunkCoord, chunk: Chunk) {
        let mut compressed = Vec::new();

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    if let Some(block) = chunk.get_block(x, y, z) {
                        let mut sub_blocks = Vec::new();

                        for ((sx, sy, sz), sub) in &block.sub_blocks {
                            sub_blocks.push(CompressedSubBlock {
                                local_pos: (*sx, *sy, *sz),
                                id: sub.id,
                                facing: sub.facing,
                                orientation: sub.orientation,
                                connections: sub.connections,
                            });
                        }

                        compressed.push(CompressedBlock {
                            position: (x as usize, y as usize, z as usize),
                            id: block.id,
                            sub_blocks,
                        });
                    }
                }
            }
        }

        self.compressed_cache.insert(coord, compressed);
        self.chunks.insert(coord, Arc::new(chunk));
    }

    pub fn get_or_generate_chunk(&mut self, coord: ChunkCoord, _seed: u32) -> &Chunk {
        if !self.chunks.contains_key(&coord) {
            let chunk = self.generate_chunk(coord);
            self.add_chunk(coord, chunk);
        }
        self.chunks.get(&coord).unwrap().as_ref()
    }

    pub fn generate_chunk(&self, coord: ChunkCoord) -> Chunk {
        let mut chunk = Chunk::new(coord);

        // Simple terrain generation
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let height = 64; // Simple flat terrain
                for y in 0..height {
                    chunk.set_block(
                        x,
                        y,
                        z,
                        Some(Block::new(BlockId::new(1, 0, 0))), // Stone block
                    );
                }
            }
        }

        chunk
    }

    pub fn generate_merged_mesh(&self) -> ChunkMesh {
        let mut merged_mesh = ChunkMesh::new();
        let mut index_offset = 0;

        for (_coord, chunk) in &self.chunks {
            let mesh = self.renderer.generate_mesh(chunk);

            merged_mesh.vertices.extend(mesh.vertices.iter());

            for idx in mesh.indices {
                merged_mesh.indices.push(idx + index_offset);
            }

            index_offset += mesh.vertices.len() as u32 / 14;
        }

        merged_mesh
    }

    pub fn save_world(&self) -> std::io::Result<()> {
        let world_dir = format!("worlds/{}", self.world_config.world_name);
        fs::create_dir_all(&world_dir)?;

        for (coord, chunk) in &self.chunks {
            chunk.save_world(Path::new(&world_dir))?;
        }

        Ok(())
    }

    pub fn load_world(&mut self) -> std::io::Result<()> {
        let world_dir = format!("worlds/{}", self.world_config.world_name);
        let world_path = Path::new(&world_dir);

        if !world_path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(world_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "bin") {
                let coord = ChunkCoord::from_path(&path)?;
                let chunk = Chunk::load_world(world_path, coord)?;
                self.add_chunk(coord, chunk);
            }
        }

        Ok(())
    }

    pub fn get_block_at(&self, world_pos: Vec3) -> Option<(&Block, IVec3)> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos, CHUNK_SIZE as i32);
        let chunk = self.chunks.get(&chunk_coord)?;
        chunk
            .get_block_at(world_pos.x as i32, world_pos.y as i32, world_pos.z as i32)
            .map(|block| {
                (
                    block,
                    IVec3::new(
                        (world_pos.x as i32).rem_euclid(CHUNK_SIZE as i32),
                        (world_pos.y as i32).rem_euclid(CHUNK_SIZE as i32),
                        (world_pos.z as i32).rem_euclid(CHUNK_SIZE as i32),
                    ),
                )
            })
    }

    pub fn get_subblock_at(&self, world_pos: Vec3) -> Option<(&SubBlock, IVec3)> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos, CHUNK_SIZE as i32);
        let chunk = self.chunks.get(&chunk_coord)?;
        chunk
            .get_subblock_at(
                world_pos.x as i32,
                world_pos.y as i32,
                world_pos.z as i32,
                (world_pos.x as i32).rem_euclid(CHUNK_SIZE as i32) as u8,
                (world_pos.y as i32).rem_euclid(CHUNK_SIZE as i32) as u8,
                (world_pos.z as i32).rem_euclid(CHUNK_SIZE as i32) as u8,
            )
            .map(|sub_block| {
                (
                    sub_block,
                    IVec3::new(
                        (world_pos.x as i32).rem_euclid(CHUNK_SIZE as i32),
                        (world_pos.y as i32).rem_euclid(CHUNK_SIZE as i32),
                        (world_pos.z as i32).rem_euclid(CHUNK_SIZE as i32),
                    ),
                )
            })
    }

    fn get_block_id_safe(&self, name: &str) -> BlockId {
        BlockId(
            self.block_registry
                .get_by_name(name)
                .map(|def| def.0)
                .unwrap_or(10),
        )
    }

    fn add_biome_features(&self, block: &mut Block, biome: BiomeType, rng: &mut ChaCha12Rng) {
        // Add biome-specific features to the block
        match biome {
            BiomeType::Plains => {
                // Add grass and flowers
                if rng.gen_bool(0.1) {
                    block.place_sub_block(
                        (0, 1, 0),
                        SubBlock {
                            id: self.get_block_id_safe("grass").into(),
                            facing: BlockFacing::PosZ,
                            orientation: BlockOrientation::North,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
            BiomeType::Forest => {
                // Add trees and bushes
                if rng.gen_bool(0.05) {
                    block.place_sub_block(
                        (0, 1, 0),
                        SubBlock {
                            id: self.get_block_id_safe("tree").into(),
                            facing: BlockFacing::PosZ,
                            orientation: BlockOrientation::North,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
            BiomeType::Desert => {
                // Add cacti and dead bushes
                if rng.gen_bool(0.02) {
                    block.place_sub_block(
                        (0, 1, 0),
                        SubBlock {
                            id: self.get_block_id_safe("cactus").into(),
                            facing: BlockFacing::PosZ,
                            orientation: BlockOrientation::North,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
            BiomeType::Mountains => {
                // Add rocks and snow
                if rng.gen_bool(0.1) {
                    block.place_sub_block(
                        (0, 1, 0),
                        SubBlock {
                            id: self.get_block_id_safe("rock").into(),
                            facing: BlockFacing::PosZ,
                            orientation: BlockOrientation::North,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
            BiomeType::Ocean => {
                // Add coral and seaweed
                if rng.gen_bool(0.05) {
                    block.place_sub_block(
                        (0, 1, 0),
                        SubBlock {
                            id: self.get_block_id_safe("coral").into(),
                            facing: BlockFacing::PosZ,
                            orientation: BlockOrientation::North,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
            BiomeType::Tundra => {
                // Add snow and ice features
                if rng.gen_bool(0.1) {
                    block.place_sub_block(
                        (0, 1, 0),
                        SubBlock {
                            id: self.get_block_id_safe("snow").into(),
                            facing: BlockFacing::PosZ,
                            orientation: BlockOrientation::North,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
            BiomeType::Swamp => {
                // Add lily pads and reeds
                if rng.gen_bool(0.08) {
                    block.place_sub_block(
                        (0, 1, 0),
                        SubBlock {
                            id: self.get_block_id_safe("lily_pad").into(),
                            facing: BlockFacing::PosZ,
                            orientation: BlockOrientation::North,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
        }
    }
}
