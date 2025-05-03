// block.rs - Complete Implementation with Advanced Tinting System

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};
use thiserror::Error;
use bitflags::bitflags;

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

// ========================
// Material System
// ========================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMaterial {
    pub id: u16,
    pub name: String,
    pub albedo: [f32; 4],        // Base color (RGBA)
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: [f32; 3],
    pub texture_path: Option<String>,
    pub normal_map_path: Option<String>,
    pub occlusion_map_path: Option<String>,
    #[serde(default)]
    pub tintable: bool,
    #[serde(default)]
    pub tint_mask_path: Option<String>,
    #[serde(default)]
    pub vertex_colored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TintSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_tint_strength")]
    pub strength: f32,
    #[serde(default)]
    pub affects_albedo: bool,
    #[serde(default)]
    pub affects_emissive: bool,
    #[serde(default)]
    pub affects_roughness: bool,
    #[serde(default)]
    pub affects_metallic: bool,
    #[serde(default)]
    pub blend_mode: TintBlendMode,
    #[serde(default)]
    pub mask_channel: TintMaskChannel,
}

fn default_tint_strength() -> f32 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TintBlendMode {
    #[default]
    Multiply,
    Overlay,
    Screen,
    Additive,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TintMaskChannel {
    #[default]
    Red,
    Green,
    Blue,
    Alpha,
    All,
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
    pub material: BlockMaterial,
    #[serde(default)]
    pub flags: BlockFlags,
    #[serde(default)]
    pub variations: Vec<BlockVariant>,
    #[serde(default)]
    pub tint_settings: TintSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockVariant {
    pub id: u16,
    pub name: String,
    #[serde(default)]
    pub texture_overrides: HashMap<BlockFacing, String>,
    #[serde(default)]
    pub material_modifiers: MaterialModifiers,
    #[serde(default)]
    pub tint_color: Option<[f32; 4]>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaterialModifiers {
    #[serde(default)]
    pub albedo_factor: Option<[f32; 3]>,
    #[serde(default)]
    pub roughness_offset: Option<f32>,
    #[serde(default)]
    pub metallic_offset: Option<f32>,
    #[serde(default)]
    pub emissive_boost: Option<[f32; 3]>,
    #[serde(default)]
    pub tint_strength: Option<f32>,
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
    pub occludes: bool,
    pub solid: bool,
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

impl BlockMaterial {
    pub fn apply_tint(&mut self, tint: [f32; 4], settings: &TintSettings) {
        if !settings.enabled || settings.strength <= 0.0 {
            return;
        }

        let strength = settings.strength.clamp(0.0, 1.0);
        
        if settings.affects_albedo {
            match settings.blend_mode {
                TintBlendMode::Multiply => {
                    self.albedo[0] *= 1.0 + (tint[0] - 1.0) * strength;
                    self.albedo[1] *= 1.0 + (tint[1] - 1.0) * strength;
                    self.albedo[2] *= 1.0 + (tint[2] - 1.0) * strength;
                    self.albedo[3] *= 1.0 + (tint[3] - 1.0) * strength;
                }
                TintBlendMode::Overlay => {
                    self.albedo[0] = if self.albedo[0] < 0.5 {
                        2.0 * self.albedo[0] * tint[0] * strength
                    } else {
                        1.0 - 2.0 * (1.0 - self.albedo[0]) * (1.0 - tint[0] * strength)
                    }.clamp(0.0, 1.0);
                    self.albedo[1] = if self.albedo[1] < 0.5 {
                        2.0 * self.albedo[1] * tint[1] * strength
                    } else {
                        1.0 - 2.0 * (1.0 - self.albedo[1]) * (1.0 - tint[1] * strength)
                    }.clamp(0.0, 1.0);
                    self.albedo[2] = if self.albedo[2] < 0.5 {
                        2.0 * self.albedo[2] * tint[2] * strength
                    } else {
                        1.0 - 2.0 * (1.0 - self.albedo[2]) * (1.0 - tint[2] * strength)
                    }.clamp(0.0, 1.0);
                    self.albedo[3] = if self.albedo[3] < 0.5 {
                        2.0 * self.albedo[3] * tint[3] * strength
                    } else {
                        1.0 - 2.0 * (1.0 - self.albedo[3]) * (1.0 - tint[3] * strength)
                    }.clamp(0.0, 1.0);
                }
                TintBlendMode::Screen => {
                    self.albedo[0] = 1.0 - (1.0 - self.albedo[0]) * (1.0 - tint[0] * strength);
                    self.albedo[1] = 1.0 - (1.0 - self.albedo[1]) * (1.0 - tint[1] * strength);
                    self.albedo[2] = 1.0 - (1.0 - self.albedo[2]) * (1.0 - tint[2] * strength);
                    self.albedo[3] = 1.0 - (1.0 - self.albedo[3]) * (1.0 - tint[3] * strength);
                }
                TintBlendMode::Additive => {
                    self.albedo[0] = (self.albedo[0] + tint[0] * strength).clamp(0.0, 1.0);
                    self.albedo[1] = (self.albedo[1] + tint[1] * strength).clamp(0.0, 1.0);
                    self.albedo[2] = (self.albedo[2] + tint[2] * strength).clamp(0.0, 1.0);
                    self.albedo[3] = (self.albedo[3] + tint[3] * strength).clamp(0.0, 1.0);
                }
                TintBlendMode::Replace => {
                    self.albedo[0] = self.albedo[0] * (1.0 - strength) + tint[0] * strength;
                    self.albedo[1] = self.albedo[1] * (1.0 - strength) + tint[1] * strength;
                    self.albedo[2] = self.albedo[2] * (1.0 - strength) + tint[2] * strength;
                    self.albedo[3] = self.albedo[3] * (1.0 - strength) + tint[3] * strength;
                }
            }
        }

        if settings.affects_emissive {
            match settings.blend_mode {
                TintBlendMode::Multiply => {
                    self.emissive[0] *= 1.0 + (tint[0] - 1.0) * strength;
                    self.emissive[1] *= 1.0 + (tint[1] - 1.0) * strength;
                    self.emissive[2] *= 1.0 + (tint[2] - 1.0) * strength;
                }
                TintBlendMode::Overlay => {
                    self.emissive[0] = if self.emissive[0] < 0.5 {
                        2.0 * self.emissive[0] * tint[0] * strength
                    } else {
                        1.0 - 2.0 * (1.0 - self.emissive[0]) * (1.0 - tint[0] * strength)
                    }.clamp(0.0, 1.0);
                    self.emissive[1] = if self.emissive[1] < 0.5 {
                        2.0 * self.emissive[1] * tint[1] * strength
                    } else {
                        1.0 - 2.0 * (1.0 - self.emissive[1]) * (1.0 - tint[1] * strength)
                    }.clamp(0.0, 1.0);
                    self.emissive[2] = if self.emissive[2] < 0.5 {
                        2.0 * self.emissive[2] * tint[2] * strength
                    } else {
                        1.0 - 2.0 * (1.0 - self.emissive[2]) * (1.0 - tint[2] * strength)
                    }.clamp(0.0, 1.0);
                }
                TintBlendMode::Screen => {
                    self.emissive[0] = 1.0 - (1.0 - self.emissive[0]) * (1.0 - tint[0] * strength);
                    self.emissive[1] = 1.0 - (1.0 - self.emissive[1]) * (1.0 - tint[1] * strength);
                    self.emissive[2] = 1.0 - (1.0 - self.emissive[2]) * (1.0 - tint[2] * strength);
                }
                TintBlendMode::Additive => {
                    self.emissive[0] = (self.emissive[0] + tint[0] * strength).clamp(0.0, f32::MAX);
                    self.emissive[1] = (self.emissive[1] + tint[1] * strength).clamp(0.0, f32::MAX);
                    self.emissive[2] = (self.emissive[2] + tint[2] * strength).clamp(0.0, f32::MAX);
                }
                TintBlendMode::Replace => {
                    self.emissive[0] = self.emissive[0] * (1.0 - strength) + tint[0] * strength;
                    self.emissive[1] = self.emissive[1] * (1.0 - strength) + tint[1] * strength;
                    self.emissive[2] = self.emissive[2] * (1.0 - strength) + tint[2] * strength;
                }
            }
        }

        if settings.affects_roughness {
            match settings.blend_mode {
                TintBlendMode::Multiply => {
                    self.roughness *= 1.0 + (tint[3] - 1.0) * strength;
                }
                TintBlendMode::Overlay => {
                    self.roughness = if self.roughness < 0.5 {
                        2.0 * self.roughness * tint[3] * strength
                    } else {
                        1.0 - 2.0 * (1.0 - self.roughness) * (1.0 - tint[3] * strength)
                    }.clamp(0.0, 1.0);
                }
                TintBlendMode::Screen => {
                    self.roughness = 1.0 - (1.0 - self.roughness) * (1.0 - tint[3] * strength);
                }
                TintBlendMode::Additive => {
                    self.roughness = (self.roughness + tint[3] * strength).clamp(0.0, 1.0);
                }
                TintBlendMode::Replace => {
                    self.roughness = self.roughness * (1.0 - strength) + tint[3] * strength;
                }
            }
        }

        if settings.affects_metallic {
            match settings.blend_mode {
                TintBlendMode::Multiply => {
                    self.metallic *= 1.0 + (tint[3] - 1.0) * strength;
                }
                TintBlendMode::Overlay => {
                    self.metallic = if self.metallic < 0.5 {
                        2.0 * self.metallic * tint[3] * strength
                    } else {
                        1.0 - 2.0 * (1.0 - self.metallic) * (1.0 - tint[3] * strength)
                    }.clamp(0.0, 1.0);
                }
                TintBlendMode::Screen => {
                    self.metallic = 1.0 - (1.0 - self.metallic) * (1.0 - tint[3] * strength);
                }
                TintBlendMode::Additive => {
                    self.metallic = (self.metallic + tint[3] * strength).clamp(0.0, 1.0);
                }
                TintBlendMode::Replace => {
                    self.metallic = self.metallic * (1.0 - strength) + tint[3] * strength;
                }
            }
        }
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
    material_cache: HashMap<BlockId, BlockMaterial>,
}

impl BlockRegistry {
    pub fn initialize_default() -> Self {
        let mut registry = Self {
            blocks: HashMap::new(),
            name_to_id: HashMap::new(),
            base_id_to_variants: HashMap::new(),
            material_cache: HashMap::new(),
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
        self.material_cache.insert(id, def.material.clone());
        
        // Handle variations
        self.base_id_to_variants.entry(id.base_id)
            .or_default()
            .push(id);
        
        for variant in def.variations {
            let variant_id = BlockId::with_variation(id.base_id, variant.id);
            let mut variant_def = def.clone();
            variant_def.id = variant_id;
            variant_def.name = variant.name.clone();
            
            // Apply variant overrides
            for (face, tex) in variant.texture_overrides {
                variant_def.texture_faces.insert(face, tex);
            }
            
            // Update material with variant modifiers
            let mut variant_material = def.material.clone();
            variant_material.apply_modifiers(&variant.material_modifiers);
            
            self.blocks.insert(variant_id, variant_def);
            self.material_cache.insert(variant_id, variant_material);
        }
    }

    pub fn get(&self, id: BlockId) -> Option<&BlockDefinition> {
        self.blocks.get(&id)
    }

    pub fn get_material(&self, id: BlockId) -> Option<BlockMaterial> {
        self.material_cache.get(&id).cloned()
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

    pub fn generate_color_variants(&mut self, base_id: u32, colors: &[([f32; 4], &str)]) {
        if let Some(base_def) = self.get_base(base_id).cloned() {
            for (i, (color, name)) in colors.iter().enumerate() {
                let variant_id = (i + 1) as u16;
                let variant = BlockVariant {
                    id: variant_id,
                    name: format!("{} {}", base_def.name, name),
                    tint_color: Some(*color),
                    ..Default::default()
                };
                
                let mut variant_def = base_def.clone();
                variant_def.id = BlockId::with_variation(base_id, variant_id);
                variant_def.name = variant.name.clone();
                variant_def.variations.clear();
                variant_def.variations.push(variant);
                
                self.add_block(variant_def);
            }
        }
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
    #[error("Material error: {0}")]
    MaterialError(String),
}
