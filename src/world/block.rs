use crate::world::block_facing::BlockFacing;
use crate::world::block_id::BlockId;
use crate::world::block_orientation::BlockOrientation;
use crate::world::block_visual::ConnectedDirections;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Block {
    pub id: BlockId,
    pub orientation: BlockOrientation,
    pub facing: BlockFacing,
    pub sub_blocks: HashMap<(u8, u8, u8), SubBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubBlock {
    pub id: BlockId,
    pub facing: BlockFacing,
    pub orientation: BlockOrientation,
    pub connections: ConnectedDirections,
}

impl Block {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            orientation: BlockOrientation::None,
            facing: BlockFacing::PosY,
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

    pub fn place_sub_block(&mut self, pos: (u8, u8, u8), sub: SubBlock) -> Option<SubBlock> {
        self.sub_blocks.insert(pos, sub)
    }

    pub fn get_sub_block(&self, pos: (u8, u8, u8)) -> Option<&SubBlock> {
        self.sub_blocks.get(&pos)
    }

    pub fn remove_sub_block(&mut self, pos: (u8, u8, u8)) -> Option<SubBlock> {
        self.sub_blocks.remove(&pos)
    }

    pub fn get_primary_id(&self) -> BlockId {
        self.id
    }
}
