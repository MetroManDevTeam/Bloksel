// In render/pipeline.rs
use crate::world::{Chunk, BlockRegistry};

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
}

impl ChunkRenderer {
    pub fn new() -> Result<Self> {
        let mut renderer = Self {
            materials: HashMap::new(),
            texture_atlas: Some(RgbaImage::new(ATLAS_START_SIZE, ATLAS_START_SIZE)),
            texture_atlas_id: None,
            texture_coordinates: HashMap::new(),
            current_atlas_pos: (TEXTURE_PADDING, TEXTURE_PADDING),
            max_row_height: 0,
            debug_mode: false,
            lod_level: 0,
            pending_textures: HashSet::new(),
        };

        renderer.init_default_materials()?;
        Ok(renderer)
    }

    fn init_default_materials(&mut self) -> Result<()> {
        self.load_material(1, BlockMaterial {
            name: "Stone".into(),
            albedo: Vec4::new(0.5, 0.5, 0.5, 1.0),
            roughness: 0.8,
            metallic: 0.2,
            texture_path: Some("textures/stone.png".into()),
            ..Default::default()
        })?;

        self.load_material(2, BlockMaterial {
            name: "Grass".into(),
            albedo: Vec4::new(0.2, 0.8, 0.3, 1.0),
            roughness: 0.4,
            metallic: 0.0,
            texture_path: Some("textures/grass.png".into()),
            ..Default::default()
        })?;

        Ok(())
    }

    pub fn load_material(&mut self, block_id: u16, material: BlockMaterial) -> Result<()> {
        if let Some(ref path) = material.texture_path {
            self.queue_texture_load(block_id, path)?;
        }
        self.materials.insert(block_id, material);
        Ok(())
    }

    fn queue_texture_load(&mut self, block_id: u16, path: &str) -> Result<()> {
        self.pending_textures.insert(block_id);
        Ok(())
    }

