use crate::config::WorldGenConfig;
use crate::render::pipeline::ChunkRenderer;
use crate::world::BlockOrientation;
use crate::world::BlockRegistry;
use crate::world::block::{Block, SubBlock};
use crate::world::block_facing::BlockFacing;
use crate::world::block_id::BlockId;
use crate::world::block_material::BlockMaterial;
use crate::world::block_visual::ConnectedDirections;
use crate::world::chunk_coord::ChunkCoord;
use crate::world::generator::terrain::BiomeType;
use crate::world::storage::core::{CompressedBlock, CompressedSubBlock};
use bincode::{deserialize_from, serialize_into};
use gl::types::GLuint;
use glam::{IVec3, Vec3};
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;
use std::sync::Arc;

pub const CHUNK_SIZE: u8 = 16;
pub const CHUNK_VOLUME: usize = (CHUNK_SIZE as usize).pow(3);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMesh {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
}

impl ChunkMesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub coord: ChunkCoord,
    pub blocks: Vec<Option<Block>>,
    pub mesh: Option<ChunkMesh>,
}

impl Chunk {
    pub fn new(coord: ChunkCoord) -> Self {
        Self {
            coord,
            blocks: vec![None; CHUNK_VOLUME],
            mesh: None,
        }
    }

    pub fn get_block(&self, x: u8, y: u8, z: u8) -> Option<&Block> {
        let index = (x as usize
            + y as usize * CHUNK_SIZE as usize
            + z as usize * CHUNK_SIZE as usize * CHUNK_SIZE as usize) as usize;
        self.blocks.get(index).and_then(|b| b.as_ref())
    }

    pub fn set_block(&mut self, x: u8, y: u8, z: u8, block: Block) {
        let index = (x as usize
            + y as usize * CHUNK_SIZE as usize
            + z as usize * CHUNK_SIZE as usize * CHUNK_SIZE as usize) as usize;
        if let Some(block_ref) = self.blocks.get_mut(index) {
            *block_ref = Some(block);
        }
    }

    pub fn from_template(template: &Chunk, coord: ChunkCoord) -> Self {
        let mut chunk = Self::new(coord);
        chunk.blocks = template.blocks.clone();
        chunk
    }

    pub fn empty() -> Self {
        Self {
            coord: ChunkCoord::new(0, 0, 0),
            blocks: vec![None; CHUNK_VOLUME],
            mesh: None,
        }
    }

    pub fn save_world(&self, world_dir: &Path) -> std::io::Result<()> {
        let chunk_file = world_dir.join(format!(
            "chunk_{}_{}_{}.bin",
            self.coord.x(),
            self.coord.y(),
            self.coord.z()
        ));
        let file = File::create(chunk_file)?;
        let writer = BufWriter::new(file);
        serialize_into(writer, self)?;
        Ok(())
    }

    pub fn load_world(world_dir: &Path, coord: ChunkCoord) -> std::io::Result<Self> {
        let chunk_file = world_dir.join(format!(
            "chunk_{}_{}_{}.bin",
            coord.x(),
            coord.y(),
            coord.z()
        ));
        let file = File::open(chunk_file)?;
        let chunk: Chunk = deserialize_from(file)?;
        Ok(chunk)
    }

    pub fn get_block_at(&self, world_pos: Vec3) -> Option<(&Block, IVec3)> {
        let local_pos = IVec3::new(
            (world_pos.x as i32).rem_euclid(CHUNK_SIZE as i32),
            (world_pos.y as i32).rem_euclid(CHUNK_SIZE as i32),
            (world_pos.z as i32).rem_euclid(CHUNK_SIZE as i32),
        );

        self.get_block(local_pos.x as u8, local_pos.y as u8, local_pos.z as u8)
            .map(|block| (block, local_pos))
    }

    pub fn get_subblock_at(&self, world_pos: Vec3) -> Option<(&SubBlock, IVec3)> {
        let local_pos = IVec3::new(
            (world_pos.x as i32).rem_euclid(CHUNK_SIZE as i32),
            (world_pos.y as i32).rem_euclid(CHUNK_SIZE as i32),
            (world_pos.z as i32).rem_euclid(CHUNK_SIZE as i32),
        );

        self.get_block(local_pos.x as u8, local_pos.y as u8, local_pos.z as u8)
            .and_then(|block| {
                block
                    .get_sub_block(
                        (local_pos.x as u8).rem_euclid(CHUNK_SIZE),
                        (local_pos.y as u8).rem_euclid(CHUNK_SIZE),
                        (local_pos.z as u8).rem_euclid(CHUNK_SIZE),
                    )
                    .map(|sub_block| (sub_block, local_pos))
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
                            if sub.id != BlockId::AIR {
                                sub_blocks.push(CompressedSubBlock {
                                    local_pos: (*sx, *sy, *sz),
                                    id: sub.id,
                                    metadata: sub.metadata,
                                    orientation: sub.orientation,
                                });
                            }
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
                        Block::new(BlockId::new(1)), // Stone block
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
        chunk.get_block_at(world_pos)
    }

    pub fn get_subblock_at(&self, world_pos: Vec3) -> Option<(&SubBlock, IVec3)> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos, CHUNK_SIZE as i32);
        let chunk = self.chunks.get(&chunk_coord)?;
        chunk.get_subblock_at(world_pos)
    }

    fn get_block_id_safe(&self, name: &str) -> BlockId {
        self.block_registry
            .get_block_id(name)
            .unwrap_or(BlockId::AIR)
    }

    fn add_biome_features(&self, block: &mut Block, biome: BiomeType, rng: &mut ChaCha12Rng) {
        // Add biome-specific features to the block
        match biome {
            BiomeType::Plains => {
                // Add grass and flowers
                if rng.gen_bool(0.1) {
                    block.place_sub_block(
                        0,
                        1,
                        0,
                        SubBlock {
                            id: self.get_block_id_safe("grass"),
                            metadata: 0,
                            facing: BlockFacing::PosY,
                            orientation: BlockOrientation::None,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
            BiomeType::Forest => {
                // Add trees and bushes
                if rng.gen_bool(0.05) {
                    block.place_sub_block(
                        0,
                        1,
                        0,
                        SubBlock {
                            id: self.get_block_id_safe("tree"),
                            metadata: 0,
                            facing: BlockFacing::PosY,
                            orientation: BlockOrientation::None,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
            BiomeType::Desert => {
                // Add cacti and dead bushes
                if rng.gen_bool(0.02) {
                    block.place_sub_block(
                        0,
                        1,
                        0,
                        SubBlock {
                            id: self.get_block_id_safe("cactus"),
                            metadata: 0,
                            facing: BlockFacing::PosY,
                            orientation: BlockOrientation::None,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
            BiomeType::Mountains => {
                // Add rocks and snow
                if rng.gen_bool(0.1) {
                    block.place_sub_block(
                        0,
                        1,
                        0,
                        SubBlock {
                            id: self.get_block_id_safe("rock"),
                            metadata: 0,
                            facing: BlockFacing::PosY,
                            orientation: BlockOrientation::None,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
            BiomeType::Ocean => {
                // Add coral and seaweed
                if rng.gen_bool(0.05) {
                    block.place_sub_block(
                        0,
                        1,
                        0,
                        SubBlock {
                            id: self.get_block_id_safe("coral"),
                            metadata: 0,
                            facing: BlockFacing::PosY,
                            orientation: BlockOrientation::None,
                            connections: ConnectedDirections::default(),
                        },
                    );
                }
            }
        }
    }
}
