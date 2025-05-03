use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;
use glam::{IVec3, Vec3};
use crate::core::block::{Block, BlockError, BlockProperties};
use crate::physics::collision::AABB;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChunkError {
    #[error("Block position out of bounds")]
    OutOfBounds,
    #[error("Block operation failed: {0}")]
    BlockError(#[from] BlockError),
    #[error("Chunk not loaded")]
    NotLoaded,
    #[error("Chunk already exists")]
    AlreadyExists,
}

pub const CHUNK_SIZE: i32 = 16;
pub const CHUNK_HEIGHT: i32 = 256;
pub const CHUNK_VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_HEIGHT) as usize;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub position: (i32, i32), // X,Z coordinates
    blocks: Arc<RwLock<HashMap<IVec3, Block>>>,
    block_count: usize,
    dirty: bool,
    light_map: Box<[u8; CHUNK_VOLUME]>,
    height_map: Box<[i16; CHUNK_SIZE as usize * CHUNK_SIZE as usize]>,
    mesh_dirty: bool,
    collision_dirty: bool,
}

impl Chunk {
    pub fn new(x: i32, z: i32) -> Self {
        Self {
            position: (x, z),
            blocks: Arc::new(RwLock::new(HashMap::with_capacity(CHUNK_VOLUME))),
            block_count: 0,
            dirty: true,
            light_map: Box::new([0; CHUNK_VOLUME]),
            height_map: Box::new([0; CHUNK_SIZE as usize * CHUNK_SIZE as usize]),
            mesh_dirty: true,
            collision_dirty: true,
        }
    }

