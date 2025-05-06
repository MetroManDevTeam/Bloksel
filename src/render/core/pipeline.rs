// In render/pipeline.rs
use crate::render::core::{Camera, Shader};
use crate::render::mesh::Mesh;
use crate::render::shaders::ShaderProgram;
use crate::world::block::Block;
use crate::world::block_material::BlockMaterial;
use crate::world::blocks_data::BlockRegistry;
use crate::world::chunk::{CHUNK_SIZE, Chunk, ChunkMesh};
use anyhow::Context;
use anyhow::Result;
use gl::types::{GLsizei, GLuint};
use glam::{Mat4, Vec2, Vec3, Vec4};
use image::DynamicImage;
use image::RgbaImage;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

const ATLAS_START_SIZE: u32 = 16;
const MAX_ATLAS_SIZE: u32 = 2048;
const TEXTURE_PADDING: u32 = 2;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Texture atlas is full")]
    AtlasFull,
    #[error("Failed to load texture: {0}")]
    TextureLoadError(String),
    #[error("OpenGL error: {0}")]
    OpenGLError(String),
}

pub struct ChunkRenderer {
    materials: HashMap<u16, BlockMaterial>,
    texture_atlas: Option<RgbaImage>,
    texture_atlas_id: Option<u32>,
    texture_coordinates: HashMap<u16, [Vec2; 4]>,
    current_atlas_pos: (u32, u32),
    max_row_height: u32,
    pub debug_mode: bool,
    pub lod_level: u8,
    pending_textures: HashSet<u16>,
    shader: ShaderProgram,
    texture_atlas_size: u32,
    block_registry: Arc<BlockRegistry>,
}

impl ChunkRenderer {
    pub fn new(
        shader: ShaderProgram,
        texture_atlas: GLuint,
        block_registry: Arc<BlockRegistry>,
    ) -> Self {
        Self {
            materials: HashMap::new(),
            texture_atlas: Some(RgbaImage::new(ATLAS_START_SIZE, ATLAS_START_SIZE)),
            texture_atlas_id: None,
            texture_coordinates: HashMap::new(),
            current_atlas_pos: (TEXTURE_PADDING, TEXTURE_PADDING),
            max_row_height: 0,
            debug_mode: false,
            lod_level: 0,
            pending_textures: HashSet::new(),
            shader,
            texture_atlas_size: ATLAS_START_SIZE,
            block_registry,
        }
    }

    fn init_default_materials(&mut self) -> Result<(), anyhow::Error> {
        self.load_material(
            1,
            BlockMaterial {
                id: 1,
                name: "Stone".into(),
                albedo: [0.5, 0.5, 0.5, 1.0],
                roughness: 0.8,
                metallic: 0.0,
                emissive: 0.0,
                texture_path: Some("textures/stone.png".into()),
                normal_map_path: None,
                occlusion_map_path: None,
                tintable: true,
                grayscale_base: false,
                tint_mask_path: None,
                vertex_colored: false,
            },
        )?;

        self.load_material(
            2,
            BlockMaterial {
                name: "Grass".into(),
                albedo: Vec4::new(0.2, 0.8, 0.3, 1.0).into(),
                roughness: 0.9,
                metallic: 0.0,
                emissive: 0.0,
                texture_path: Some("textures/grass.png".into()),
                ..Default::default()
            },
        )?;

        Ok(())
    }

    pub fn load_material(
        &mut self,
        block_id: u16,
        material: BlockMaterial,
    ) -> Result<(), anyhow::Error> {
        if let Some(ref path) = material.texture_path {
            self.queue_texture_load(block_id, path)?;
        }
        self.materials.insert(block_id, material);
        Ok(())
    }

    fn queue_texture_load(&mut self, block_id: u16, path: &str) -> Result<(), anyhow::Error> {
        self.pending_textures.insert(block_id);
        Ok(())
    }

    pub fn process_texture_queue(&mut self) -> Result<(), anyhow::Error> {
        let mut new_atlas = RgbaImage::new(ATLAS_START_SIZE, ATLAS_START_SIZE);
        let mut current_pos = (TEXTURE_PADDING, TEXTURE_PADDING);
        let mut max_row_height = 0;

        for &block_id in &self.pending_textures.clone() {
            let material = self.materials.get(&block_id).unwrap();
            if let Some(path) = &material.texture_path {
                let img = image::open(path)
                    .with_context(|| format!("Failed to load texture: {}", path))?
                    .to_rgba8();

                let (width, height) = img.dimensions();

                // Check if we need to expand atlas
                if current_pos.0 + width + TEXTURE_PADDING > new_atlas.width()
                    || current_pos.1 + height + TEXTURE_PADDING > new_atlas.height()
                {
                    let new_size = (new_atlas.width() * 2).min(MAX_ATLAS_SIZE);
                    if new_size > new_atlas.width() {
                        new_atlas = RgbaImage::new(new_size, new_size);
                        current_pos = (TEXTURE_PADDING, TEXTURE_PADDING);
                        max_row_height = 0;
                    } else {
                        return Err(RenderError::AtlasFull.into());
                    }
                }

                // Copy texture to atlas
                for y in 0..height {
                    for x in 0..width {
                        let pixel = img.get_pixel(x, y);
                        new_atlas.put_pixel(current_pos.0 + x, current_pos.1 + y, *pixel);
                    }
                }

                // Store texture coordinates
                let u_min = current_pos.0 as f32 / new_atlas.width() as f32;
                let v_min = current_pos.1 as f32 / new_atlas.height() as f32;
                let u_max = (current_pos.0 + width) as f32 / new_atlas.width() as f32;
                let v_max = (current_pos.1 + height) as f32 / new_atlas.height() as f32;

                self.texture_coordinates.insert(
                    block_id,
                    [
                        Vec2::new(u_min, v_min),
                        Vec2::new(u_max, v_min),
                        Vec2::new(u_max, v_max),
                        Vec2::new(u_min, v_max),
                    ],
                );

                current_pos.0 += width + TEXTURE_PADDING;
                max_row_height = max_row_height.max(height);

                if current_pos.0 + TEXTURE_PADDING > new_atlas.width() {
                    current_pos.0 = TEXTURE_PADDING;
                    current_pos.1 += max_row_height + TEXTURE_PADDING;
                    max_row_height = 0;
                }
            }
        }

        self.texture_atlas = Some(new_atlas);
        self.pending_textures.clear();
        Ok(())
    }

