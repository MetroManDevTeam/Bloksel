use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use glam::{IVec2, IVec3};
use crate::core::{Block, Chunk, ChunkError};
use crate::physics::collision::AABB;
use crate::terrain::generator::TerrainGenerator;
use thiserror::Error;
use rayon::prelude::*;

#[derive(Error, Debug)]
pub enum WorldError {
    #[error("Chunk error: {0}")]
    ChunkError(#[from] ChunkError),
    #[error("Chunk not loaded")]
    ChunkNotLoaded,
    #[error("Chunk already loaded")]
    ChunkAlreadyLoaded,
    #[error("Position out of world bounds")]
    OutOfBounds,
}

pub struct World {
    chunks: RwLock<HashMap<IVec2, Arc<RwLock<Chunk>>>>,
    generator: Arc<TerrainGenerator>,
    spawn_point: IVec3,
    world_bounds: (IVec2, IVec2), // min and max chunk coords
    loaded_chunks: usize,
    max_loaded_chunks: usize,
    chunk_load_queue: VecDeque<IVec2>,
    chunk_unload_queue: VecDeque<IVec2>,
}

impl World {
    pub fn new(
        seed: u64,
        world_size: i32,
        max_loaded_chunks: usize,
    ) -> Self {
        let generator = Arc::new(TerrainGenerator::new(seed));
        let min = IVec2::splat(-world_size);
        let max = IVec2::splat(world_size);

        Self {
            chunks: RwLock::new(HashMap::new()),
            generator,
            spawn_point: IVec3::new(0, 70, 0),
            world_bounds: (min, max),
            loaded_chunks: 0,
            max_loaded_chunks,
            chunk_load_queue: VecDeque::new(),
            chunk_unload_queue: VecDeque::new(),
        }
    }

    pub fn get_spawn_point(&self) -> IVec3 {
        self.spawn_point
    }

    pub fn set_spawn_point(&mut self, pos: IVec3) {
        self.spawn_point = pos;
    }

    pub fn load_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> Result<(), WorldError> {
        let chunk_pos = IVec2::new(chunk_x, chunk_z);
        
        // Check bounds
        if chunk_x < self.world_bounds.0.x || chunk_x > self.world_bounds.1.x ||
           chunk_z < self.world_bounds.0.y || chunk_z > self.world_bounds.1.y {
            return Err(WorldError::OutOfBounds);
        }

        let mut chunks = self.chunks.write();
        
        // Check if already loaded
        if chunks.contains_key(&chunk_pos) {
            return Err(WorldError::ChunkAlreadyLoaded);
        }

        // Generate new chunk
        let chunk = self.generator.generate_chunk(chunk_x, chunk_z);
        chunks.insert(chunk_pos, Arc::new(RwLock::new(chunk)));
        self.loaded_chunks += 1;

        Ok(())
    }

    pub fn unload_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> Result<(), WorldError> {
        let chunk_pos = IVec2::new(chunk_x, chunk_z);
        let mut chunks = self.chunks.write();

        if chunks.remove(&chunk_pos).is_some() {
            self.loaded_chunks -= 1;
            Ok(())
        } else {
            Err(WorldError::ChunkNotLoaded)
        }
    }

    pub fn get_chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<Arc<RwLock<Chunk>>> {
        self.chunks.read().get(&IVec2::new(chunk_x, chunk_z)).cloned()
    }

