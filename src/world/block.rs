use crate::world::block_facing::BlockFacing;
use crate::world::block_id::BlockId;
use crate::world::block_orientation::BlockOrientation;
use crate::world::block_tech::BlockPhysics;
use crate::world::block_visual::ConnectedDirections;
use crate::world::{BlockMaterial, BlockRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub facing: BlockFacing,
    pub orientation: BlockOrientation,
    pub connections: ConnectedDirections,
    pub sub_blocks: HashMap<(u8, u8, u8), SubBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubBlock {
    pub id: u16,
    pub facing: BlockFacing,
    pub orientation: BlockOrientation,
    pub connections: ConnectedDirections,
}

impl Block {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            facing: BlockFacing::default(),
            orientation: BlockOrientation::default(),
            connections: ConnectedDirections::empty(),
            sub_blocks: HashMap::new(),
        }
    }

    pub fn with_facing(mut self, facing: BlockFacing) -> Self {
        self.facing = facing;
        self
    }

    pub fn with_orientation(mut self, orientation: BlockOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn with_connections(mut self, connections: ConnectedDirections) -> Self {
        self.connections = connections;
        self
    }

    pub fn base_id(&self) -> u16 {
        self.id.base_id() as u16
    }

    pub fn variation(&self) -> u8 {
        self.id.variation() as u8
    }

    pub fn color_id(&self) -> u8 {
        self.id.color_id() as u8
    }

    pub fn get_material(&self, registry: &BlockRegistry) -> BlockMaterial {
        registry.get_block_material(self.id).unwrap_or_default()
    }

    pub fn get_physics(&self, registry: &BlockRegistry) -> BlockPhysics {
        registry.get_block_flags(self.id).map_or_else(
            || BlockPhysics::default(),
            |flags| BlockPhysics::from(flags),
        )
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
    pub fn new(id: u16) -> Self {
        Self {
            id,
            facing: BlockFacing::default(),
            orientation: BlockOrientation::default(),
            connections: ConnectedDirections::empty(),
        }
    }

    pub fn with_facing(mut self, facing: BlockFacing) -> Self {
        self.facing = facing;
        self
    }

    pub fn with_orientation(mut self, orientation: BlockOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn with_connections(mut self, connections: ConnectedDirections) -> Self {
        self.connections = connections;
        self
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
