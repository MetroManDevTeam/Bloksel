use super::terrain_generator::{Block, BlockState, BlockOrientation, ChunkData};
use std::collections::HashMap;
use std::path::PathBuf;
use glam::{Vec3, Mat4};
use rayon::prelude::*;

#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

#[derive(Debug, Clone)]
pub struct SolidMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub block_id: String,
    pub texture_path: PathBuf,
    pub transform: Mat4,
}

pub struct ChunkRenderer {
    meshes: HashMap<(i32, i32), SolidMesh>,
    texture_cache: HashMap<String, PathBuf>,
}

impl ChunkRenderer {
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
            texture_cache: HashMap::new(),
        }
    }
    
    pub fn solidify_chunk(&mut self, chunk: ChunkData, texture_path: PathBuf) -> SolidMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut current_index = 0;
        let mut visited = [[[false; 30]; 30]; 30];
        
        // First pass: merge full blocks
        for x in 0..30 {
            for y in 0..30 {
                for z in 0..30 {
                    if visited[x][y][z] || !chunk.blocks[x][y][z].is_solid() {
                        continue;
                    }
                    
                    let block = chunk.blocks[x][y][z];
                    
                    if block.state == BlockState::Full {
                        let (w, h, d) = self.find_contiguous_blocks(
                            &chunk.blocks, &mut visited, x, y, z, block
                        );
                        
                        self.add_cube_to_mesh(
                            &mut vertices,
                            &mut indices,
                            &mut current_index,
                            x as f32, y as f32, z as f32,
                            w as f32, h as f32, d as f32,
                            block,
                            &texture_path
                        );
                    }
                }
            }
        }
        
        // Second pass: add partial blocks
        for x in 0..30 {
            for y in 0..30 {
                for z in 0..30 {
                    let block = chunk.blocks[x][y][z];
                    if !visited[x][y][z] && block.is_solid() {
                        match block.state {
                            BlockState::Half(orient) => {
                                self.add_partial_block(
                                    &mut vertices,
                                    &mut indices,
                                    &mut current_index,
                                    x, y, z,
                                    0.5, orient,
                                    block,
                                    &texture_path
                                );
                            }
                            BlockState::Quarter(orient) => {
                                self.add_partial_block(
                                    &mut vertices,
                                    &mut indices,
                                    &mut current_index,
                                    x, y, z,
                                    0.25, orient,
                                    block,
                                    &texture_path
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        // Create final mesh with transform
        let transform = Mat4::from_translation(Vec3::new(
            chunk.position.0 as f32 * 30.0,
            0.0,
            chunk.position.1 as f32 * 30.0
        ));
        
        let mesh = SolidMesh {
            vertices,
            indices,
            block_id: format!("chunk_{}_{}", chunk.position.0, chunk.position.1),
            texture_path,
            transform,
        };
        
        self.meshes.insert(chunk.position, mesh.clone());
        mesh
    }
    
    fn find_contiguous_blocks(
        &self,
        blocks: &[[[Block; 30]; 30]; 30],
        visited: &mut [[[bool; 30]; 30]; 30],
        start_x: usize,
        start_y: usize,
        start_z: usize,
        block: Block,
    ) -> (usize, usize, usize) {
        let mut width = 1;
        let mut height = 1;
        let mut depth = 1;
        
        // X expansion
        while start_x + width < 30 
            && blocks[start_x + width][start_y][start_z].can_merge(&block)
            && !visited[start_x + width][start_y][start_z] {
            width += 1;
        }
        
        // Y expansion
        'y_loop: while start_y + height < 30 {
            for x in start_x..start_x + width {
                if !blocks[x][start_y + height][start_z].can_merge(&block) 
                    || visited[x][start_y + height][start_z] {
                    break 'y_loop;
                }
            }
            height += 1;
        }
        
        // Z expansion
        'z_loop: while start_z + depth < 30 {
            for x in start_x..start_x + width {
                for y in start_y..start_y + height {
                    if !blocks[x][y][start_z + depth].can_merge(&block) 
                        || visited[x][y][start_z + depth] {
                        break 'z_loop;
                    }
                }
            }
            depth += 1;
        }
        
        // Mark visited
        for x in start_x..start_x + width {
            for y in start_y..start_y + height {
                for z in start_z..start_z + depth {
                    visited[x][y][z] = true;
                }
            }
        }
        
        (width, height, depth)
    }
    
    fn add_cube_to_mesh(
        &self,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        current_index: &mut u32,
        x: f32, y: f32, z: f32,
        width: f32, height: f32, depth: f32,
        block: Block,
        texture_path: &PathBuf,
    ) {
        let px = x;
        let py = y;
        let pz = z;
        let nx = x + width;
        let ny = y + height;
        let nz = z + depth;
        
        // Vertices
        let base_index = *current_index;
        vertices.extend_from_slice(&[
            // Front face
            Vertex { position: [px, py, pz], normal: [0.0, 0.0, -1.0], tex_coord: [0.0, 0.0] },
            Vertex { position: [nx, py, pz], normal: [0.0, 0.0, -1.0], tex_coord: [1.0, 0.0] },
            Vertex { position: [nx, ny, pz], normal: [0.0, 0.0, -1.0], tex_coord: [1.0, 1.0] },
            Vertex { position: [px, ny, pz], normal: [0.0, 0.0, -1.0], tex_coord: [0.0, 1.0] },
            // Back face
            Vertex { position: [px, py, nz], normal: [0.0, 0.0, 1.0], tex_coord: [0.0, 0.0] },
            Vertex { position: [nx, py, nz], normal: [0.0, 0.0, 1.0], tex_coord: [1.0, 0.0] },
            Vertex { position: [nx, ny, nz], normal: [0.0, 0.0, 1.0], tex_coord: [1.0, 1.0] },
            Vertex { position: [px, ny, nz], normal: [0.0, 0.0, 1.0], tex_coord: [0.0, 1.0] },
        ]);
        
        // Indices (two triangles per face)
        indices.extend_from_slice(&[
            // Front
            base_index, base_index + 1, base_index + 2,
            base_index + 2, base_index + 3, base_index,
            // Back
            base_index + 4, base_index + 5, base_index + 6,
            base_index + 6, base_index + 7, base_index + 4,
            // Left
            base_index, base_index + 3, base_index + 7,
            base_index + 7, base_index + 4, base_index,
            // Right
            base_index + 1, base_index + 2, base_index + 6,
            base_index + 6, base_index + 5, base_index + 1,
            // Top
            base_index + 2, base_index + 3, base_index + 7,
            base_index + 7, base_index + 6, base_index + 2,
            // Bottom
            base_index, base_index + 1, base_index + 5,
            base_index + 5, base_index + 4, base_index,
        ]);
        
        *current_index += 8;
    }
    
    fn add_partial_block(
        &self,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        current_index: &mut u32,
        x: usize, y: usize, z: usize,
        size: f32,
        orientation: BlockOrientation,
        block: Block,
        texture_path: &PathBuf,
    ) {
        let px = x as f32;
        let py = y as f32;
        let pz = z as f32;
        let nx = px + 1.0;
        let ny = py + 1.0;
        let nz = pz + 1.0;
        
        let (v1, v2, v3, v4) = match orientation {
            BlockOrientation::Up => (
                [px, py + size, pz], [nx, py + size, pz],
                [nx, py + size, nz], [px, py + size, nz]
            ),
            BlockOrientation::Down => (
                [px, ny - size, pz], [nx, ny - size, pz],
                [nx, ny - size, nz], [px, ny - size, nz]
            ),
            BlockOrientation::North => (
                [px, py, pz + size], [nx, py, pz + size],
                [nx, ny, pz + size], [px, ny, pz + size]
            ),
            BlockOrientation::South => (
                [px, py, nz - size], [nx, py, nz - size],
                [nx, ny, nz - size], [px, ny, nz - size]
            ),
            BlockOrientation::East => (
                [px + size, py, pz], [px + size, py, nz],
                [px + size, ny, nz], [px + size, ny, pz]
            ),
            BlockOrientation::West => (
                [nx - size, py, pz], [nx - size, py, nz],
                [nx - size, ny, nz], [nx - size, ny, pz]
            ),
        };
        
        let normal = match orientation {
            BlockOrientation::Up => [0.0, 1.0, 0.0],
            BlockOrientation::Down => [0.0, -1.0, 0.0],
            BlockOrientation::North => [0.0, 0.0, 1.0],
            BlockOrientation::South => [0.0, 0.0, -1.0],
            BlockOrientation::East => [1.0, 0.0, 0.0],
            BlockOrientation::West => [-1.0, 0.0, 0.0],
        };
        
        let base_index = *current_index;
        vertices.extend_from_slice(&[
            Vertex { position: v1, normal, tex_coord: [0.0, 0.0] },
            Vertex { position: v2, normal, tex_coord: [1.0, 0.0] },
            Vertex { position: v3, normal, tex_coord: [1.0, 1.0] },
            Vertex { position: v4, normal, tex_coord: [0.0, 1.0] },
        ]);
        
        indices.extend_from_slice(&[
            base_index, base_index + 1, base_index + 2,
            base_index + 2, base_index + 3, base_index,
        ]);
        
        *current_index += 4;
    }
    
    pub fn get_chunk_mesh(&self, x: i32, z: i32) -> Option<&SolidMesh> {
        self.meshes.get(&(x, z))
    }
    
    pub fn clear_meshes(&mut self) {
        self.meshes.clear();
    }
}
