use crate::world::block::Block;
use crate::world::block_error::BlockError;
use crate::world::block_facing::BlockFacing;
use crate::world::block_material::{BlockMaterial, MaterialModifiers, TintSettings};
use crate::world::block_orientation::BlockOrientation;
use crate::world::block_tech::{BlockFlags, BlockPhysics};
use crate::world::block_visual::{BlockVisual, ConnectedDirections};
use crate::world::blocks_data::BLOCKS;
use glam::Vec4;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockData {
    pub id: BlockId,
    pub metadata: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockCategory {
    Solid,
    Liquid,
    Gas,
    Flora,
    Transparent,
    Ore,
    Decorative,
    Mechanical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub u16);

impl BlockId {
    pub const fn new(base_id: u32, variation: u32, color_id: u32) -> Self {
        Self((base_id << 16) | ((variation & 0xFF) << 8) | (color_id & 0xFF) as u16)
    }

    pub fn base_id(&self) -> u32 {
        (self.0 as u32) >> 16
    }

    pub fn variation(&self) -> u32 {
        ((self.0 as u32) >> 8) & 0xFF
    }

    pub fn color_id(&self) -> u32 {
        self.0 as u32 & 0xFF
    }

    pub fn get_id(&self) -> u32 {
        self.0 as u32
    }

    pub fn to_block(self) -> Block {
        Block::new(self)
    }

    pub const AIR: BlockId = BlockId(0);

    pub fn with_variation(base_id: u32, variation: u16) -> Self {
        Self((base_id << 16 | variation as u32) as u16)
    }

    pub fn with_color(base_id: u32, color_id: u16) -> Self {
        Self((base_id << 16 | color_id as u32) as u16)
    }

    pub fn from_str(s: &str) -> Result<Self, BlockError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            return Err(BlockError::InvalidIdFormat);
        }

        let base_id = parts[0]
            .parse::<u32>()
            .map_err(|_| BlockError::InvalidIdFormat)?;
        let variation = parts[1]
            .parse::<u32>()
            .map_err(|_| BlockError::InvalidIdFormat)?;
        let color_id = parts[2]
            .parse::<u32>()
            .map_err(|_| BlockError::InvalidIdFormat)?;

        Ok(Self::new(base_id, variation, color_id))
    }

    pub fn to_combined(&self) -> u64 {
        ((self.0 as u64) << 32) | ((self.0 as u64) << 16)
    }

    pub fn is_colored(&self) -> bool {
        self.0 != 0
    }
}

impl Display for BlockId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0 >> 16)?;

        if self.0 & 0xFFFF != 0 {
            write!(f, ":{}", self.0 & 0xFFFF)?;
        }

        Ok(())
    }
}

impl From<BlockId> for u32 {
    fn from(id: BlockId) -> Self {
        id.0 as u32
    }
}

impl From<u32> for BlockId {
    fn from(id: u32) -> Self {
        Self(id as u16)
    }
}

impl From<BlockId> for u64 {
    fn from(id: BlockId) -> Self {
        id.to_combined()
    }
}

impl From<u64> for BlockId {
    fn from(combined: u64) -> Self {
        Self(combined as u16)
    }
}

impl From<i32> for BlockId {
    fn from(value: i32) -> Self {
        Self(value as u16)
    }
}

impl FromStr for BlockId {
    type Err = BlockError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            return Err(BlockError::InvalidIdFormat);
        }

        let base_id = parts[0]
            .parse::<u32>()
            .map_err(|_| BlockError::InvalidIdFormat)?;
        let variation = parts[1]
            .parse::<u32>()
            .map_err(|_| BlockError::InvalidIdFormat)?;
        let color_id = parts[2]
            .parse::<u32>()
            .map_err(|_| BlockError::InvalidIdFormat)?;

        Ok(Self::new(base_id, variation, color_id))
    }
}

impl Default for BlockId {
    fn default() -> Self {
        Self(0)
    }
}

impl From<BlockId> for u16 {
    fn from(id: BlockId) -> u16 {
        id.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDefinition {
    pub id: BlockId,
    pub name: String,
    pub category: BlockCategory,
    #[serde(default)]
    pub default_facing: BlockFacing,
    #[serde(default)]
    pub default_orientation: BlockOrientation,
    #[serde(default)]
    pub connects_to: HashSet<BlockCategory>,
    #[serde(default)]
    pub texture_faces: HashMap<BlockFacing, String>,
    pub material: BlockMaterial,
    #[serde(default)]
    pub flags: BlockFlags,
    #[serde(default)]
    pub physics: BlockPhysics,
    #[serde(default)]
    pub variations: Vec<BlockVariant>,
    #[serde(default)]
    pub color_variations: Vec<ColorVariant>,
    #[serde(default)]
    pub tint_settings: TintSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockVariant {
    pub id: u16,
    pub name: String,
    #[serde(default)]
    pub texture_overrides: HashMap<BlockFacing, String>,
    #[serde(default)]
    pub material_modifiers: MaterialModifiers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorVariant {
    pub id: u16,
    pub name: String,
    pub color: [f32; 4],
    #[serde(default)]
    pub material_modifiers: MaterialModifiers,
}

#[derive(Debug, Clone)]
pub struct BlockRegistry {
    blocks: HashMap<String, BlockId>,
    materials: HashMap<BlockId, BlockMaterial>,
    physics: HashMap<BlockId, BlockPhysics>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            materials: HashMap::new(),
            physics: HashMap::new(),
        }
    }

    pub fn register_block(
        &mut self,
        name: &str,
        id: BlockId,
        material: BlockMaterial,
        physics: BlockPhysics,
    ) {
        self.blocks.insert(name.to_string(), id);
        self.materials.insert(id, material);
        self.physics.insert(id, physics);
    }

    pub fn get_by_name(&self, name: &str) -> Option<BlockId> {
        self.blocks.get(name).copied()
    }

    pub fn get_material(&self, id: BlockId) -> Option<&BlockMaterial> {
        self.materials.get(&id)
    }

    pub fn get_physics(&self, id: BlockId) -> Option<&BlockPhysics> {
        self.physics.get(&id)
    }
}
