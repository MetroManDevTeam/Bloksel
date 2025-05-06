use crate::render::pipeline::ChunkRenderer;
use crate::ChunkCoord;
use crate::world::BlockOrientation;
use crate::world::block::Block;
use crate::world::block_id::BlockId;
use crate::world::block_mat::BlockMaterial;
use crate::world::block_visual::{BlockFacing, ConnectedDirections};
use crate::world::storage::core::{CompressedBlock, CompressedSubBlock};
use crate::world::{BlockRegistry, WorldConfig};
use bincode::{deserialize_from, serialize_into};
use gl::types::GLuint;
use glam::{IVec3, Vec3};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;
use std::sync::Arc;

pub const CHUNK_SIZE: u8 = 16;
pub const CHUNK_VOLUME: usize = (CHUNK_SIZE as usize).pow(3);

#[derive(Debug, Clone)]
pub struct ChunkMesh {
    pub vao: GLuint,
    pub vbo: GLuint,
    pub ebo: GLuint,
    pub index_count: i32,
    pub needs_upload: bool,
    pub vertex_data: Vec<f32>,
    pub index_data: Vec<u32>,
}

impl ChunkMesh {
    pub fn new() -> Self {
        Self {
            vao: 0,
            vbo: 0,
            ebo: 0,
            index_count: 0,
            needs_upload: false,
            vertex_data: Vec::new(),
            index_data: Vec::new(),
        }
    }
}

pub struct Chunk {
    pub coord: ChunkCoord,
    pub blocks: Vec<Option<Block>>,
    pub mesh: ChunkMesh,
}

impl Chunk {
    pub fn new(coord: ChunkCoord) -> Self {
        Self {
            coord,
            blocks: vec![None; CHUNK_VOLUME],
            mesh: ChunkMesh::new(),
        }
    }

    pub fn get_block(&self, x: u8, y: u8, z: u8) -> Option<&Block> {
        let index = (x as usize)
            + (y as usize) * CHUNK_SIZE as usize
            + (z as usize) * (CHUNK_SIZE as usize).pow(2);
        self.blocks[index].as_ref()
    }

    pub fn set_block(&mut self, x: u8, y: u8, z: u8, block: Option<Block>) {
        let index = (x as usize)
            + (y as usize) * CHUNK_SIZE as usize
            + (z as usize) * (CHUNK_SIZE as usize).pow(2);
        self.blocks[index] = block;
    }

    pub fn empty(size: u8) -> Self {
        Self {
            coord: ChunkCoord::new(0, 0, 0),
            blocks: vec![None; (size as usize).pow(3)],
            mesh: ChunkMesh::new(),
        }
    }

    pub fn from_template(template: &Self, coord: ChunkCoord) -> Self {
        Self {
            coord,
            blocks: template.blocks.clone(),
            mesh: ChunkMesh::new(),
        }
    }
}

pub struct ChunkManager {
    chunks: std::collections::HashMap<ChunkCoord, Arc<Chunk>>,
    renderer: ChunkRenderer,
    world_config: WorldConfig,
    compressed_cache: HashMap<ChunkCoord, Vec<CompressedBlock>>,
    block_registry: Arc<BlockRegistry>, // Added missing field
}

impl ChunkManager {
    pub fn new(
        world_config: WorldConfig,
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

        for x in 0..self.world_config.chunk_size {
            for y in 0..self.world_config.chunk_size {
                for z in 0..self.world_config.chunk_size {
                    if let Some(block) = chunk.blocks[x][y][z].as_ref() {
                        let mut sub_blocks = Vec::new();

                        for ((sx, sy, sz), sub) in &block.sub_blocks {
                            if sub.id != BlockId::AIR {
                                sub_blocks.push(CompressedSubBlock {
                                    local_pos: (*sx, *sy, *sz),
                                    id: sub.id,
                                    metadata: sub.metadata, // Added missing field
                                    orientation: sub.orientation,
                                });
                            }
                        }

                        compressed.push(CompressedBlock {
                            position: (x, y, z),
                            id: block.get_primary_id().base_id as u16, // Convert BlockId to u16
                            sub_blocks,
                        });
                    }
                }
            }
        }

        self.compressed_cache.insert(coord, compressed);
        self.chunks.insert(coord, Arc::new(chunk));
    }

    pub fn get_or_generate_chunk(&mut self, coord: ChunkCoord, seed: u32) -> &Chunk {
        if !self.chunks.contains_key(&coord) {
            let chunk = self.generate_chunk(coord);
            self.add_chunk(coord, chunk);
        }
        self.chunks.get(&coord).unwrap().as_ref().unwrap()
    }