    pub fn process_texture_queue(&mut self) -> Result<()> {
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
                if current_pos.0 + width + TEXTURE_PADDING > new_atlas.width() ||
                   current_pos.1 + height + TEXTURE_PADDING > new_atlas.height() 
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
                        new_atlas.put_pixel(
                            current_pos.0 + x,
                            current_pos.1 + y,
                            *pixel,
                        );
                    }
                }

                // Store texture coordinates
                let u_min = current_pos.0 as f32 / new_atlas.width() as f32;
                let v_min = current_pos.1 as f32 / new_atlas.height() as f32;
                let u_max = (current_pos.0 + width) as f32 / new_atlas.width() as f32;
                let v_max = (current_pos.1 + height) as f32 / new_atlas.height() as f32;

                self.texture_coordinates.insert(block_id, [
                    Vec2::new(u_min, v_min),
                    Vec2::new(u_max, v_min),
                    Vec2::new(u_max, v_max),
                    Vec2::new(u_min, v_max),
                ]);

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
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as i32);
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

    fn upload_chunk_data(&self, chunk: &mut Chunk, mesh: &ChunkMesh) -> Result<()> {
    unsafe {
        // Generate buffers if they don't exist
        if chunk.mesh.vao == 0 {
            gl::GenVertexArrays(1, &mut chunk.mesh.vao);
        }
        if chunk.mesh.vbo == 0 {
            gl::GenBuffers(1, &mut chunk.mesh.vbo);
        }
        if chunk.mesh.ebo == 0 {
            gl::GenBuffers(1, &mut chunk.mesh.ebo);
        }

        gl::BindVertexArray(chunk.mesh.vao);

        // Upload vertex data
        gl::BindBuffer(gl::ARRAY_BUFFER, chunk.mesh.vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (mesh.vertex_data.len() * std::mem::size_of::<f32>()) as isize,
            mesh.vertex_data.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );

        // Upload index data
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, chunk.mesh.ebo);
        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            (mesh.index_data.len() * std::mem::size_of::<u32>()) as isize,
            mesh.index_data.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );

        // Vertex attributes (18 floats per vertex)
        let stride = (18 * std::mem::size_of::<f32>()) as GLsizei;
        
        // Position (location = 0)
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
        gl::EnableVertexAttribArray(0);
        
        // Normal (location = 1)
        gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE, stride, (3 * std::mem::size_of::<f32>()) as *const _);
        gl::EnableVertexAttribArray(1);
        
        // Tangent (location = 2)
        gl::VertexAttribPointer(2, 3, gl::FLOAT, gl::FALSE, stride, (6 * std::mem::size_of::<f32>()) as *const _);
        gl::EnableVertexAttribArray(2);
        
        // Bitangent (location = 3)
        gl::VertexAttribPointer(3, 3, gl::FLOAT, gl::FALSE, stride, (9 * std::mem::size_of::<f32>()) as *const _);
        gl::EnableVertexAttribArray(3);
        
        // Texture coordinates (location = 4)
        gl::VertexAttribPointer(4, 2, gl::FLOAT, gl::FALSE, stride, (12 * std::mem::size_of::<f32>()) as *const _);
        gl::EnableVertexAttribArray(4);
        
        // Material properties (location = 5)
        gl::VertexAttribPointer(5, 4, gl::FLOAT, gl::FALSE, stride, (14 * std::mem::size_of::<f32>()) as *const _);
        gl::EnableVertexAttribArray(5);

        chunk.mesh.index_count = mesh.index_data.len() as i32;
        chunk.mesh.needs_upload = false;
    }
    Ok(())
}

    pub fn generate_mesh(&self, chunk: &Chunk) -> ChunkMesh {
        match self.lod_level {
            0 => self.generate_greedy_mesh(chunk),
            1 => self.generate_simplified_mesh(chunk, 2),
            2 => self.generate_simplified_mesh(chunk, 4),
            _ => self.generate_bounding_box_mesh(chunk),
        }
    }

    fn generate_greedy_mesh(&self, chunk: &Chunk) -> ChunkMesh {
        let mut mesh = ChunkMesh::new();
        // Implement greedy meshing algorithm here
        // (This would be a full implementation spanning multiple functions)
        mesh
    }

    fn generate_simplified_mesh(&self, chunk: &Chunk, factor: u8) -> ChunkMesh {
        let mut mesh = ChunkMesh::new();
        // LOD implementation
        mesh
    }

    fn generate_bounding_box_mesh(&self, chunk: &Chunk) -> ChunkMesh {
        let mut mesh = ChunkMesh::new();
        // Simple bounding box representation
        mesh
    }

    fn add_face(&self, position: Vec3, face: usize, material: &BlockMaterial, mesh: &mut ChunkMesh) {
        let tex_coords = self.texture_coordinates.get(&material.id)
            .unwrap_or(&[Vec2::ZERO; 4]);

        let (vertices, normals, tangents, bitangents) = match face {
    // West face
    0 => (
        [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0)
        ],
        [Vec3::NEG_X; 4],
        [Vec3::NEG_Z; 4],
        [Vec3::NEG_Y; 4]
    ),
    // East face
    1 => (
        [
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0)
        ],
        [Vec3::X; 4],
        [Vec3::Z; 4],
        [Vec3::Y; 4]
    ),
    // Bottom face
    2 => (
        [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0)
        ],
        [Vec3::NEG_Y; 4],
        [Vec3::X; 4],
        [Vec3::Z; 4]
    ),
    // Top face
    3 => (
        [
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0)
        ],
        [Vec3::Y; 4],
        [Vec3::X; 4],
        [Vec3::NEG_Z; 4]
    ),
    // North face
    4 => (
        [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0)
        ],
        [Vec3::NEG_Z; 4],
        [Vec3::X; 4],
        [Vec3::Y; 4]
    ),
    // South face
    5 => (
        [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0)
        ],
        [Vec3::Z; 4],
        [Vec3::X; 4],
        [Vec3::Y; 4]
    ),
    _ => panic!("Invalid face direction")
};

        for i in 0..4 {
            mesh.vertex_data.extend_from_slice(&[
                // Position
                vertices[i].x, vertices[i].y, vertices[i].z,
                // Normal
                normals[i].x, normals[i].y, normals[i].z,
                // Tangent
                tangents[i].x, tangents[i].y, tangents[i].z,
                // Bitangent
                bitangents[i].x, bitangents[i].y, bitangents[i].z,
                // Texture coordinates
                tex_coords[i].x, tex_coords[i].y,
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

    pub fn render_chunk(
        &self,
        chunk: &Chunk,
        shader: &ShaderProgram,
        view_matrix: &Mat4,
        projection_matrix: &Mat4,
    ) -> Result<()> {
        unsafe {
            gl::BindVertexArray(chunk.mesh.vao);
            if chunk.mesh.needs_upload {
                 self.upload_chunk_data(&mut chunk, &chunk.mesh)?;
            }

            shader.set_uniform_mat4("model", &chunk.transform_matrix());
            shader.set_uniform_mat4("view", view_matrix);
            shader.set_uniform_mat4("projection", projection_matrix);

            if let Some(texture_id) = self.texture_atlas_id {
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, texture_id);
            }

            gl::DrawElements(
                gl::TRIANGLES,
                chunk.mesh.index_count as i32,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );

            Ok(())
        }
    }

    
}
