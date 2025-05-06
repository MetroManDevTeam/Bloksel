use crate::world::{BlockFacing, BlockId, BlockOrientation, ConnectedDirections};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Block {
    pub id: BlockId,
    pub sub_blocks: HashMap<(u8, u8, u8), SubBlock>,
    pub resolution: u8,
}

#[derive(Debug, Clone)]
pub struct SubBlock {
    pub id: BlockId,
    pub metadata: u8,
    pub facing: BlockFacing,
    pub orientation: BlockOrientation,
    pub connections: ConnectedDirections,
}

impl Block {
    pub fn new(id: BlockId, resolution: u8) -> Self {
        Self {
            id,
            sub_blocks: HashMap::new(),
            resolution,
        }
    }

    pub fn place_sub_block(&mut self, x: u8, y: u8, z: u8, sub_block: SubBlock) {
        self.sub_blocks.insert((x, y, z), sub_block);
    }

    pub fn get_sub_block(&self, x: u8, y: u8, z: u8) -> Option<&SubBlock> {
        self.sub_blocks.get(&(x, y, z))
    }

    pub fn remove_sub_block(&mut self, x: u8, y: u8, z: u8) {
        self.sub_blocks.remove(&(x, y, z));
    }
}