    pub fn set_block(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
    ) -> Result<Option<Block>, ChunkError> {
        if !self.in_bounds(x, y, z) {
            return Err(ChunkError::OutOfBounds);
        }

        let pos = IVec3::new(x, y, z);
        let mut blocks = self.blocks.write();
        let old_block = blocks.insert(pos, block);

        if old_block.is_none() {
            self.block_count += 1;
        }

        self.dirty = true;
        self.mesh_dirty = true;
        self.collision_dirty = true;

        // Update heightmap
        if y >= self.height_map[(z * CHUNK_SIZE + x) as usize] as i32 {
            self.height_map[(z * CHUNK_SIZE + x) as usize] = y as i16;
        }

        Ok(old_block)
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<Block> {
        if !self.in_bounds(x, y, z) {
            return None;
        }
        self.blocks.read().get(&IVec3::new(x, y, z)).cloned()
    }

    pub fn remove_block(&mut self, x: i32, y: i32, z: i32) -> Result<Option<Block>, ChunkError> {
        if !self.in_bounds(x, y, z) {
            return Err(ChunkError::OutOfBounds);
        }

        let pos = IVec3::new(x, y, z);
        let mut blocks = self.blocks.write();
        let removed = blocks.remove(&pos);

        if removed.is_some() {
            self.block_count -= 1;
            self.dirty = true;
            self.mesh_dirty = true;
            self.collision_dirty = true;

            // Recalculate heightmap if needed
            if y == self.height_map[(z * CHUNK_SIZE + x) as usize] as i32 {
                self.recalculate_heightmap_column(x, z);
            }
        }

        Ok(removed)
    }

    fn recalculate_heightmap_column(&mut self, x: i32, z: i32) {
        let mut max_y = -1;
        let blocks = self.blocks.read();

        for y in (0..CHUNK_HEIGHT).rev() {
            let pos = IVec3::new(x, y, z);
            if blocks.contains_key(&pos) {
                max_y = y;
                break;
            }
        }

        self.height_map[(z * CHUNK_SIZE + x) as usize] = max_y as i16;
    }

    pub fn get_blocks(&self) -> Vec<Block> {
        self.blocks.read().values().cloned().collect()
    }

    pub fn get_blocks_mut(&mut self) -> Vec<Block> {
        self.blocks.write().values().cloned().collect()
    }

    pub fn get_collision_boxes(&self) -> Vec<AABB> {
        self.blocks
            .read()
            .values()
            .filter(|b| b.is_solid())
            .map(|b| {
                let (min, max) = b.get_collision_box();
                AABB::new(min, max)
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.block_count == 0
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn is_mesh_dirty(&self) -> bool {
        self.mesh_dirty
    }

    pub fn is_collision_dirty(&self) -> bool {
        self.collision_dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn mark_mesh_clean(&mut self) {
        self.mesh_dirty = false;
    }

    pub fn mark_collision_clean(&mut self) {
        self.collision_dirty = false;
    }

    pub fn get_height(&self, x: i32, z: i32) -> i32 {
        if x < 0 || x >= CHUNK_SIZE || z < 0 || z >= CHUNK_SIZE {
            return 0;
        }
        self.height_map[(z * CHUNK_SIZE + x) as usize] as i32
    }

    pub fn get_light(&self, x: i32, y: i32, z: i32) -> u8 {
        if !self.in_bounds(x, y, z) {
            return 0;
        }
        self.light_map[(y * CHUNK_SIZE * CHUNK_SIZE + z * CHUNK_SIZE + x) as usize]
    }

    pub fn set_light(&mut self, x: i32, y: i32, z: i32, value: u8) {
        if self.in_bounds(x, y, z) {
            self.light_map[(y * CHUNK_SIZE * CHUNK_SIZE + z * CHUNK_SIZE + x) as usize] = value;
            self.dirty = true;
        }
    }

    pub fn recalculate_lighting(&mut self) {
        // TODO: Implement proper lighting propagation
        for y in 0..CHUNK_HEIGHT {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let pos = IVec3::new(x, y, z);
                    if let Some(block) = self.blocks.read().get(&pos) {
                        self.light_map[(y * CHUNK_SIZE * CHUNK_SIZE + z * CHUNK_SIZE + x) as usize] =
                            block.light_level;
                    } else {
                        self.light_map[(y * CHUNK_SIZE * CHUNK_SIZE + z * CHUNK_SIZE + x) as usize] = 0;
                    }
                }
            }
        }
        self.dirty = true;
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && x < CHUNK_SIZE && y >= 0 && y < CHUNK_HEIGHT && z >= 0 && z < CHUNK_SIZE
    }

    pub fn world_pos_to_chunk_pos(world_pos: Vec3) -> (IVec3, IVec3) {
        let chunk_x = (world_pos.x / CHUNK_SIZE as f32).floor() as i32;
        let chunk_z = (world_pos.z / CHUNK_SIZE as f32).floor() as i32;
        let local_x = (world_pos.x - (chunk_x * CHUNK_SIZE) as f32) as i32;
        let local_y = world_pos.y as i32;
        let local_z = (world_pos.z - (chunk_z * CHUNK_SIZE) as f32) as i32;

        (IVec3::new(chunk_x, 0, chunk_z), IVec3::new(local_x, local_y, local_z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::block::{Block, BlockProperties};
    use std::path::PathBuf;

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
    fn test_chunk_operations() {
        let mut chunk = Chunk::new(0, 0);
        let block = test_block();

        // Test set and get
        assert!(chunk.set_block(0, 0, 0, block.clone()).unwrap().is_none());
        assert_eq!(chunk.get_block(0, 0, 0).unwrap().id, block.id);
        assert_eq!(chunk.block_count, 1);

        // Test bounds checking
        assert!(matches!(
            chunk.set_block(-1, 0, 0, block.clone()),
            Err(ChunkError::OutOfBounds)
        ));

        // Test remove
        assert!(chunk.remove_block(0, 0, 0).unwrap().is_some());
        assert!(chunk.get_block(0, 0, 0).is_none());
        assert_eq!(chunk.block_count, 0);
    }

    #[test]
    fn test_heightmap() {
        let mut chunk = Chunk::new(0, 0);
        let block = test_block();

        chunk.set_block(5, 10, 5, block.clone()).unwrap();
        assert_eq!(chunk.get_height(5, 5), 10);

        chunk.remove_block(5, 10, 5).unwrap();
        assert_eq!(chunk.get_height(5, 5), -1);
    }
}
