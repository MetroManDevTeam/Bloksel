use crate::world::block_facing::BlockFacing;
use crate::world::block_id::BlockId;
use crate::world::block_orientation::BlockOrientation;
use crate::world::block_visual::ConnectedDirections;
use crate::world::{BlockMaterial, BlockPhysics, BlockRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub orientation: BlockOrientation,
    pub facing: BlockFacing,
    pub sub_blocks: HashMap<(u8, u8, u8), SubBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            facing: BlockFacing::None,
            sub_blocks: HashMap::new(),
        }
    }

    pub fn get_material(&self, registry: &BlockRegistry) -> BlockMaterial {
        registry.get_material(self.id).unwrap_or_default()
    }

    pub fn get_physics(&self, registry: &BlockRegistry) -> BlockPhysics {
        registry.get_physics(self.id)
    }

    pub fn place_sub_block(&mut self, pos: (u8, u8, u8), sub_block: SubBlock) {
        self.sub_blocks.insert(pos, sub_block);
    }

    pub fn remove_sub_block(&mut self, pos: &(u8, u8, u8)) -> Option<SubBlock> {
        self.sub_blocks.remove(pos)
    }

    pub fn get_sub_block(&self, pos: &(u8, u8, u8)) -> Option<&SubBlock> {
        self.sub_blocks.get(pos)
    }

    pub fn get_sub_block_mut(&mut self, pos: &(u8, u8, u8)) -> Option<&mut SubBlock> {
        self.sub_blocks.get_mut(pos)
    }

    pub fn has_sub_blocks(&self) -> bool {
        !self.sub_blocks.is_empty()
    }

    pub fn get_primary_id(&self) -> BlockId {
        self.id
    }
}

impl SubBlock {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            facing: BlockFacing::None,
            orientation: BlockOrientation::None,
            connections: ConnectedDirections::default(),
        }
    }

    pub fn update_connections(&mut self, directions: ConnectedDirections) {
        self.connections = directions;
    }

    pub fn has_connection(&self, direction: ConnectedDirections) -> bool {
        self.connections.contains(direction)
    }

    pub fn set_facing(&mut self, facing: BlockFacing) {
        self.facing = facing;
    }

    pub fn set_orientation(&mut self, orientation: BlockOrientation) {
        self.orientation = orientation;
    }
}
