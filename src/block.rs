// block.rs - Final Implementation with Variation Support

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};
use thiserror::Error;
use bitflags::bitflags;
use crate::chunk_renderer::BlockMaterial;

// ========================
// Core Type Definitions
// ========================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockFacing {
    North,
    South,
    East,
    West,
    Up,
    Down,
    None,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockOrientation {
    Wall,
    Floor,
    Ceiling,
    Corner,
    Edge,
    Custom(u8),
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct ConnectedDirections: u8 {
        const NORTH = 0b00000001;
        const SOUTH = 0b00000010;
        const EAST  = 0b00000100;
        const WEST  = 0b00001000;
        const UP    = 0b00010000;
        const DOWN  = 0b00100000;
        const ALL   = Self::NORTH.bits | Self::SOUTH.bits | Self::EAST.bits | Self::WEST.bits | Self::UP.bits | Self::DOWN.bits;
    }
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

// Remove duplicate BlockMaterial definition and use this single version
#[derive(Clone, Serialize, Deserialize)]
pub struct BlockMaterial {
    pub id: u16,
    pub name: String,
    pub albedo: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: [f32; 3],
    pub texture_path: Option<String>,
    pub normal_map_path: Option<String>,
}

// ========================
// Block Identification
// ========================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId {
    pub base_id: u32,
    pub variation: u16,
}

impl BlockId {
    pub fn new(base_id: u32) -> Self {
        Self { base_id, variation: 0 }
    }

    pub fn with_variation(base_id: u32, variation: u16) -> Self {
        Self { base_id, variation }
    }

    pub fn from_combined(combined: u64) -> Self {
        Self {
            base_id: (combined >> 16) as u32,
            variation: (combined & 0xFFFF) as u16,
        }
    }

    pub fn to_combined(&self) -> u64 {
        ((self.base_id as u64) << 16) | (self.variation as u64)
    }
}

impl Display for BlockId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.variation == 0 {
            write!(f, "{}", self.base_id)
        } else {
            write!(f, "{}:{}", self.base_id, self.variation)
        }
    }
}

impl From<u32> for BlockId {
    fn from(base_id: u32) -> Self {
        Self::new(base_id)
    }
}

// ========================
// Block Definitions
// ========================

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
    #[serde(default)]
    pub material: BlockMaterial,
    #[serde(default)]
    pub flags: BlockFlags,
    #[serde(default)]
    pub variations: Vec<BlockVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockVariant {
    pub id: u16,
    pub name: String,
    #[serde(default)]
    pub texture_overrides: HashMap<BlockFacing, String>,
    #[serde(default)]
    pub material_modifiers: MaterialModifiers,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaterialModifiers {
    pub albedo_factor: Option<[f32; 3]>,
    pub roughness_offset: Option<f32>,
    pub metallic_offset: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlockFlags {
    pub transparent: bool,
    pub emissive: bool,
    pub flammable: bool,
    pub conductive: bool,
    pub magnetic: bool,
    pub liquid: bool,
    pub climbable: bool,
}

// ========================
// Block Instance System
// ========================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubBlock {
    pub id: BlockId,
    pub metadata: u8,
    pub facing: BlockFacing,
    pub orientation: BlockOrientation,
    pub connections: ConnectedDirections,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub sub_blocks: HashMap<(u8, u8, u8), SubBlock>,
    pub resolution: u8,
    pub current_connections: ConnectedDirections,
}

// ========================
// Implementation
// ========================

impl BlockFacing {
    pub fn opposite(&self) -> Self {
        match self {
            BlockFacing::North => BlockFacing::South,
            BlockFacing::South => BlockFacing::North,
            BlockFacing::East => BlockFacing::West,
            BlockFacing::West => BlockFacing::East,
            BlockFacing::Up => BlockFacing::Down,
            BlockFacing::Down => BlockFacing::Up,
            _ => BlockFacing::None,
        }
    }
}

impl Block {
    pub fn get_primary_id(&self) -> BlockId {
        self.sub_blocks.values().next().map(|sb| sb.id).unwrap_or_else(|| BlockId::new(0))
    }

    pub fn get_material(&self, registry: &BlockRegistry) -> BlockMaterial {
        let primary = self.get_primary_id();
        let mut material = registry.get_material(primary).unwrap_or_default();
        
        if let Some(variant) = registry.get_variant(primary) {
            material.apply_modifiers(&variant.material_modifiers);
        }
        
        material
    }
}

// ========================
// Block Registry
// ========================

#[derive(Debug, Clone)]
pub struct BlockRegistry {
    blocks: HashMap<BlockId, BlockDefinition>,
    name_to_id: HashMap<String, BlockId>,
    base_id_to_variants: HashMap<u32, Vec<BlockId>>,
}

impl BlockRegistry {
    pub fn initialize_default() -> Self {
        let mut registry = Self {
            blocks: HashMap::new(),
            name_to_id: HashMap::new(),
            base_id_to_variants: HashMap::new(),
        };

        // Include the generated block data
        let blocks_data = include!("blocks_data.rs");
        for def in blocks_data {
            registry.add_block(def);
        }

        registry
    }

    pub fn add_block(&mut self, def: BlockDefinition) {
        let id = def.id;
        
        // Add main definition
        self.blocks.insert(id, def.clone());
        self.name_to_id.insert(def.name.clone(), id);
        
        // Handle variations
        self.base_id_to_variants.entry(id.base_id)
            .or_default()
            .push(id);
        
        for variant in def.variations {
            let variant_id = BlockId::with_variation(id.base_id, variant.id);
            let mut variant_def = def.clone();
            variant_def.id = variant_id;
            
            // Apply variant overrides
            for (face, tex) in variant.texture_overrides {
                variant_def.texture_faces.insert(face, tex);
            }
            
            self.blocks.insert(variant_id, variant_def);
        }
    }

    pub fn get(&self, id: BlockId) -> Option<&BlockDefinition> {
        self.blocks.get(&id)
    }

    pub fn get_base(&self, base_id: u32) -> Option<&BlockDefinition> {
        self.get(BlockId::new(base_id))
    }

    pub fn get_variant(&self, id: BlockId) -> Option<&BlockVariant> {
        self.get(id.base_id)?
            .variations
            .iter()
            .find(|v| v.id == id.variation)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&BlockDefinition> {
        self.name_to_id.get(name).and_then(|id| self.get(*id))
    }
}

// ========================
// Error Handling
// ========================

#[derive(Error, Debug)]
pub enum BlockError {
    #[error("Invalid block ID format")]
    InvalidIdFormat,
    #[error("Block not found: {0}")]
    BlockNotFound(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
}
