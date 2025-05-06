// In render/pipeline.rs
use crate::render::mesh::Mesh;
use crate::render::shaders::ShaderProgram;
use crate::render::{Camera, Shader};
use crate::world::block_material::BlockMaterial;
use crate::world::chunk::ChunkMesh;
use crate::world::{BlockRegistry, Chunk};
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
}

impl ChunkRenderer {
    pub fn new() -> Result<Self, anyhow::Error> {
        let shader = ShaderProgram::new("assets/shaders/chunk.vert", "assets/shaders/chunk.frag")?;

        let mut texture_atlas = 0;
        unsafe {
            gl::GenTextures(1, &mut texture_atlas);
            gl::BindTexture(gl::TEXTURE_2D, texture_atlas);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        }

        Ok(Self {
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
        })
    }

    fn init_default_materials(&mut self) -> Result<(), anyhow::Error> {
        self.load_material(
            1,
            BlockMaterial {
                name: "Stone".into(),
                albedo: Vec4::new(0.5, 0.5, 0.5, 1.0),
                roughness: 0.8,
                metallic: 0.2,
                texture_path: Some("textures/stone.png".into()),
                ..Default::default()
            },
        )?;

        self.load_material(
            2,
            BlockMaterial {
                name: "Grass".into(),
                albedo: Vec4::new(0.2, 0.8, 0.3, 1.0),
                roughness: 0.4,
                metallic: 0.0,
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

    pub fn upload_chunk_data(&mut self, _chunk: &mut Chunk, mesh: &mut ChunkMesh) {
        unsafe {
            // Generate and bind VAO
            let mut vao = 0;
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);
            mesh.vao = vao;

            // Generate and bind VBO for vertices
            let mut vbo = 0;
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            mesh.vbo = vbo;

            // Upload vertex data
            let vertex_data = mesh.vertices.as_slice();
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertex_data.len() * std::mem::size_of::<f32>()) as isize,
                vertex_data.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // Set up vertex position attribute
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
            gl::EnableVertexAttribArray(0);

            // Generate and bind VBO for normals
            let mut normal_vbo = 0;
            gl::GenBuffers(1, &mut normal_vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, normal_vbo);

            // Upload normal data
            let normal_data = mesh.normals.as_slice();
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (normal_data.len() * std::mem::size_of::<f32>()) as isize,
                normal_data.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // Set up normal attribute
            gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
            gl::EnableVertexAttribArray(1);

            // Generate and bind VBO for UVs
            let mut uv_vbo = 0;
            gl::GenBuffers(1, &mut uv_vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, uv_vbo);

            // Upload UV data
            let uv_data = mesh.uvs.as_slice();
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (uv_data.len() * std::mem::size_of::<f32>()) as isize,
                uv_data.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // Set up UV attribute
            gl::VertexAttribPointer(2, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
            gl::EnableVertexAttribArray(2);

            // Generate and bind EBO
            let mut ebo = 0;
            gl::GenBuffers(1, &mut ebo);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            mesh.ebo = ebo;

            // Upload index data
            let index_data = mesh.indices.as_slice();
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (index_data.len() * std::mem::size_of::<u32>()) as isize,
                index_data.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // Unbind VAO
            gl::BindVertexArray(0);
        }
    }

    pub fn generate_mesh(&self, chunk: &Chunk) -> ChunkMesh {
        let mut mesh = ChunkMesh::new();
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
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
        mesh.normals = normals;
        mesh.uvs = uvs;
        mesh.indices = indices;

        mesh
    }

    fn add_face(
        &self,
        position: Vec3,
        face: usize,
        material: &BlockMaterial,
        mesh: &mut ChunkMesh,
    ) {
        let tex_coords = self
            .texture_coordinates
            .get(&material.id)
            .unwrap_or(&[Vec2::ZERO; 4]);

        let (vertices, normals, tangents, bitangents) = match face {
            // West face
            0 => (
                [
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 1.0, 1.0),
                    Vec3::new(0.0, 0.0, 1.0),
                ],
                [Vec3::NEG_X; 4],
                [Vec3::NEG_Z; 4],
                [Vec3::NEG_Y; 4],
            ),
            // East face
            1 => (
                [
                    Vec3::new(1.0, 0.0, 1.0),
                    Vec3::new(1.0, 1.0, 1.0),
                    Vec3::new(1.0, 1.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                ],
                [Vec3::X; 4],
                [Vec3::Z; 4],
                [Vec3::Y; 4],
            ),
            // Bottom face
            2 => (
                [
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 1.0),
                    Vec3::new(0.0, 0.0, 1.0),
                ],
                [Vec3::NEG_Y; 4],
                [Vec3::X; 4],
                [Vec3::Z; 4],
            ),
            // Top face
            3 => (
                [
                    Vec3::new(0.0, 1.0, 1.0),
                    Vec3::new(1.0, 1.0, 1.0),
                    Vec3::new(1.0, 1.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                ],
                [Vec3::Y; 4],
                [Vec3::X; 4],
                [Vec3::NEG_Z; 4],
            ),
            // North face
            4 => (
                [
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(1.0, 1.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 0.0, 0.0),
                ],
                [Vec3::NEG_Z; 4],
                [Vec3::X; 4],
                [Vec3::Y; 4],
            ),
            // South face
            5 => (
                [
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(0.0, 1.0, 1.0),
                    Vec3::new(1.0, 1.0, 1.0),
                    Vec3::new(1.0, 0.0, 1.0),
                ],
                [Vec3::Z; 4],
                [Vec3::X; 4],
                [Vec3::Y; 4],
            ),
            _ => panic!("Invalid face direction"),
        };

        for i in 0..4 {
            mesh.vertex_data.extend_from_slice(&[
                // Position
                vertices[i].x,
                vertices[i].y,
                vertices[i].z,
                // Normal
                normals[i].x,
                normals[i].y,
                normals[i].z,
                // Tangent
                tangents[i].x,
                tangents[i].y,
                tangents[i].z,
                // Bitangent
                bitangents[i].x,
                bitangents[i].y,
                bitangents[i].z,
                // Texture coordinates
                tex_coords[i].x,
                tex_coords[i].y,
                // Material properties
                material.albedo[0], // Red component
                material.albedo[1], // Green component
                material.albedo[2], // Blue component
                material.albedo[3], // Alpha component
                material.roughness,
                material.metallic,
            ]);
        }

        let base_index = mesh.vertex_data.len() / 18; // 18 components per vertex

        for i in 0..4 {
            // Position (offset by chunk position)
            mesh.vertex_data.push(vertices[i].x + position.x);
            mesh.vertex_data.push(vertices[i].y + position.y);
            mesh.vertex_data.push(vertices[i].z + position.z);

            // Normal
            mesh.vertex_data.push(normals[i].x);
            mesh.vertex_data.push(normals[i].y);
            mesh.vertex_data.push(normals[i].z);

            // Tangent
            mesh.vertex_data.push(tangents[i].x);
            mesh.vertex_data.push(tangents[i].y);
            mesh.vertex_data.push(tangents[i].z);

            // Bitangent
            mesh.vertex_data.push(bitangents[i].x);
            mesh.vertex_data.push(bitangents[i].y);
            mesh.vertex_data.push(bitangents[i].z);

            // Texture coordinates
            mesh.vertex_data.push(tex_coords[i].x);
            mesh.vertex_data.push(tex_coords[i].y);

            // Material properties
            mesh.vertex_data.push(material.albedo.x);
            mesh.vertex_data.push(material.albedo.y);
            mesh.vertex_data.push(material.albedo.z);
            mesh.vertex_data.push(material.albedo.w);
            mesh.vertex_data.push(material.roughness);
            mesh.vertex_data.push(material.metallic);
        }

        // Add indices (two triangles per face)
        mesh.index_data.push(base_index as u32);
        mesh.index_data.push((base_index + 1) as u32);
        mesh.index_data.push((base_index + 2) as u32);
        mesh.index_data.push((base_index + 2) as u32);
        mesh.index_data.push((base_index + 3) as u32);
        mesh.index_data.push(base_index as u32);
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
                gl::BindTexture(gl::TEXTURE_2D, self.texture_atlas);
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
