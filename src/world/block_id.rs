use crate::world::block::Block;
use crate::world::block_error::BlockError;
use crate::world::block_facing::BlockFacing;
use crate::world::block_flags::BlockFlags;
use crate::world::block_material::{BlockMaterial, MaterialModifiers, TintSettings};
use crate::world::block_orientation::BlockOrientation;
use crate::world::block_tech::BlockPhysics;
use crate::world::block_visual::ConnectedDirections;
use crate::world::blocks_data::BLOCKS;
use glam::Vec4;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};

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
pub struct BlockId(u32);

impl BlockId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn get_id(&self) -> u32 {
        self.0
    }

    pub fn to_block(self) -> Block {
        Block::new(self)
    }

    pub const AIR: BlockId = BlockId(0);

    pub fn with_variation(base_id: u32, variation: u16) -> Self {
        Self(base_id << 16 | variation as u32)
    }

    pub fn with_color(base_id: u32, color_id: u16) -> Self {
        Self(base_id << 16 | color_id as u32)
    }

    pub fn from_str(s: &str) -> Result<Self, BlockError> {
        let parts: Vec<&str> = s.split(':').collect();
        let base_id = parts[0].parse().map_err(|_| BlockError::InvalidIdFormat)?;

        let mut variation = 0;
        let mut color_id = 0;

        if parts.len() > 1 {
            for part in &parts[1..] {
                if part.starts_with('C') {
                    color_id = part[1..].parse().map_err(|_| BlockError::InvalidIdFormat)?;
                } else {
                    variation = part.parse().map_err(|_| BlockError::InvalidIdFormat)?;
                }
            }
        }

        Ok(Self(base_id << 16 | (variation << 16 | color_id as u32)))
    }

    pub fn to_combined(&self) -> u64 {
        ((self.0 as u64) << 32) | ((self.0 >> 16) as u64)
    }

    pub fn is_colored(&self) -> bool {
        self.0 & 0xFFFF != 0
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
        id.0
    }
}

impl From<u32> for BlockId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<BlockId> for u64 {
    fn from(id: BlockId) -> Self {
        id.to_combined()
    }
}

impl From<u64> for BlockId {
    fn from(combined: u64) -> Self {
        Self(combined as u32)
    }
}

impl From<i32> for BlockId {
    fn from(value: i32) -> Self {
        Self(value as u32)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRegistry {
    blocks: HashMap<BlockId, Block>,
    materials: HashMap<BlockId, BlockMaterial>,
    physics_cache: HashMap<BlockId, BlockPhysics>,
    name_to_id: HashMap<String, BlockId>,
    id_to_name: HashMap<BlockId, String>,
    next_id: u32,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            materials: HashMap::new(),
            physics_cache: HashMap::new(),
            name_to_id: HashMap::new(),
            id_to_name: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn register_block(
        &mut self,
        name: String,
        material: BlockMaterial,
        flags: BlockFlags,
    ) -> Result<BlockId, BlockError> {
        if self.name_to_id.contains_key(&name) {
            return Err(BlockError::DuplicateName(name));
        }

        let id = BlockId::new(self.next_id);
        self.next_id += 1;

        let block = Block::new(id);
        self.blocks.insert(id, block);
        self.materials.insert(id, material);
        self.physics_cache.insert(id, BlockPhysics::from(flags));
        self.name_to_id.insert(name.clone(), id);
        self.id_to_name.insert(id, name);

        Ok(id)
    }

    pub fn get_block(&self, id: BlockId) -> Option<&Block> {
        self.blocks.get(&id)
    }

    pub fn get_block_mut(&mut self, id: BlockId) -> Option<&mut Block> {
        self.blocks.get_mut(&id)
    }

    pub fn get_material(&self, id: BlockId) -> Option<BlockMaterial> {
        self.materials.get(&id).cloned()
    }

    pub fn get_physics(&self, id: BlockId) -> BlockPhysics {
        self.physics_cache.get(&id).cloned().unwrap_or_default()
    }

    pub fn get_block_id(&self, name: &str) -> Option<BlockId> {
        self.name_to_id.get(name).copied()
    }

    pub fn get_block_name(&self, id: BlockId) -> Option<&str> {
        self.id_to_name.get(&id).map(|s| s.as_str())
    }
}

impl BlockMaterial {
    pub fn apply_tint(&mut self, color: [f32; 4], settings: &TintSettings) {
        let [r, g, b, a] = color;
        let color = [r, g, b];
        // ... rest of the method ...
    }
}
