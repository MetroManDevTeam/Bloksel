// block.rs - Complete Implementation with Advanced Color Variation System

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};
use thiserror::Error;
use bitflags::bitflags;

// ========================
// Core Type Definitions
// ========================

#[derive(Debug, Clone, Copy, Serialize, Deserialize)] // Add Serialize/Deserialize
pub enum BlockFacing {
    North,
    South,
    East,
    West,
    Up,
    Down,
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)] // Add Serialize/Deserialize
pub enum BlockOrientation {
    Wall,
    Floor,
    Ceiling,
    Corner,
    Edge,
    Custom(u8),
}

bitflags! {
    #[derive(Serialize, Deserialize, Clone, Copy)] // Add Serialize/Deserialize
    pub struct ConnectedDirections: u8 {
        const NORTH = 0b00000001;
        const SOUTH = 0b00000010;
        const EAST = 0b00000100;
        const WEST = 0b00001000;
        const UP = 0b00010000;
        const DOWN = 0b00100000;
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockMaterial {
    pub id: u16,
    pub name: String,
    pub albedo: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: [f32; 3],
    pub texture_path: Option<String>,
    pub normal_map_path: Option<String>,
    pub occlusion_map_path: Option<String>,
    #[serde(default)]
    pub tintable: bool,
    #[serde(default)]
    pub grayscale_base: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId {
    pub base_id: u32,
    pub variation: u16,
    pub color_id: u16,
}

impl BlockId {
    pub fn new(base_id: u32) -> Self {
        Self { base_id, variation: 0, color_id: 0 }
    }

     pub const AIR: BlockId = BlockId {
        base_id: 0,
        variation: 0,
        color_id: 0
    };


    pub fn with_variation(base_id: u32, variation: u16) -> Self {
        Self { base_id, variation, color_id: 0 }
    }

    pub fn with_color(base_id: u32, color_id: u16) -> Self {
        Self { base_id, variation: 0, color_id }
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
        
        Ok(Self { base_id, variation, color_id })
    }

    pub fn to_combined(&self) -> u64 {
        ((self.base_id as u64) << 32) | 
        ((self.variation as u64) << 16) | 
        (self.color_id as u64)
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
    pub color_variations: Vec<ColorVariant>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorVariant {
    pub id: u16,
    pub name: String,
    pub color: [f32; 4],
    #[serde(default)]
    pub material_modifiers: MaterialModifiers,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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

    pub fn apply_modifiers(&mut self, modifiers: &MaterialModifiers) {
        // Apply albedo factor if present
        if let Some(factor) = modifiers.albedo_factor {
            self.albedo[0] *= factor[0];
            self.albedo[1] *= factor[1];
            self.albedo[2] *= factor[2];
        }

        // Apply roughness offset
        if let Some(offset) = modifiers.roughness_offset {
            self.roughness = (self.roughness + offset).clamp(0.0, 1.0);
        }

        // Apply metallic offset
        if let Some(offset) = modifiers.metallic_offset {
            self.metallic = (self.metallic + offset).clamp(0.0, 1.0);
        }

        // Apply emissive boost
        if let Some(boost) = modifiers.emissive_boost {
            self.emissive[0] += boost[0];
            self.emissive[1] += boost[1];
            self.emissive[2] += boost[2];
        }
    }
    pub fn apply_tint(&mut self, tint: [f32; 4], settings: &TintSettings) {
        if !settings.enabled || settings.strength <= 0.0 {
            return;
        }

        let strength = settings.strength.clamp(0.0, 1.0);
        
        if settings.affects_albedo {
            if self.grayscale_base {
                // Special handling for grayscale textures
                let luminance = 0.2126 * self.albedo[0] + 0.7152 * self.albedo[1] + 0.0722 * self.albedo[2];
                
                self.albedo[0] = luminance * tint[0] * strength + self.albedo[0] * (1.0 - strength);
                self.albedo[1] = luminance * tint[1] * strength + self.albedo[1] * (1.0 - strength);
                self.albedo[2] = luminance * tint[2] * strength + self.albedo[2] * (1.0 - strength);
            } else {
                // Standard tinting for colored textures
                match settings.blend_mode {
                    TintBlendMode::Multiply => {
                        self.albedo[0] *= 1.0 + (tint[0] - 1.0) * strength;
                        self.albedo[1] *= 1.0 + (tint[1] - 1.0) * strength;
                        self.albedo[2] *= 1.0 + (tint[2] - 1.0) * strength;
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
                    }
                    TintBlendMode::Screen => {
                        self.albedo[0] = 1.0 - (1.0 - self.albedo[0]) * (1.0 - tint[0] * strength);
                        self.albedo[1] = 1.0 - (1.0 - self.albedo[1]) * (1.0 - tint[1] * strength);
                        self.albedo[2] = 1.0 - (1.0 - self.albedo[2]) * (1.0 - tint[2] * strength);
                    }
                    TintBlendMode::Additive => {
                        self.albedo[0] = (self.albedo[0] + tint[0] * strength).clamp(0.0, 1.0);
                        self.albedo[1] = (self.albedo[1] + tint[1] * strength).clamp(0.0, 1.0);
                        self.albedo[2] = (self.albedo[2] + tint[2] * strength).clamp(0.0, 1.0);
                    }
                    TintBlendMode::Replace => {
                        self.albedo[0] = self.albedo[0] * (1.0 - strength) + tint[0] * strength;
                        self.albedo[1] = self.albedo[1] * (1.0 - strength) + tint[1] * strength;
                        self.albedo[2] = self.albedo[2] * (1.0 - strength) + tint[2] * strength;
                    }
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
                    }.clamp(0.0, f32::MAX);
                    self.emissive[1] = if self.emissive[1] < 0.5 {
                        2.0 * self.emissive[1] * tint[1] * strength
                    } else {
                        1.0 - 2.0 * (1.0 - self.emissive[1]) * (1.0 - tint[1] * strength)
                    }.clamp(0.0, f32::MAX);
                    self.emissive[2] = if self.emissive[2] < 0.5 {
                        2.0 * self.emissive[2] * tint[2] * strength
                    } else {
                        1.0 - 2.0 * (1.0 - self.emissive[2]) * (1.0 - tint[2] * strength)
                    }.clamp(0.0, f32::MAX);
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

#[derive(Clone)]
impl Block {
    pub fn get_primary_id(&self) -> BlockId {
        self.sub_blocks.values().next().map(|sb| sb.id).unwrap_or_else(|| BlockId::new(0))
    }

       pub const AIR: BlockId = BlockId {
        base_id: 0,
        variation: 0,
        color_id: 0
    };

    pub fn new(id: BlockId, resolution: u8) -> Self {
        Self {
            sub_blocks: HashMap::new(),
            resolution,
        }
    }

    pub fn place_sub_block(&mut self, x: u8, y: u8, z: u8, sub: SubBlock) {
        self.sub_blocks.insert((x, y, z), sub);
    }

    pub fn get_material(&self, registry: &BlockRegistry) -> BlockMaterial {
        let primary = self.get_primary_id();
        let mut material = registry.get_material(primary).unwrap_or_default();
        
        if let Some(def) = registry.get(primary) {
            // Apply regular variant modifiers if exists
            if let Some(variant) = registry.get_variant(primary) {
                material.apply_modifiers(&variant.material_modifiers);
            }
            
            // Apply color tint if exists
            if primary.is_colored() {
                if let Some(color_variant) = registry.get_color_variant(primary) {
                    material.apply_tint(color_variant.color, &def.tint_settings);
                    
                    // Apply color-specific material modifiers
                    material.apply_modifiers(&color_variant.material_modifiers);
                }
            }
        }
        
        material
    }
}

#[derive(Clone)]
impl SubBlock {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            metadata: 0,
            facing: BlockFacing::None,
            orientation: BlockOrientation::Wall,
             current_connections: ConnectedDirections::empty(),
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
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            name_to_id: HashMap::new(),
            base_id_to_variants: HashMap::new(),
            material_cache: HashMap::new(),
        }
    }

    pub fn initialize_default() -> Self {
        let mut registry = Self::new();

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
        
        // Handle regular variations
        self.base_id_to_variants.entry(id.base_id)
            .or_default()
            .push(id);
        
        for variant in &def.variations {
            let variant_id = BlockId {
                base_id: id.base_id,
                variation: variant.id,
                color_id: 0,
            };
            
            let mut variant_def = def.clone();
            variant_def.id = variant_id;
            variant_def.name = variant.name.clone();
            
            // Apply variant overrides
            for (face, tex) in &variant.texture_overrides.clone() {
                    variant_def.texture_faces.insert(*face, tex.clone());

             }
            
            // Update material with variant modifiers
            let mut variant_material = def.material.clone();
            variant_material.apply_modifiers(&variant.material_modifiers);
            
            self.blocks.insert(variant_id, variant_def);
            self.material_cache.insert(variant_id, variant_material);
        }
        
        // Handle color variations
        for color_variant in def.color_variations {
            let color_id = BlockId {
                base_id: id.base_id,
                variation: 0,
                color_id: color_variant.id,
            };
            
            let mut color_def = def.clone();
            color_def.id = color_id;
            color_def.name = format!("{} {}", def.name, color_variant.name);
            
            // Apply color to material
            let mut color_material = def.material.clone();
            color_material.apply_tint(color_variant.color, &def.tint_settings);
            color_material.apply_modifiers(&color_variant.material_modifiers);
            
            self.blocks.insert(color_id, color_def);
            self.material_cache.insert(color_id, color_material);
        }
    }

    pub fn add_color_palette(&mut self, base_id: u32, palette: &[([f32; 4], &str)]) {
        if let Some(base_def) = self.get_base(base_id).cloned() {
            for (i, (color, name)) in palette.iter().enumerate() {
                let color_id = (i + 1) as u16;
                let color_block_id = BlockId {
                    base_id,
                    variation: 0,
                    color_id,
                };

                let mut color_def = base_def.clone();
                color_def.id = color_block_id;
                color_def.name = format!("{} {}", base_def.name, name);
                
                // Add to color variations
                color_def.color_variations.push(ColorVariant {
                    id: color_id,
                    name: name.to_string(),
                    color: *color,
                    material_modifiers: MaterialModifiers::default(),
                });

                self.blocks.insert(color_block_id, color_def);
                self.base_id_to_variants.entry(base_id)
                    .or_default()
                    .push(color_block_id);
            }
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
        self.get(id)?
            .variations
            .iter()
            .find(|v| v.id == id.variation)
    }

    pub fn get_color_variant(&self, id: BlockId) -> Option<&ColorVariant> {
        self.get(id)?
            .color_variations
            .iter()
            .find(|v| v.id == id.color_id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&BlockDefinition> {
        self.name_to_id.get(name).and_then(|id| self.get(*id))
    }

    pub fn get_all_colors(&self, base_id: u32) -> Vec<(BlockId, [f32; 4])> {
        self.base_id_to_variants.get(&base_id)
            .map(|ids| ids.iter()
                .filter_map(|id| {
                    if id.color_id != 0 {
                        self.get_color_variant(*id).map(|v| (*id, v.color))
                    } else {
                        None
                    }
                })
                .collect())
            .unwrap_or_default()
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
    #[error("Color variant error: {0}")]
    ColorVariantError(String),
                        }