    pub fn upload_textures(&mut self) -> Result<(), RenderError> {
        unsafe {
            let mut texture_id: GLuint = 0;
            gl::GenTextures(1, &mut texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);

            // Set texture parameters
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::LINEAR_MIPMAP_LINEAR as i32,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            if let Some(atlas) = &self.texture_atlas {
                let (width, height) = atlas.dimensions();
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    width as i32,
                    height as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    atlas.as_ptr() as *const _,
                );
                gl::GenerateMipmap(gl::TEXTURE_2D);
            }

            self.texture_atlas_id = Some(texture_id);
        }
        Ok(())
    }

    pub fn upload_chunk_data(&self, mesh: &mut ChunkMesh) {
        unsafe {
            // Generate and bind VAO
            gl::GenVertexArrays(1, &mut mesh.vao);
            gl::BindVertexArray(mesh.vao);

            // Generate and bind VBO for vertices
            gl::GenBuffers(1, &mut mesh.vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, mesh.vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (mesh.vertices.len() * std::mem::size_of::<f32>()) as isize,
                mesh.vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
            gl::EnableVertexAttribArray(0);

            // Generate and bind EBO for indices
            gl::GenBuffers(1, &mut mesh.ebo);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, mesh.ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (mesh.indices.len() * std::mem::size_of::<u32>()) as isize,
                mesh.indices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // Unbind VAO
            gl::BindVertexArray(0);
        }
    }

    pub fn generate_mesh(&self, chunk: &Chunk) -> ChunkMesh {
        let mut mesh = ChunkMesh::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if let Some(block) = chunk.get_block(x, y, z) {
                        let material = block.get_material(&self.block_registry);
                        let position = Vec3::new(x as f32, y as f32, z as f32);

                        // Add vertices
                        vertices.push(position.x);
                        vertices.push(position.y);
                        vertices.push(position.z);

                        // Add normals
                        normals.push(0.0);
                        normals.push(1.0);
                        normals.push(0.0);

                        // Add UVs
                        uvs.push(0.0);
                        uvs.push(0.0);

                        // Add indices
                        let base_index = (vertices.len() / 3) as u32 - 1;
                        indices.push(base_index);
                        indices.push(base_index + 1);
                        indices.push(base_index + 2);
                        indices.push(base_index + 2);
                        indices.push(base_index + 3);
                        indices.push(base_index);
                    }
                }
            }
        }

        mesh.vertices = vertices;
        mesh.indices = indices;
        mesh.normals = normals;
        mesh.uvs = uvs;
        mesh
    }

    pub fn render_chunk(&self, chunk: &Chunk, camera: &Camera) {
        if let Some(mesh) = &chunk.mesh {
            unsafe {
                self.shader.use_program();
                self.shader.set_uniform("model", &chunk.transform());
                self.shader.set_uniform("view", &camera.view_matrix());
                self.shader
                    .set_uniform("projection", &camera.projection_matrix());

                // Bind texture atlas
                gl::ActiveTexture(gl::TEXTURE0);
                if let Some(texture_id) = self.texture_atlas_id {
                    gl::BindTexture(gl::TEXTURE_2D, texture_id);
                }
                self.shader.set_uniform("texture_atlas", &0);

                // Bind VAO and draw
                gl::BindVertexArray(mesh.vao);
                gl::DrawElements(
                    gl::TRIANGLES,
                    mesh.indices.len() as i32,
                    gl::UNSIGNED_INT,
                    std::ptr::null(),
                );
                gl::BindVertexArray(0);
            }
        }
    }
}

pub struct RenderPipeline {
    pub camera: Camera,
    pub shader: Arc<Shader>,
    pub meshes: Vec<Arc<Mesh>>,
}

impl RenderPipeline {
    pub fn new(camera: Camera, shader: Arc<Shader>) -> Self {
        Self {
            camera,
            shader,
            meshes: Vec::new(),
        }
    }

    pub fn add_mesh(&mut self, mesh: Arc<Mesh>) {
        self.meshes.push(mesh);
    }
}
