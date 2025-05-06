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
pub struct BlockId {
    pub base_id: u32,
    pub variation: u16,
    pub color_id: u16,
}

impl BlockId {
    pub fn new(id: u16) -> Self {
        Self {
            base_id: id as u32,
            variation: 0,
            color_id: 0,
        }
    }

    pub fn to_block(self) -> Block {
        Block::new(self)
    }

    pub const AIR: BlockId = BlockId {
        base_id: 0,
        variation: 0,
        color_id: 0,
    };

    pub fn with_variation(base_id: u32, variation: u16) -> Self {
        Self {
            base_id,
            variation,
            color_id: 0,
        }
    }

    pub fn with_color(base_id: u32, color_id: u16) -> Self {
        Self {
            base_id,
            variation: 0,
            color_id,
        }
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

        Ok(Self {
            base_id,
            variation,
            color_id,
        })
    }

    pub fn to_combined(&self) -> u64 {
        ((self.base_id as u64) << 32) | ((self.variation as u64) << 16) | (self.color_id as u64)
    }

    pub fn is_colored(&self) -> bool {
        self.color_id != 0
    }
}

impl Display for BlockId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base_id)?;

        if self.variation != 0 || self.color_id != 0 {
            write!(f, ":{}", self.variation)?;
        }

        if self.color_id != 0 {
            write!(f, ":C{}", self.color_id)?;
        }

        Ok(())
    }
}

impl From<u16> for BlockId {
    fn from(id: u16) -> Self {
        Self::new(id)
    }
}

impl From<u32> for BlockId {
    fn from(base_id: u32) -> Self {
        Self {
            base_id,
            variation: 0,
            color_id: 0,
        }
    }
}

impl From<BlockId> for u16 {
    fn from(id: BlockId) -> Self {
        id.base_id as u16
    }
}

impl From<BlockId> for u32 {
    fn from(id: BlockId) -> Self {
        id.base_id
    }
}

impl From<BlockId> for u64 {
    fn from(id: BlockId) -> Self {
        id.to_combined()
    }
}

impl From<u64> for BlockId {
    fn from(combined: u64) -> Self {
        Self {
            base_id: (combined >> 32) as u32,
            variation: ((combined >> 16) & 0xFFFF) as u16,
            color_id: (combined & 0xFFFF) as u16,
        }
    }
}

