use crate::config::WorldGenConfig;
use crate::render::pipeline::{ChunkRenderer, RenderError};
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
    pub vertices: Vec<f32>,        // 3D positions (x, y, z)
    pub normals: Vec<f32>,         // Normal vectors (nx, ny, nz)
    pub uvs: Vec<f32>,             // Texture coordinates (u, v)
    pub block_ids: Vec<u32>,       // Block type identifiers
    pub variant_data: Vec<u32>,    // Variant and connection data
    pub indices: Vec<u32>,         // Vertex indices
    pub vertex_count: usize,       // Total vertex count
    pub index_count: usize,        // Total index count
}

impl ChunkMesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            block_ids: Vec::new(),
            variant_data: Vec::new(),
            indices: Vec::new(),
            vertex_count: 0,
            index_count: 0,
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.normals.clear();
        self.uvs.clear();
        self.block_ids.clear();
        self.variant_data.clear();
        self.indices.clear();
        self.vertex_count = 0;
        self.index_count = 0;
    }

    pub fn add_face(
        &mut self,
        positions: &[Vec3],
        normal: Vec3,
        uvs: &[Vec2],
        block_id: u32,
        variant_data: u32,
    ) {
        let base_index = self.vertex_count as u32;
        
        // Add vertices
        for (pos, uv) in positions.iter().zip(uvs) {
            self.vertices.extend(&[pos.x, pos.y, pos.z]);
            self.normals.extend(&[normal.x, normal.y, normal.z]);
            self.uvs.extend(&[uv.x, uv.y]);
            self.block_ids.push(block_id);
            self.variant_data.push(variant_data);
            self.vertex_count += 1;
        }
        
        // Add indices (two triangles per quad face)
        self.indices.extend(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index + 2,
            base_index + 3,
            base_index,
        ]);
        self.index_count += 6;
    }

    pub fn is_empty(&self) -> bool {
        self.vertex_count == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub position: ChunkCoord,
    pub blocks: Vec<Option<Block>>,
    pub mesh: Option<ChunkMesh>,
    pub needs_remesh: bool,
}

impl Chunk {
    pub fn new(position: ChunkCoord) -> Self {
        Self {
            position,
            blocks: vec![None; CHUNK_VOLUME],
            mesh: None,
            needs_remesh: true,
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
        self.needs_remesh = true;
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
        bincode::deserialize_from(&mut reader)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    pub fn generate_mesh(&mut self, renderer: &ChunkRenderer) -> Result<(), RenderError> {
        if !self.needs_remesh {
            return Ok(());
        }

        let mut mesh = ChunkMesh::new();

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    if let Some(block) = self.get_block(x, y, z) {
                        self.generate_block_mesh(&mut mesh, block, x, y, z);
                    }
                }
            }
        }

        if mesh.is_empty() {
            self.mesh = None;
        } else {
            self.mesh = Some(mesh);
        }

