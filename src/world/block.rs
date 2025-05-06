use crate::world::block_facing::BlockFacing;
use crate::world::{BlockId, BlockOrientation, ConnectedDirections};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Block {
    pub id: u16,
    pub orientation: BlockOrientation,
    pub facing: BlockFacing,
    pub sub_blocks: HashMap<(u8, u8, u8), SubBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubBlock {
    pub id: BlockId,
    pub metadata: u8,
    pub facing: BlockFacing,
    pub orientation: BlockOrientation,
    pub connections: ConnectedDirections,
}

impl Block {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            orientation: BlockOrientation::None,
            facing: BlockFacing::None,
            sub_blocks: HashMap::new(),
        }
    }

    pub fn with_orientation(mut self, orientation: BlockOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn with_facing(mut self, facing: BlockFacing) -> Self {
        self.facing = facing;
        self
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
