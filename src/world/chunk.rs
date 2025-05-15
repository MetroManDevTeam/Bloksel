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
use glam::{IVec3, Mat4, Vec2, Vec3, Vec4};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;
use ash::vk;
use crate::render::core::Camera;

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

#[derive(Debug, Clone)]
pub struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    pub fn from_view_projection(view_proj: &Mat4) -> Self {
        let mut planes = [Vec4::ZERO; 6];
        let m = view_proj.to_cols_array_2d();

        // Extract frustum planes from view-projection matrix
        planes[0] = Vec4::new(m[0][3] + m[0][0], m[1][3] + m[1][0], m[2][3] + m[2][0], m[3][3] + m[3][0]); // Left
        planes[1] = Vec4::new(m[0][3] - m[0][0], m[1][3] - m[1][0], m[2][3] - m[2][0], m[3][3] - m[3][0]); // Right
        planes[2] = Vec4::new(m[0][3] + m[0][1], m[1][3] + m[1][1], m[2][3] + m[2][1], m[3][3] + m[3][1]); // Bottom
        planes[3] = Vec4::new(m[0][3] - m[0][1], m[1][3] - m[1][1], m[2][3] - m[2][1], m[3][3] - m[3][1]); // Top
        planes[4] = Vec4::new(m[0][3] + m[0][2], m[1][3] + m[1][2], m[2][3] + m[2][2], m[3][3] + m[3][2]); // Near
        planes[5] = Vec4::new(m[0][3] - m[0][2], m[1][3] - m[1][2], m[2][3] - m[2][2], m[3][3] - m[3][2]); // Far

        // Normalize planes
        for plane in &mut planes {
            let length = Vec3::new(plane.x, plane.y, plane.z).length();
            *plane = *plane / length;
        }

        Self { planes }
    }

    pub fn intersects_aabb(&mut self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            let p = Vec3::new(plane.x, plane.y, plane.z);
            let d = plane.w;

            // Find the farthest point in the negative direction of the plane normal
            let mut farthest = min;
            if p.x > 0.0 { farthest.x = max.x; }
            if p.y > 0.0 { farthest.y = max.y; }
            if p.z > 0.0 { farthest.z = max.z; }

            // If the farthest point is outside the plane, the AABB is outside the frustum
            if p.dot(farthest) + d < 0.0 {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub position: ChunkCoord,
    pub blocks: Vec<Option<Block>>,
    pub mesh: Option<ChunkMesh>,
    pub needs_remesh: bool,
    #[serde(skip)]
    pub bounds: (Vec3, Vec3), // (min, max) world-space AABB
}

impl Chunk {
    pub fn new(position: ChunkCoord) -> Self {
        let min = Vec3::new(
            position.x() as f32 * CHUNK_SIZE as f32,
            position.y() as f32 * CHUNK_SIZE as f32,
            position.z() as f32 * CHUNK_SIZE as f32,
        );
        let max = min + Vec3::new(CHUNK_SIZE as f32, CHUNK_SIZE as f32, CHUNK_SIZE as f32);

        Self {
            position,
            blocks: vec![None; CHUNK_VOLUME],
            mesh: None,
            needs_remesh: true,
            bounds: (min, max),
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

    fn get_index(&mut self, x: u32, y: u32, z: u32) -> usize {
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

    pub fn save_to_writer(&mut self, mut writer: impl io::Write) -> io::Result<()> {
        bincode::serialize_into(&mut writer, self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::load_from_reader(reader)
    }

    pub fn load_from_reader(mut reader: impl io::Read) -> io::Result<Self> {
        let mut chunk: Self = bincode::deserialize_from(&mut reader)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        
        // Recalculate bounds after loading
        let min = Vec3::new(
            chunk.position.x() as f32 * CHUNK_SIZE as f32,
            chunk.position.y() as f32 * CHUNK_SIZE as f32,
            chunk.position.z() as f32 * CHUNK_SIZE as f32,
        );
        chunk.bounds = (min, min + Vec3::new(CHUNK_SIZE as f32, CHUNK_SIZE as f32, CHUNK_SIZE as f32));
        
        Ok(chunk)
    }


    pub fn generate_mesh(&mut self, renderer: &ChunkRenderer) -> Result<(), RenderError> {
    if !self.needs_remesh {
        return Ok(());
    }

    let mut mesh = ChunkMesh::new();

    // Collect block data first to avoid borrow conflicts
    let blocks: Vec<_> = (0..CHUNK_SIZE)
        .flat_map(|x| (0..CHUNK_SIZE)
            .flat_map(move |y| (0..CHUNK_SIZE)
                .map(move |z| (x, y, z, self.get_block(x, y, z).cloned()))))
        .collect();

    for (x, y, z, block) in blocks {
        if let Some(block) = block {
            self.generate_block_mesh(&mut mesh, &block, x, y, z);
        }
    }

    self.mesh = if mesh.is_empty() { None } else { Some(mesh) };
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
        let block_id = sub_block.id;
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

    fn should_render_face(&mut self, sub_block: &SubBlock, face: usize) -> bool {
        // Check if face should be rendered based on connections or neighboring blocks
        // This is a simplified version - should be expanded based on your connection system
        true
    }

    fn calculate_variant_data(&mut self, sub_block: &SubBlock) -> u32 {
        // Pack variant and connection data into a single u32
        let variant = sub_block.id.1 as u32; // Variant ID
        let connections = sub_block.connections.bits() as u32;
        
        // Pack as: variant in upper 16 bits, connections in lower 16 bits
        (variant << 16) | connections
    }

    pub fn transform(&mut self) -> Mat4 {
        let pos = Vec3::new(
            self.position.x() as f32 * CHUNK_SIZE as f32,
            self.position.y() as f32 * CHUNK_SIZE as f32,
            self.position.z() as f32 * CHUNK_SIZE as f32,
        );
        Mat4::from_translation(pos)
    }

    pub fn is_visible(&self, frustum: &Frustum) -> bool {
        frustum.intersects_aabb(self.bounds.0, self.bounds.1)
    }

    pub fn get_aabb_corners(&mut self) -> [Vec3; 8] {
        let (min, max) = self.bounds;
        [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
        ]
    }

    pub fn is_solid_at(&mut self, world_x: i32, world_y: i32, world_z: i32) -> bool {
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
        let mut chunk = Self {
            position: serialized.coord,
            blocks: serialized.blocks,
            mesh: None,
            needs_remesh: true,
            bounds: (Vec3::ZERO, Vec3::ZERO),
        };
        
        // Recalculate bounds
        let min = Vec3::new(
            chunk.position.x() as f32 * CHUNK_SIZE as f32,
            chunk.position.y() as f32 * CHUNK_SIZE as f32,
            chunk.position.z() as f32 * CHUNK_SIZE as f32,
        );
        chunk.bounds = (min, min + Vec3::new(CHUNK_SIZE as f32, CHUNK_SIZE as f32, CHUNK_SIZE as f32));
        
        Ok(chunk)
    }
}

pub struct ChunkManager {
    chunks: HashMap<ChunkCoord, Arc<Chunk>>,
    renderer: ChunkRenderer,
    world_config: WorldGenConfig,
    compressed_cache: HashMap<ChunkCoord, Vec<CompressedBlock>>,
    block_registry: Arc<BlockRegistry>,
    visible_chunks: Vec<Arc<Chunk>>,
    last_view_proj: Option<Mat4>,
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
            visible_chunks: Vec::new(),
            last_view_proj: None,
        }
    }

    pub fn add_chunk(&mut self, coord: ChunkCoord, chunk: &mut Chunk) {
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

    pub fn generate_chunk(&mut self, coord: ChunkCoord) -> Chunk {
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

    pub fn update_visibility(&mut self, view_proj: &Mat4) {
        // Only recalculate if view-projection matrix changed
        if let Some(last) = &self.last_view_proj {
            if last.abs_diff_eq(*view_proj, 0.001) {
                return;
            }
        }
        self.last_view_proj = Some(*view_proj);

        let frustum = Frustum::from_view_projection(view_proj);
        self.visible_chunks.clear();

        for chunk in self.chunks.values() {
            if chunk.is_visible(&frustum) {
                self.visible_chunks.push(chunk.clone());
            }
        }
    }

    pub fn render_visible_chunks(
        &self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        camera: &Camera,
    ) {
        for chunk in &self.visible_chunks {
            if let Some(mesh) = &chunk.mesh {
                if !mesh.is_empty() {
                    self.renderer.render_chunk(device, command_buffer, chunk, camera);
                }
            }
        }
    }

    pub fn save_world(&mut self) -> std::io::Result<()> {
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

    pub fn get_block_at(&mut self, world_pos: Vec3) -> Option<(&Block, IVec3)> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos, CHUNK_SIZE as i32);
        let mut chunk = self.chunks.get(&chunk_coord)?;

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
}