        self.needs_remesh = false;
        Ok(())
    }

    fn generate_block_mesh(
        &mut self,
        mesh: &mut ChunkMesh,
        block: &Block,
        x: u32,
        y: u32,
        z: u32,
    ) {
        // Convert block coordinates to world space
        let world_pos = Vec3::new(
            x as f32 + self.position.x() as f32 * CHUNK_SIZE as f32,
            y as f32 + self.position.y() as f32 * CHUNK_SIZE as f32,
            z as f32 + self.position.z() as f32 * CHUNK_SIZE as f32,
        );

        // Generate mesh for each sub-block
        for ((sub_x, sub_y, sub_z), sub_block) in &block.sub_blocks {
            let sub_pos = world_pos + Vec3::new(*sub_x as f32, *sub_y as f32, *sub_z as f32);
            self.generate_subblock_mesh(mesh, sub_block, sub_pos);
        }
    }

    fn generate_subblock_mesh(
        &mut self,
        mesh: &mut ChunkMesh,
        sub_block: &SubBlock,
        position: Vec3,
    ) {
        // Generate faces based on block type and connections
        let block_id = sub_block.id.0;
        let variant_data = self.calculate_variant_data(sub_block);

        // Define cube vertices
        let vertices = [
            // Front face
            position + Vec3::new(0.0, 0.0, 1.0),
            position + Vec3::new(1.0, 0.0, 1.0),
            position + Vec3::new(1.0, 1.0, 1.0),
            position + Vec3::new(0.0, 1.0, 1.0),
            // Back face
            position + Vec3::new(1.0, 0.0, 0.0),
            position + Vec3::new(0.0, 0.0, 0.0),
            position + Vec3::new(0.0, 1.0, 0.0),
            position + Vec3::new(1.0, 1.0, 0.0),
            // Top face
            position + Vec3::new(0.0, 1.0, 1.0),
            position + Vec3::new(1.0, 1.0, 1.0),
            position + Vec3::new(1.0, 1.0, 0.0),
            position + Vec3::new(0.0, 1.0, 0.0),
            // Bottom face
            position + Vec3::new(0.0, 0.0, 0.0),
            position + Vec3::new(1.0, 0.0, 0.0),
            position + Vec3::new(1.0, 0.0, 1.0),
            position + Vec3::new(0.0, 0.0, 1.0),
            // Right face
            position + Vec3::new(1.0, 0.0, 1.0),
            position + Vec3::new(1.0, 0.0, 0.0),
            position + Vec3::new(1.0, 1.0, 0.0),
            position + Vec3::new(1.0, 1.0, 1.0),
            // Left face
            position + Vec3::new(0.0, 0.0, 0.0),
            position + Vec3::new(0.0, 0.0, 1.0),
            position + Vec3::new(0.0, 1.0, 1.0),
            position + Vec3::new(0.0, 1.0, 0.0),
        ];

        // Define face normals
        let normals = [
            Vec3::new(0.0, 0.0, 1.0),  // Front
            Vec3::new(0.0, 0.0, -1.0),  // Back
            Vec3::new(0.0, 1.0, 0.0),   // Top
            Vec3::new(0.0, -1.0, 0.0),  // Bottom
            Vec3::new(1.0, 0.0, 0.0),   // Right
            Vec3::new(-1.0, 0.0, 0.0),  // Left
        ];

        // Define UV coordinates (basic, can be adjusted per face)
        let uv_coords = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];

        // Add each face to the mesh if it's visible
        for face in 0..6 {
            if self.should_render_face(sub_block, face) {
                let face_vertices = &vertices[face * 4..face * 4 + 4];
                mesh.add_face(
                    face_vertices,
                    normals[face],
                    &uv_coords,
                    block_id,
                    variant_data,
                );
            }
        }
    }

    fn should_render_face(&self, sub_block: &SubBlock, face: usize) -> bool {
        // Check if face should be rendered based on connections or neighboring blocks
        // This is a simplified version - should be expanded based on your connection system
        true
    }

    fn calculate_variant_data(&self, sub_block: &SubBlock) -> u32 {
        // Pack variant and connection data into a single u32
        let variant = sub_block.id.1 as u32; // Variant ID
        let connections = sub_block.connections.bits() as u32;
        
        // Pack as: variant in upper 16 bits, connections in lower 16 bits
        (variant << 16) | connections
    }

    pub fn transform(&self) -> Mat4 {
        let pos = Vec3::new(
            self.position.x() as f32 * CHUNK_SIZE as f32,
            self.position.y() as f32 * CHUNK_SIZE as f32,
            self.position.z() as f32 * CHUNK_SIZE as f32,
        );
        Mat4::from_translation(pos)
    }

    pub fn is_solid_at(&self, world_x: i32, world_y: i32, world_z: i32) -> bool {
        self.get_block_at(world_x, world_y, world_z)
            .map_or(false, |block| block.is_solid())
    }
}

pub struct ChunkManager {
    chunks: HashMap<ChunkCoord, Arc<Chunk>>,
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
            chunks: HashMap::new(),
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

    pub fn update_meshes(&mut self) -> Result<(), RenderError> {
        for chunk in self.chunks.values_mut() {
            if let Some(chunk) = Arc::get_mut(chunk) {
                chunk.generate_mesh(&self.renderer)?;
            }
        }
        Ok(())
    }
    }
