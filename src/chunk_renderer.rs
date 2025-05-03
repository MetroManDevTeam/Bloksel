use crate::terrain_generator::{BlockData, Chunk, ChunkCoord, ChunkMesh, Integrity, Orientation};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use glam::{Vec3, Vec2, Vec4};
use image::DynamicImage;

#[derive(Debug, Clone)]
pub struct BlockMaterial {
    pub top_color: Vec4,    // RGBA
    pub side_color: Vec4,
    pub bottom_color: Vec4,
    pub texture_path: Option<String>,
    pub texture_id: Option<u32>,
}

impl Default for BlockMaterial {
    fn default() -> Self {
        Self {
            top_color: Vec4::new(0.5, 0.5, 0.5, 1.0), // Gray
            side_color: Vec4::new(0.5, 0.5, 0.5, 1.0),
            bottom_color: Vec4::new(0.5, 0.5, 0.5, 1.0),
            texture_path: None,
            texture_id: None,
        }
    }
}

pub struct ChunkRenderer {
    materials: HashMap<u16, BlockMaterial>,
    texture_atlas: Option<DynamicImage>,
    texture_atlas_id: Option<u32>,
    missing_texture_color: Vec4,
}

impl ChunkRenderer {
    pub fn new() -> Self {
        let mut renderer = Self {
            materials: HashMap::new(),
            texture_atlas: None,
            texture_atlas_id: None,
            missing_texture_color: Vec4::new(1.0, 0.0, 1.0, 1.0), // Magenta error color
        };

        // Initialize default materials
        renderer.define_core_materials();
        renderer
    }

    fn define_core_materials(&mut self) {
        // Stone (ID 1)
        self.materials.insert(1, BlockMaterial {
            top_color: Vec4::new(0.5, 0.5, 0.5, 1.0),
            side_color: Vec4::new(0.5, 0.5, 0.5, 1.0),
            bottom_color: Vec4::new(0.4, 0.4, 0.4, 1.0),
            texture_path: Some("textures/stone.png".to_string()),
            texture_id: None,
        });

        // Grass (ID 2)
        self.materials.insert(2, BlockMaterial {
            top_color: Vec4::new(0.2, 0.8, 0.3, 1.0),
            side_color: Vec4::new(0.5, 0.5, 0.3, 1.0),
            bottom_color: Vec4::new(0.4, 0.3, 0.2, 1.0),
            texture_path: Some("textures/grass.png".to_string()),
            texture_id: None,
        });

        // Dirt (ID 3)
        self.materials.insert(3, BlockMaterial {
            top_color: Vec4::new(0.4, 0.3, 0.2, 1.0),
            side_color: Vec4::new(0.4, 0.3, 0.2, 1.0),
            bottom_color: Vec4::new(0.3, 0.2, 0.1, 1.0),
            texture_path: Some("textures/dirt.png".to_string()),
            texture_id: None,
        });

        // Sand (ID 4)
        self.materials.insert(4, BlockMaterial {
            top_color: Vec4::new(0.9, 0.8, 0.5, 1.0),
            side_color: Vec4::new(0.9, 0.8, 0.5, 1.0),
            bottom_color: Vec4::new(0.8, 0.7, 0.4, 1.0),
            texture_path: Some("textures/sand.png".to_string()),
            texture_id: None,
        });
    }

    pub fn load_textures(&mut self) -> Result<(), image::ImageError> {
        let mut texture_atlas = DynamicImage::new_rgba8(1024, 1024);
        let mut current_x = 0;
        let mut current_y = 0;
        let mut max_row_height = 0;

        for (_, material) in self.materials.iter_mut() {
            if let Some(path) = &material.texture_path {
                match image::open(path) {
                    Ok(img) => {
                        let img = img.to_rgba8();
                        let (width, height) = img.dimensions();

                        // Check if texture fits in current row
                        if current_x + width > 1024 {
                            current_x = 0;
                            current_y += max_row_height;
                            max_row_height = 0;
                        }

                        // Check if we have space in atlas
                        if current_y + height > 1024 {
                            log::error!("Texture atlas full!");
                            break;
                        }

                        // Copy texture to atlas
                        for y in 0..height {
                            for x in 0..width {
                                let pixel = img.get_pixel(x, y);
                                texture_atlas.put_pixel(
                                    current_x + x,
                                    current_y + y,
                                    *pixel,
                                );
                            }
                        }

                        // Update material with texture coordinates
                        material.texture_id = Some(0); // Using single atlas for now

                        current_x += width;
                        max_row_height = max_row_height.max(height);
                    }
                    Err(e) => {
                        log::warn!("Failed to load texture {}: {}", path, e);
                        material.texture_id = None;
                    }
                }
            }
        }

        self.texture_atlas = Some(texture_atlas);
        Ok(())
    }