impl From<i32> for BlockId {
    fn from(value: i32) -> Self {
        BlockId {
            base_id: value as u16,
            variation: 0,
            color_id: 0,
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubBlock {
    pub id: BlockId,
    pub metadata: u8,
    pub facing: BlockFacing,
    pub orientation: BlockOrientation,
    pub connections: ConnectedDirections,
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

impl Block {
    pub fn get_material(&self, registry: &BlockRegistry) -> BlockMaterial {
        let primary_id = self.get_primary_id();
        registry.get_material(primary_id).unwrap_or_default()
    }

    pub fn get_physics(&self, registry: &BlockRegistry) -> BlockPhysics {
        let primary_id = self.get_primary_id();
        registry.get_physics(primary_id)
    }

    pub fn place_sub_block(&mut self, pos: (u8, u8, u8), sub: SubBlock) -> Option<SubBlock> {
        self.sub_blocks.insert(pos, sub)
    }

    pub fn get_sub_block(&self, pos: (u8, u8, u8)) -> Option<&SubBlock> {
        self.sub_blocks.get(&pos)
    }
}

impl SubBlock {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            metadata: 0,
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

pub struct BlockRegistry {
    // Primary storage
    blocks: HashMap<BlockId, BlockDefinition>,
    name_to_id: HashMap<String, BlockId>,

    // Indexes
    base_id_to_variants: HashMap<u32, Vec<BlockId>>,
    category_index: HashMap<BlockCategory, HashSet<BlockId>>,

    // Caches
    material_cache: HashMap<BlockId, BlockMaterial>,
    physics_cache: HashMap<BlockId, BlockPhysics>,

    // Texture management
    texture_atlas_indices: HashMap<String, u32>,
    pending_texture_loads: HashSet<String>,
}

impl BlockRegistry {
    pub fn get_block_id(&self, name: &str) -> Option<BlockId> {
        self.name_to_id.get(name).copied()
    }

    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            name_to_id: HashMap::new(),
            base_id_to_variants: HashMap::new(),
            category_index: HashMap::new(),
            material_cache: HashMap::new(),
            physics_cache: HashMap::new(),
            texture_atlas_indices: HashMap::new(),
            pending_texture_loads: HashSet::new(),
        }
    }

    pub fn initialize_default() -> Self {
        let mut registry = Self::new();
        for block in BLOCKS {
            registry.add_block(block.clone()).unwrap();
        }
        registry
    }

    pub fn add_block(&mut self, def: BlockDefinition) -> Result<(), BlockError> {
        if self.blocks.contains_key(&def.id) {
            return Err(BlockError::DuplicateId(def.id));
        }

        if self.name_to_id.contains_key(&def.name) {
            return Err(BlockError::DuplicateName(def.name));
        }

        self.blocks.insert(def.id, def.clone());
        self.name_to_id.insert(def.name.clone(), def.id);

        // Add to category index
        self.category_index
            .entry(def.category)
            .or_insert_with(HashSet::new)
            .insert(def.id);

        // Process variants
        self.process_variants(&def)?;

        // Process color variations
        self.process_color_variations(&def)?;

        // Add to base_id index
        self.base_id_to_variants
            .entry(def.id.base_id)
            .or_insert_with(Vec::new)
            .push(def.id);

        // Cache material and physics
        self.material_cache.insert(def.id, def.material.clone());
        self.physics_cache.insert(def.id, def.flags.into());

        Ok(())
    }

    fn process_variants(&mut self, base_def: &BlockDefinition) -> Result<(), BlockError> {
        for variant in &base_def.variations {
            let variant_id = BlockId::with_variation(base_def.id.base_id, variant.id);
            let mut variant_def = base_def.clone();
            variant_def.id = variant_id;
            variant_def.name = format!("{}:{}", base_def.name, variant.name);

            // Apply texture overrides
            for (face, texture) in &variant.texture_overrides {
                variant_def.texture_faces.insert(*face, texture.clone());
            }

            // Apply material modifiers
            variant_def
                .material
                .apply_modifiers(&variant.material_modifiers);

            self.add_block(variant_def)?;
        }
        Ok(())
    }

    fn process_color_variations(&mut self, base_def: &BlockDefinition) -> Result<(), BlockError> {
        for color in &base_def.color_variations {
            let color_id = BlockId::with_color(base_def.id.base_id, color.id);
            let mut color_def = base_def.clone();
            color_def.id = color_id;
            color_def.name = format!("{}:C{}", base_def.name, color.name);

            // Apply color tint
            color_def
                .material
                .apply_tint(color.color, &base_def.tint_settings);

            // Apply material modifiers
            color_def
                .material
                .apply_modifiers(&color.material_modifiers);

            self.add_block(color_def)?;
        }
        Ok(())
    }

    pub fn get(&self, id: BlockId) -> Option<&BlockDefinition> {
        self.blocks.get(&id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&BlockDefinition> {
        self.name_to_id.get(name).and_then(|id| self.get(*id))
    }

    pub fn get_base(&self, base_id: u32) -> Option<&BlockDefinition> {
        self.get(BlockId::from(base_id))
    }

    pub fn get_variant(&self, id: BlockId) -> Option<&BlockVariant> {
        self.get(id)
            .and_then(|def| def.variations.iter().find(|v| v.id == id.variation))
    }

    pub fn get_color_variant(&self, id: BlockId) -> Option<&ColorVariant> {
        self.get(id)
            .and_then(|def| def.color_variations.iter().find(|c| c.id == id.color_id))
    }

    pub fn get_material(&self, id: BlockId) -> Option<BlockMaterial> {
        self.material_cache.get(&id).cloned()
    }

    pub fn get_physics(&self, id: BlockId) -> BlockPhysics {
        self.physics_cache.get(&id).copied().unwrap_or_default()
    }

    pub fn get_all_variants(&self, base_id: u32) -> Vec<BlockId> {
        self.base_id_to_variants
            .get(&base_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_all_colors(&self, base_id: u32) -> Vec<(BlockId, Vec4)> {
        self.get_all_variants(base_id)
            .into_iter()
            .filter_map(|id| {
                self.get_color_variant(id)
                    .map(|color| (id, Vec4::from_slice(&color.color)))
            })
            .collect()
    }

    pub fn get_by_category(&self, category: BlockCategory) -> Vec<BlockId> {
        self.category_index
            .get(&category)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect()
    }

    pub fn rebuild_texture_atlas(&mut self) {
        self.texture_atlas_indices.clear();
        self.pending_texture_loads.clear();

        for def in self.blocks.values() {
            for texture in def.texture_faces.values() {
                if !self.texture_atlas_indices.contains_key(texture) {
                    self.pending_texture_loads.insert(texture.clone());
                }
            }
        }
    }

    pub fn get_texture_index(&self, path: &str) -> Option<u32> {
        self.texture_atlas_indices.get(path).copied()
    }

    pub fn serialize(&self) -> Result<Vec<u8>, BlockError> {
        bincode::serialize(self).map_err(|_| BlockError::SerializationError)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, BlockError> {
        bincode::deserialize(data).map_err(|_| BlockError::DeserializationError)
    }

    fn rebuild_caches(&mut self) {
        self.material_cache.clear();
        self.physics_cache.clear();

        for def in self.blocks.values() {
            self.material_cache.insert(def.id, def.material.clone());
            self.physics_cache.insert(def.id, def.flags.into());
        }
    }
}

impl BlockMaterial {
    pub fn apply_tint(&mut self, color: [f32; 4], settings: &TintSettings) {
        let [r, g, b, a] = color;
        let color = [r, g, b];
        // ... rest of the method ...
    }
}