    pub fn get_block(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<Block> {
        let (chunk_pos, local_pos) = self.world_to_chunk_pos(world_x, world_y, world_z);
        self.get_chunk(chunk_pos.x, chunk_pos.z)?
            .read()
            .get_block(local_pos.x, local_pos.y, local_pos.z)
    }

    pub fn set_block(
        &self,
        world_x: i32,
        world_y: i32,
        world_z: i32,
        block: Block,
    ) -> Result<Option<Block>, WorldError> {
        let (chunk_pos, local_pos) = self.world_to_chunk_pos(world_x, world_y, world_z);
        let chunk = self.get_chunk(chunk_pos.x, chunk_pos.z)
            .ok_or(WorldError::ChunkNotLoaded)?;

        let mut chunk = chunk.write();
        chunk.set_block(local_pos.x, local_pos.y, local_pos.z, block)
            .map_err(WorldError::from)
    }

    pub fn update_chunks_around(&mut self, center: IVec3, radius: i32) {
        // Unload distant chunks
        self.chunks.read().keys()
            .filter(|&&pos| {
                let dist_x = (pos.x * Chunk::CHUNK_SIZE as i32 - center.x).abs();
                let dist_z = (pos.y * Chunk::CHUNK_SIZE as i32 - center.z).abs();
                dist_x > radius * Chunk::CHUNK_SIZE as i32 || 
                dist_z > radius * Chunk::CHUNK_SIZE as i32
            })
            .for_each(|&pos| {
                self.chunk_unload_queue.push_back(pos);
            });

        // Load nearby chunks
        for x in -radius..=radius {
            for z in -radius..=radius {
                let chunk_x = (center.x / Chunk::CHUNK_SIZE as i32) + x;
                let chunk_z = (center.z / Chunk::CHUNK_SIZE as i32) + z;
                let chunk_pos = IVec2::new(chunk_x, chunk_z);

                if !self.chunks.read().contains_key(&chunk_pos) &&
                   chunk_x >= self.world_bounds.0.x && chunk_x <= self.world_bounds.1.x &&
                   chunk_z >= self.world_bounds.0.y && chunk_z <= self.world_bounds.1.y {
                    self.chunk_load_queue.push_back(chunk_pos);
                }
            }
        }
    }

    pub fn process_queues(&mut self) -> usize {
        let mut processed = 0;

        // Process unload queue
        while let Some(pos) = self.chunk_unload_queue.pop_front() {
            if self.unload_chunk(pos.x, pos.y).is_ok() {
                processed += 1;
            }
            if self.loaded_chunks <= self.max_loaded_chunks / 2 {
                break;
            }
        }

        // Process load queue
        while let Some(pos) = self.chunk_load_queue.pop_front() {
            if self.load_chunk(pos.x, pos.y).is_ok() {
                processed += 1;
            }
            if self.loaded_chunks >= self.max_loaded_chunks {
                break;
            }
        }

        processed
    }

    pub fn get_collision_boxes(
        &self,
        center: IVec3,
        radius: i32,
    ) -> Vec<AABB> {
        let mut boxes = Vec::new();
        let chunk_radius = (radius as f32 / Chunk::CHUNK_SIZE as f32).ceil() as i32;

        for x in -chunk_radius..=chunk_radius {
            for z in -chunk_radius..=chunk_radius {
                let chunk_x = (center.x / Chunk::CHUNK_SIZE as i32) + x;
                let chunk_z = (center.z / Chunk::CHUNK_SIZE as i32) + z;

                if let Some(chunk) = self.get_chunk(chunk_x, chunk_z) {
                    boxes.extend(chunk.read().get_collision_boxes());
                }
            }
        }

        boxes
    }

    pub fn world_to_chunk_pos(&self, x: i32, y: i32, z: i32) -> (IVec2, IVec3) {
        let chunk_x = x.div_euclid(Chunk::CHUNK_SIZE as i32);
        let chunk_z = z.div_euclid(Chunk::CHUNK_SIZE as i32);
        let local_x = x.rem_euclid(Chunk::CHUNK_SIZE as i32);
        let local_z = z.rem_euclid(Chunk::CHUNK_SIZE as i32);

        (IVec2::new(chunk_x, chunk_z), IVec3::new(local_x, y, local_z))
    }

    pub fn chunk_to_world_pos(&self, chunk_pos: IVec2, local_pos: IVec3) -> IVec3 {
        IVec3::new(
            chunk_pos.x * Chunk::CHUNK_SIZE as i32 + local_pos.x,
            local_pos.y,
            chunk_pos.y * Chunk::CHUNK_SIZE as i32 + local_pos.z,
        )
    }

    pub fn get_loaded_chunks(&self) -> Vec<IVec2> {
        self.chunks.read().keys().cloned().collect()
    }

    pub fn save_all(&self, save_path: &str) -> Result<(), std::io::Error> {
        // TODO: Implement proper world saving
        Ok(())
    }

    pub fn load_all(&self, save_path: &str) -> Result<(), std::io::Error> {
        // TODO: Implement proper world loading
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::block::{Block, BlockProperties};
    use std::path::PathBuf;

    fn test_world() -> World {
        World::new(12345, 10, 100)
    }

    fn test_block() -> Block {
        Block {
            id: "1FNM1.0S".to_string(),
            position: (0, 0, 0),
            properties: BlockProperties {
                friction: 0.6,
                restitution: 0.1,
                density: 1.0,
                viscosity: 0.0,
                is_solid: true,
                is_liquid: false,
                is_transparent: false,
                break_time: 1.0,
                light_emission: 0,
                texture_path: PathBuf::from("stone.png"),
                collision_box: (Vec3::ZERO, Vec3::ONE),
            },
            health: 1.0,
            light_level: 0,
            temperature: 293.0,
            last_updated: 0,
        }
    }

    #[test]
    fn test_chunk_loading() {
        let mut world = test_world();
        assert!(world.load_chunk(0, 0).is_ok());
        assert!(matches!(
            world.load_chunk(0, 0),
            Err(WorldError::ChunkAlreadyLoaded)
        ));
    }

    #[test]
    fn test_block_placement() {
        let mut world = test_world();
        world.load_chunk(0, 0).unwrap();
        
        let block = test_block();
        assert!(world.set_block(5, 60, 5, block.clone()).is_ok());
        assert_eq!(world.get_block(5, 60, 5).unwrap().id, block.id);
    }

    #[test]
    fn test_position_conversion() {
        let world = test_world();
        let (chunk_pos, local_pos) = world.world_to_chunk_pos(35, 60, -12);
        assert_eq!(chunk_pos, IVec2::new(2, -1));
        assert_eq!(local_pos, IVec3::new(3, 60, 4));

        let world_pos = world.chunk_to_world_pos(chunk_pos, local_pos);
        assert_eq!(world_pos, IVec3::new(35, 60, -12));
    }
}