    pub fn upload_textures(&mut self) {
        unsafe {
            let mut texture_id = 0;
            gl::GenTextures(1, &mut texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);

            // Set texture parameters
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST_MIPMAP_LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);

            if let Some(atlas) = &self.texture_atlas {
                let img = atlas.to_rgba8();
                let (width, height) = img.dimensions();
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    width as i32,
                    height as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    img.as_ptr() as *const _,
                );
                gl::GenerateMipmap(gl::TEXTURE_2D);
            }

            self.texture_atlas_id = Some(texture_id);
        }
    }

    pub fn generate_mesh(&self, chunk: &Chunk) -> ChunkMesh {
        let mut vertex_data = Vec::new();
        let mut index_data = Vec::new();
        let mut current_index = 0;

        let chunk_size = chunk.blocks.len();
        
        for x in 0..chunk_size {
            for y in 0..chunk_size {
                for z in 0..chunk_size {
                    if let Some(block) = &chunk.blocks[x][y][z] {
                        // Get block material or fallback to default
                        let material = self.materials.get(&block.id).unwrap_or_else(|| {
                            log::warn!("Unknown block ID: {}, using default material", block.id);
                            &self.materials.get(&1).unwrap() // Fallback to stone
                        });

                        // Check each neighbor to see if we need to render a face
                        let neighbors = [
                            (x > 0 && chunk.blocks[x-1][y][z].is_some()), // West
                            (x < chunk_size-1 && chunk.blocks[x+1][y][z].is_some()), // East
                            (y > 0 && chunk.blocks[x][y-1][z].is_some()), // Down
                            (y < chunk_size-1 && chunk.blocks[x][y+1][z].is_some()), // Up
                            (z > 0 && chunk.blocks[x][y][z-1].is_some()), // South
                            (z < chunk_size-1 && chunk.blocks[x][y][z+1].is_some()), // North
                        ];

                        for (face, &occluded) in neighbors.iter().enumerate() {
                            if !occluded {
                                // Add quad for this face with proper material
                                self.add_block_face(
                                    x as f32, y as f32, z as f32,
                                    face,
                                    block,
                                    material,
                                    &mut vertex_data,
                                    &mut index_data,
                                    current_index,
                                );
                                current_index += 4;
                            }
                        }
                    }
                }
            }
        }

        ChunkMesh {
            vertex_data,
            index_data,
        }
    }

    fn add_block_face(
        &self,
        x: f32, y: f32, z: f32,
        face: usize,
        block: &BlockData,
        material: &BlockMaterial,
        vertex_data: &mut Vec<f32>,
        index_data: &mut Vec<u32>,
        base_index: u32,
    ) {
        // Determine face color based on orientation
        let face_color = match face {
            0 | 1 | 4 | 5 => material.side_color,    // Vertical faces
            2 => material.bottom_color,              // Bottom face
            3 => material.top_color,                 // Top face
            _ => material.side_color,
        };

        // Check if we should use texture or color
        let use_texture = material.texture_id.is_some() && self.texture_atlas_id.is_some();
        let tex_coords = if use_texture {
            self.get_texture_coords(block.id, face)
        } else {
            [Vec2::ZERO; 4]
        };

        // Positions for a cube face
        let positions = match face {
            0 => [ // West face (left)
                (x,     y,     z + 1.0),
                (x,     y + 1.0, z + 1.0),
                (x,     y + 1.0, z),
                (x,     y,     z),
            ],
            1 => [ // East face (right)
                (x + 1.0, y,     z),
                (x + 1.0, y + 1.0, z),
                (x + 1.0, y + 1.0, z + 1.0),
                (x + 1.0, y,     z + 1.0),
            ],
            2 => [ // Bottom face
                (x,     y,     z),
                (x + 1.0, y,     z),
                (x + 1.0, y,     z + 1.0),
                (x,     y,     z + 1.0),
            ],
            3 => [ // Top face
                (x,     y + 1.0, z + 1.0),
                (x + 1.0, y + 1.0, z + 1.0),
                (x + 1.0, y + 1.0, z),
                (x,     y + 1.0, z),
            ],
            4 => [ // South face (back)
                (x + 1.0, y,     z),
                (x + 1.0, y + 1.0, z),
                (x,     y + 1.0, z),
                (x,     y,     z),
            ],
            5 => [ // North face (front)
                (x,     y,     z + 1.0),
                (x,     y + 1.0, z + 1.0),
                (x + 1.0, y + 1.0, z + 1.0),
                (x + 1.0, y,     z + 1.0),
            ],
            _ => unreachable!(),
        };

        // Adjust for integrity (partial blocks)
        let positions = self.adjust_for_integrity(positions, block.integrity, face);

        // Add vertices (position, normal, color, texture coordinates, block ID)
        for (i, pos) in positions.iter().enumerate() {
            // Position
            vertex_data.extend_from_slice(&[pos.0, pos.1, pos.2]);

            // Normal (based on face)
            let normal = match face {
                0 => [-1.0, 0.0, 0.0], // West
                1 => [1.0, 0.0, 0.0],  // East
                2 => [0.0, -1.0, 0.0], // Bottom
                3 => [0.0, 1.0, 0.0],  // Top
                4 => [0.0, 0.0, -1.0], // South
                5 => [0.0, 0.0, 1.0],  // North
                _ => [0.0, 0.0, 0.0],
            };
            vertex_data.extend_from_slice(&normal);

            // Color (only used if texture is not available)
            vertex_data.extend_from_slice(&[
                face_color.x,
                face_color.y,
                face_color.z,
                face_color.w,
            ]);

            // Texture coordinates
            vertex_data.extend_from_slice(&[
                tex_coords[i].x,
                tex_coords[i].y,
            ]);

            // Block ID (for shader)
            vertex_data.push(block.id as f32);

            // Flag for texture/color (0 = color, 1 = texture)
            vertex_data.push(if use_texture { 1.0 } else { 0.0 });
        }

        // Add indices (two triangles)
        index_data.extend_from_slice(&[
            base_index, base_index + 1, base_index + 2,
            base_index, base_index + 2, base_index + 3,
        ]);
    }

    fn get_texture_coords(&self, block_id: u16, face: usize) -> [Vec2; 4] {
        // This is a simplified version - a real implementation would calculate
        // proper atlas coordinates based on block ID and face
        match face {
            0 => [ // West
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
            1 => [ // East
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
            2 => [ // Bottom
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
            3 => [ // Top
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
            4 => [ // South
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
            5 => [ // North
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
            _ => [Vec2::ZERO; 4],
        }
    }

    fn adjust_for_integrity(
        &self,
        positions: [(f32, f32, f32); 4],
        integrity: Integrity,
        face: usize,
    ) -> [(f32, f32, f32); 4] {
        match integrity {
            Integrity::Full => positions,
            Integrity::Half => {
                match face {
                    0 | 1 => { // Vertical faces (X axis)
                        [
                            positions[0],
                            (positions.1.0, positions.1.1, positions.1.2 * 0.5),
                            (positions.2.0, positions.2.1, positions.2.2 * 0.5),
                            (positions.3.0, positions.3.1, positions.3.2 * 0.5),
                        ]
                    },
                    2 | 3 => { // Horizontal faces (Y axis)
                        [
                            positions[0],
                            (positions.1.0, positions.1.1 * 0.5, positions.1.2),
                            (positions.2.0, positions.2.1 * 0.5, positions.2.2),
                            (positions.3.0, positions.3.1 * 0.5, positions.3.2),
                        ]
                    },
                    4 | 5 => { // Vertical faces (Z axis)
                        [
                            positions[0],
                            (positions.1.0 * 0.5, positions.1.1, positions.1.2),
                            (positions.2.0 * 0.5, positions.2.1, positions.2.2),
                            (positions.3.0 * 0.5, positions.3.1, positions.3.2),
                        ]
                    },
                    _ => positions,
                }
            },
            Integrity::Quarter => {
                match face {
                    0 | 1 => { // Vertical faces (X axis)
                        [
                            positions[0],
                            (positions.1.0, positions.1.1, positions.1.2 * 0.25),
                            (positions.2.0, positions.2.1, positions.2.2 * 0.25),
                            (positions.3.0, positions.3.1, positions.3.2 * 0.25),
                        ]
                    },
                    2 | 3 => { // Horizontal faces (Y axis)
                        [
                            positions[0],
                            (positions.1.0, positions.1.1 * 0.25, positions.1.2),
                            (positions.2.0, positions.2.1 * 0.25, positions.2.2),
                            (positions.3.0, positions.3.1 * 0.25, positions.3.2),
                        ]
                    },
                    4 | 5 => { // Vertical faces (Z axis)
                        [
                            positions[0],
                            (positions.1.0 * 0.25, positions.1.1, positions.1.2),
                            (positions.2.0 * 0.25, positions.2.1, positions.2.2),
                            (positions.3.0 * 0.25, positions.3.1, positions.3.2),
                        ]
                    },
                    _ => positions,
                }
            },
            Integrity::Special => positions, // Special blocks are handled elsewhere
        }
    }

    pub fn update_chunk_mesh(&self, chunk: &mut Chunk) {
        if chunk.needs_remesh {
            let mesh = self.generate_mesh(chunk);
            chunk.mesh = Some(mesh);
            chunk.needs_remesh = false;
            chunk.needs_upload = true;
        }
    }

    pub fn render_chunk(
        &self,
        chunk: &Chunk,
        shader: &ShaderProgram,
        camera_pos: &[f32; 3],
        light_pos: &[f32; 3],
        view_matrix: &[f32; 16],
        projection_matrix: &[f32; 16],
    ) {
        if let Some(mesh) = &chunk.mesh {
            unsafe {
                // Set shader uniforms
                shader.set_uniform_mat4("model", &chunk.get_model_matrix());
                shader.set_uniform_mat4("view", view_matrix);
                shader.set_uniform_mat4("projection", projection_matrix);
                shader.set_uniform_vec3("viewPos", camera_pos);
                shader.set_uniform_vec3("lightPos", light_pos);
                shader.set_uniform_1i("textureAtlas", 0);

                // Bind texture atlas if available
                if let Some(texture_id) = self.texture_atlas_id {
                    gl::ActiveTexture(gl::TEXTURE0);
                    gl::BindTexture(gl::TEXTURE_2D, texture_id);
                }

                // Enable depth testing
                gl::Enable(gl::DEPTH_TEST);
                gl::DepthFunc(gl::LESS);

                // Bind VAO and upload data if needed
                gl::BindVertexArray(chunk.vao);
                if chunk.needs_upload {
                    gl::BindBuffer(gl::ARRAY_BUFFER, chunk.vbo);
                    gl::BufferData(
                        gl::ARRAY_BUFFER,
                        (mesh.vertex_data.len() * std::mem::size_of::<f32>()) as isize,
                        mesh.vertex_data.as_ptr() as *const _,
                        gl::STATIC_DRAW,
                    );

                    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, chunk.ebo);
                    gl::BufferData(
                        gl::ELEMENT_ARRAY_BUFFER,
                        (mesh.index_data.len() * std::mem::size_of::<u32>()) as isize,
                        mesh.index_data.as_ptr() as *const _,
                        gl::STATIC_DRAW,
                    );

                    chunk.needs_upload = false;
                }

                // Draw the mesh
                gl::DrawElements(
                    gl::TRIANGLES,
                    mesh.index_data.len() as i32,
                    gl::UNSIGNED_INT,
                    std::ptr::null(),
                );

                // Cleanup
                gl::BindVertexArray(0);
                gl::BindTexture(gl::TEXTURE_2D, 0);
            }
        }
    }
}

// Vertex format for the shader
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
    tex_coords: [f32; 2],
    block_id: f32,
    use_texture: f32,
}
