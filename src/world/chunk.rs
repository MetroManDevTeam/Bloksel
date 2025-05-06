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
                            id: block.id.base_id,
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

        let path = Path::new(&world_dir).join("world.dat");
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        for (coord, chunk) in &self.chunks {
            serialize_into(&mut writer, &(coord, chunk))?;
        }

        Ok(())
    }

    pub fn load_world(&mut self) -> std::io::Result<()> {
        let world_dir = format!("worlds/{}", self.world_config.world_name);
        let path = Path::new(&world_dir).join("world.dat");

        if !path.exists() {
            return Ok(());
        }

        let file = File::open(path)?;
        let mut reader = std::io::BufReader::new(file);

        while let Ok((coord, chunk)) = deserialize_from(&mut reader) {
            self.add_chunk(coord, chunk);
        }

        Ok(())
    }

    pub fn get_block_at(&self, world_pos: Vec3) -> Option<(&Block, IVec3)> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos, CHUNK_SIZE as u32);
        let chunk = self.chunks.get(&chunk_coord)?;

        let local_x = (world_pos.x as i32 % CHUNK_SIZE as i32) as u8;
        let local_y = (world_pos.y as i32 % CHUNK_SIZE as i32) as u8;
        let local_z = (world_pos.z as i32 % CHUNK_SIZE as i32) as u8;

        chunk.get_block(local_x, local_y, local_z).map(|block| {
            (
                block,
                IVec3::new(
                    chunk_coord.x() * CHUNK_SIZE as i32 + local_x as i32,
                    chunk_coord.y() * CHUNK_SIZE as i32 + local_y as i32,
                    chunk_coord.z() * CHUNK_SIZE as i32 + local_z as i32,
                ),
            )
        })
    }

    pub fn get_subblock_at(&self, world_pos: Vec3) -> Option<(&SubBlock, IVec3)> {
        let (block, block_pos) = self.get_block_at(world_pos)?;

        let local_x = (world_pos.x as i32 % CHUNK_SIZE as i32) as u8;
        let local_y = (world_pos.y as i32 % CHUNK_SIZE as i32) as u8;
        let local_z = (world_pos.z as i32 % CHUNK_SIZE as i32) as u8;

        block.get_sub_block(local_x, local_y, local_z).map(|sub| {
            (
                sub,
                IVec3::new(
                    block_pos.x * CHUNK_SIZE as i32 + local_x as i32,
                    block_pos.y * CHUNK_SIZE as i32 + local_y as i32,
                    block_pos.z * CHUNK_SIZE as i32 + local_z as i32,
                ),
            )
        })
    }

    fn get_block_id_safe(&self, name: &str) -> BlockId {
        self.block_registry
            .get_by_name(name)
            .map(|def| def.id)
            .unwrap_or(BlockId::AIR)
    }

    fn add_biome_features(&self, block: &mut Block, biome: BiomeType, rng: &mut ChaCha12Rng) {
        match biome {
            BiomeType::Grassland => {
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