    pub fn generate_chunk(&self, coord: ChunkCoord) -> Arc<Chunk> {
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
                        BlockId {
                            base_id: 1,
                            variation: 0,
                            color_id: 0,
                        },
                    ); // Stone block
                }
            }
        }

        Arc::new(chunk)
    }

    pub fn generate_merged_mesh(&self) -> ChunkMesh {
        let mut merged_mesh = ChunkMesh::new();
        let mut index_offset = 0;

        for (_coord, chunk) in &self.chunks {
            let mesh = self.renderer.generate_mesh(chunk);

            merged_mesh.vertex_data.extend(mesh.vertex_data.iter());

            for idx in mesh.index_data {
                merged_mesh.index_data.push(idx + index_offset);
            }

            index_offset += mesh.vertex_data.len() as u32 / 14;
        }

        merged_mesh
    }

    pub fn save_world(&self) -> std::io::Result<()> {
        let world_dir = format!("worlds/{}", self.world_config.world_name);
        fs::create_dir_all(&world_dir)?;

        let path = Path::new(&world_dir).join("world.dat");
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        bincode::serialize_into(&mut writer, &self.compressed_cache)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub fn load_world(&mut self) -> std::io::Result<()> {
        let path = format!("worlds/{}/world.dat", self.world_config.world_name);
        let file = File::open(path)?;
        let compressed_cache: HashMap<ChunkCoord, Vec<CompressedBlock>> =
            bincode::deserialize_from(file)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        for (coord, blocks) in compressed_cache {
            let mut chunk = Chunk::new(
                self.world_config.chunk_size,
                self.world_config.sub_resolution,
                coord,
            );

            for compressed in blocks {
                let (x, y, z) = compressed.position;
                let mut block = Block::new(
                    BlockId::new(compressed.id as u32),
                    self.world_config.sub_resolution as u8,
                );

                for sub in compressed.sub_blocks {
                    block.place_sub_block(
                        sub.local_pos.0,
                        sub.local_pos.1,
                        sub.local_pos.2,
                        SubBlock {
                            id: sub.id,
                            metadata: sub.metadata,
                            facing: BlockFacing::None, // Default facing
                            orientation: sub.orientation,
                            connections: ConnectedDirections::empty(), // Default connections
                        },
                    );
                }

                chunk.blocks[x][y][z] = Some(block);
            }

            self.chunks.insert(coord, Arc::new(chunk));
            self.compressed_cache.insert(coord, blocks);
        }

        Ok(())
    }

    pub fn get_block_at(&self, world_pos: Vec3) -> Option<(&Block, IVec3)> {
        let chunk_size = self.world_config.chunk_size as f32;
        let chunk_coord = ChunkCoord::from_world(world_pos, chunk_size);

        if let Some(chunk) = self.chunks.get(&chunk_coord) {
            let local_x = (world_pos.x % chunk_size).floor() as usize;
            let local_y = (world_pos.y % chunk_size).floor() as usize;
            let local_z = (world_pos.z % chunk_size).floor() as usize;

            chunk.blocks[local_x][local_y][local_z]
                .as_ref()
                .map(|block| (block, chunk_coord.into()))
        } else {
            None
        }
    }

    pub fn get_subblock_at(&self, world_pos: Vec3) -> Option<(&SubBlock, IVec3)> {
        let (block, chunk_coord) = self.get_block_at(world_pos)?;
        let sub_size = 1.0 / self.world_config.sub_resolution as f32;

        let local_pos = world_pos
            - Vec3::new(
                chunk_coord.x as f32 * self.world_config.chunk_size as f32,
                chunk_coord.y as f32 * self.world_config.chunk_size as f32,
                chunk_coord.z as f32 * self.world_config.chunk_size as f32,
            );

        let sx = (local_pos.x / sub_size).floor() as u8;
        let sy = (local_pos.y / sub_size).floor() as u8;
        let sz = (local_pos.z / sub_size).floor() as u8;

        block
            .sub_blocks
            .get(&(sx, sy, sz))
            .map(|sub| (sub, chunk_coord))
    }

    fn get_block_id_safe(&self, name: &str) -> BlockId {
        self.block_registry
            .get_by_name(name)
            .map(|def| def.id)
            .unwrap_or_else(|| {
                log::warn!("Block {} not found in registry", name);
                BlockId::AIR
            })
    }

    fn add_biome_features(&self, block: &mut Block, biome: BiomeType, rng: &mut ChaCha12Rng) {
        match biome {
            BiomeType::Forest => {
                if rng.gen_ratio(1, 10) {
                    self.add_grass_features(block, rng);
                }
            }
            BiomeType::Swamp => {
                if rng.gen_ratio(1, 8) {
                    self.add_swamp_features(block, rng);
                }
            }
            _ => {}
        }
    }
}
