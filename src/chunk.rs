// src/world/chunk.rs
use crate::terrain_generator::{BlockData, Chunk, ChunkCoord, ChunkMesh, Integrity, Orientation, TerrainGenerator};
use crate::chunk_renderer::ChunkRenderer;
use glam::{IVec3, Vec3};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct WorldConfig {
    pub world_name: String,
    pub chunk_size: usize,
    pub sub_resolution: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct SerializedChunk {
    coord: ChunkCoord,
    blocks: Vec<CompressedBlock>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CompressedBlock {
    position: (usize, usize, usize),
    id: u16,
    sub_blocks: Vec<CompressedSubBlock>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CompressedSubBlock {
    local_pos: (u8, u8, u8),
    id: u16,
    integrity: Integrity,
    orientation: Orientation,
}

pub struct ChunkManager {
    chunks: HashMap<ChunkCoord, Chunk>,
    renderer: ChunkRenderer,
    world_config: WorldConfig,
    compressed_cache: HashMap<ChunkCoord, Vec<CompressedBlock>>,
}

impl ChunkManager {
    pub fn new(world_config: WorldConfig, renderer: ChunkRenderer) -> Self {
        Self {
            chunks: HashMap::new(),
            renderer,
            world_config,
            compressed_cache: HashMap::new(),
        }
    }

    pub fn add_chunk(&mut self, coord: ChunkCoord, chunk: Chunk) {
        let mut compressed = Vec::new();

        for x in 0..self.world_config.chunk_size {
            for y in 0..self.world_config.chunk_size {
                for z in 0..self.world_config.chunk_size {
                    if let Some(block) = &chunk.blocks[x][y][z] {
                        let mut sub_blocks = Vec::new();

                        for ((sx, sy, sz), sub) in &block.grid {
                            if sub.id != 0 {
                                sub_blocks.push(CompressedSubBlock {
                                    local_pos: (*sx, *sy, *sz),
                                    id: sub.id,
                                    integrity: sub.integrity,
                                    orientation: sub.orientation,
                                });
                            }
                        }

                        compressed.push(CompressedBlock {
                            position: (x, y, z),
                            id: block.id,
                            sub_blocks,
                        });
                    }
                }
            }
        }

        self.compressed_cache.insert(coord, compressed);
        self.chunks.insert(coord, chunk);
    }

    pub fn get_or_generate_chunk(&mut self, coord: ChunkCoord, seed: u32) -> &Chunk {
        if !self.chunks.contains_key(&coord) {
            let chunk = self.generate_chunk(coord, seed);
            self.add_chunk(coord, chunk);
        }
        self.chunks.get(&coord).unwrap()
    }

    pub fn generate_chunk(&self, coord: ChunkCoord, seed: u32) -> Chunk {
        let generator = TerrainGenerator::new(
            seed,
            self.world_config.chunk_size,
            self.world_config.sub_resolution,
        );

        let mut chunk = Chunk::new(self.world_config.chunk_size, self.world_config.sub_resolution);

        generator.generate_into_chunk(&mut chunk, coord);

        // Future hook for real topographic data:
        // if let Some(topographic_data) = load_dem_for_coord(coord) {
        //     generator.override_with_topographic(&mut chunk, topographic_data);
        // }

        chunk
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

        let mut chunks_to_save = Vec::new();

        for (coord, compressed) in &self.compressed_cache {
            let serialized = SerializedChunk {
                coord: *coord,
                blocks: compressed.clone(),
            };
            chunks_to_save.push(serialized);
        }

        let path = format!("{}/world.dat", world_dir);
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        bincode::serialize_into(&mut writer, &chunks_to_save)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(())
    }

    pub fn load_world(&mut self) -> std::io::Result<()> {
        let path = format!("worlds/{}/world.dat", self.world_config.world_name);
        let file = File::open(path)?;
        let chunks: Vec<SerializedChunk> = bincode::deserialize_from(file)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        for serialized in chunks {
            let mut chunk = Chunk::new(
                self.world_config.chunk_size,
                self.world_config.sub_resolution,
            );

            for compressed in serialized.blocks {
                let (x, y, z) = compressed.position;
                let mut block = BlockData::new(compressed.id);

                for sub in compressed.sub_blocks {
                    block.grid.insert(
                        (sub.local_pos.0, sub.local_pos.1, sub.local_pos.2),
                        BlockData {
                            id: sub.id,
                            integrity: sub.integrity,
                            orientation: sub.orientation,
                            ..Default::default()
                        },
                    );
                }

                chunk.blocks[x][y][z] = Some(block);
            }

            self.chunks.insert(serialized.coord, chunk);
            self.compressed_cache.insert(serialized.coord, serialized.blocks);
        }

        Ok(())
    }

    pub fn get_block_at(&self, world_pos: Vec3) -> Option<(&BlockData, IVec3)> {
        let chunk_size = self.world_config.chunk_size as f32;
        let chunk_coord = IVec3::new(
            (world_pos.x / chunk_size).floor() as i32,
            (world_pos.y / chunk_size).floor() as i32,
            (world_pos.z / chunk_size).floor() as i32,
        );

        if let Some(chunk) = self.chunks.get(&chunk_coord) {
            let local_x = (world_pos.x % chunk_size).floor() as usize;
            let local_y = (world_pos.y % chunk_size).floor() as usize;
            let local_z = (world_pos.z % chunk_size).floor() as usize;

            chunk.blocks[local_x][local_y][local_z]
                .as_ref()
                .map(|block| (block, chunk_coord))
        } else {
            None
        }
    }

    pub fn get_subblock_at(&self, world_pos: Vec3) -> Option<(&BlockData, IVec3)> {
        let (block, chunk_coord) = self.get_block_at(world_pos)?;
        let sub_size = 1.0 / self.world_config.sub_resolution as f32;

        let local_pos = world_pos - Vec3::new(
            chunk_coord.x as f32 * self.world_config.chunk_size as f32,
            chunk_coord.y as f32 * self.world_config.chunk_size as f32,
            chunk_coord.z as f32 * self.world_config.chunk_size as f32,
        );

        let sx = (local_pos.x / sub_size).floor() as u8;
        let sy = (local_pos.y / sub_size).floor() as u8;
        let sz = (local_pos.z / sub_size).floor() as u8;

        block.grid.get(&(sx, sy, sz)).map(|sub| (sub, chunk_coord))
    }
}
