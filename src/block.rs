// block.rs - Complete Implementation with Advanced Color Variation System

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};
use thiserror::Error;
use bitflags::bitflags;
use glam::{Vec3, Vec4};
// ========================
// Core Type Definitions
// ========================

// In block.rs add Default implementations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BlockFacing {
    None,
    North,
    South,
    East,
    West,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum BlockOrientation {
    #[default] 
    Wall,
    Floor,
    Ceiling,
    Corner,
    Edge,
    Custom(u8),
}

bitflags! {
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
    pub albedo: [f32:4],
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

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Wall,
            1 => Self::Floor,
            2 => Self::Ceiling,
            3 => Self::Corner,
            4 => Self::Edge,
            n => Self::Custom(n),
        }
    }
    
}

impl BlockOrientation {

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::North),
            2 => Some(Self::South),
            3 => Some(Self::East),
            4 => Some(Self::West),
            5 => Some(Self::Up),
            6 => Some(Self::Down),
            _ => None,
        }
    }

   } 

impl Default for BlockFacing {
    fn default() -> Self {
        BlockFacing::None
    }
}

impl BlockMaterial {
    /// Applies material modifiers from variants/colors
    pub fn apply_modifiers(&mut self, modifiers: &MaterialModifiers) {
        // Albedo RGB multiplier
        if let Some(factor) = modifiers.albedo_factor {
            self.albedo.x *= factor.x;
            self.albedo.y *= factor.y;
            self.albedo.z *= factor.z;
        }

        // Roughness adjustment
        if let Some(offset) = modifiers.roughness_offset {
            self.roughness = (self.roughness + offset).clamp(0.0, 1.0);
        }

        // Metallic adjustment
        if let Some(offset) = modifiers.metallic_offset {
            self.metallic = (self.metallic + offset).clamp(0.0, 1.0);
        }

        // Emissive boost (clamped to prevent HDR overflow)
        if let Some(boost) = modifiers.emissive_boost {
            self.emissive = (self.emissive + boost).min(Vec3::splat(1000.0)); // Arbitrary high value
        }

        // Direct tint strength override
        if let Some(strength) = modifiers.tint_strength {
            self.tint_strength = strength.clamp(0.0, 1.0);
        }
    }

    /// Applies color tint using specified blend mode
    pub fn apply_tint(&mut self, tint: Vec4, settings: &TintSettings) {
        if !settings.enabled || settings.strength <= 0.0 {
            return;
        }

        let strength = settings.strength.clamp(0.0, 1.0);
        let tint = tint * strength;

        // Albedo tinting
        if settings.affects_albedo {
            self.albedo = match settings.blend_mode {
                TintBlendMode::Multiply => Vec4::new(
                    self.albedo.x * tint.x,
                    self.albedo.y * tint.y,
                    self.albedo.z * tint.z,
                    self.albedo.w
                ),
                TintBlendMode::Overlay => Vec4::new(
                    if self.albedo.x < 0.5 {
                        2.0 * self.albedo.x * tint.x
                    } else {
                        1.0 - 2.0 * (1.0 - self.albedo.x) * (1.0 - tint.x)
                    },
                    // Repeat for y/z
                    self.albedo.y,
                    self.albedo.z,
                    self.albedo.w
                ),
                TintBlendMode::Screen => Vec4::ONE - (Vec4::ONE - self.albedo) * (Vec4::ONE - tint),
                TintBlendMode::Additive => (self.albedo + tint).min(Vec4::ONE),
                TintBlendMode::Replace => self.albedo.lerp(tint, strength),
            };
        }

        // Emissive tinting (RGB only)
        if settings.affects_emissive {
            self.emissive = match settings.blend_mode {
                TintBlendMode::Multiply => Vec3::new(
                    self.emissive.x * tint.x,
                    self.emissive.y * tint.y,
                    self.emissive.z * tint.z
                ),
                TintBlendMode::Overlay => Vec3::new(
                    if self.emissive.x < 0.5 {
                        2.0 * self.emissive.x * tint.x
                    } else {
                        1.0 - 2.0 * (1.0 - self.emissive.x) * (1.0 - tint.x)
                    },
                    // Repeat for y/z
                    self.emissive.y,
                    self.emissive.z
                ),
                TintBlendMode::Screen => Vec3::ONE - (Vec3::ONE - self.emissive) * (Vec3::ONE - tint.truncate()),
                TintBlendMode::Additive => (self.emissive + tint.truncate()).min(Vec3::splat(1000.0)),
                TintBlendMode::Replace => self.emissive.lerp(tint.truncate(), strength),
            };
        }

        // Roughness adjustment (using tint alpha)
        if settings.affects_roughness {
            self.roughness = match settings.blend_mode {
                TintBlendMode::Multiply => self.roughness * tint.w,
                TintBlendMode::Overlay => if self.roughness < 0.5 {
                    2.0 * self.roughness * tint.w
                } else {
                    1.0 - 2.0 * (1.0 - self.roughness) * (1.0 - tint.w)
                },
                TintBlendMode::Screen => 1.0 - (1.0 - self.roughness) * (1.0 - tint.w),
                TintBlendMode::Additive => (self.roughness + tint.w).min(1.0),
                TintBlendMode::Replace => self.roughness * (1.0 - strength) + tint.w * strength,
            }.clamp(0.0, 1.0);
        }

        // Metallic adjustment (using tint alpha)
        if settings.affects_metallic {
            self.metallic = match settings.blend_mode {
                TintBlendMode::Multiply => self.metallic * tint.w,
                TintBlendMode::Overlay => if self.metallic < 0.5 {
                    2.0 * self.metallic * tint.w
                } else {
                    1.0 - 2.0 * (1.0 - self.metallic) * (1.0 - tint.w)
                },
                TintBlendMode::Screen => 1.0 - (1.0 - self.metallic) * (1.0 - tint.w),
                TintBlendMode::Additive => (self.metallic + tint.w).min(1.0),
                TintBlendMode::Replace => self.metallic * (1.0 - strength) + tint.w * strength,
            }.clamp(0.0, 1.0);
        }
    }

    /// Generates a GPU-friendly material uniform
    pub fn to_uniform(&self) -> MaterialUniform {
        MaterialUniform {
            albedo: self.albedo,
            emissive: self.emissive.extend(0.0),
            roughness_metallic: Vec2::new(self.roughness, self.metallic),
            flags: self.get_flags_bits(),
        }
    }

    /// Packs material flags into bitfield
    fn get_flags_bits(&self) -> u32 {
        let mut bits = 0;
        bits |= (self.tintable as u32) << 0;
        bits |= (self.grayscale_base as u32) << 1;
        bits |= (self.vertex_colored as u32) << 2;
        bits
    }
}

impl Block {
    pub fn get_material(&self, registry: &BlockRegistry) -> BlockMaterial {
        let primary_id = self.get_primary_id();
        let mut material = registry
            .get_material(primary_id)
            .unwrap_or_else(BlockMaterial::default);

        if let Some(def) = registry.get(primary_id) {
            // Apply variant modifiers
            if let Some(variant) = registry.get_variant(primary_id) {
                material.apply_modifiers(&variant.material_modifiers);
            }

            // Apply color tint
            if primary_id.is_colored() {
                if let Some(color_variant) = registry.get_color_variant(primary_id) {
                    material.apply_tint(color_variant.color, &def.tint_settings);
                    material.apply_modifiers(&color_variant.material_modifiers);
                }
            }
        }

        material
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
            orientation: BlockOrientation::Wall,
            connections: ConnectedDirections::empty(),
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

// ========================
// Block Registry
// ========================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPhysics {
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
    pub dynamic: bool,
    pub passable: bool,
    pub break_resistance: f32,
    pub flammability: f32,
    pub thermal_conductivity: f32,
    pub emissive: bool,
    pub light_level: u8,
    pub 
    physics: HashMap<BlockId, BlockPhysics>,  // Add this
     
}

impl Default for BlockPhysics {
    fn default() -> Self {
        Self {
            density: 1000.0, // Water density as default
            friction: 0.6,
            restitution: 0.0,
            dynamic: false,
            passable: false,
            break_resistance: 1.0,
            flammability: 0.0,
            thermal_conductivity: 0.5,
            emissive: false,
            light_level: 0,
        }
    }
}

impl BlockPhysics {
    pub fn solid(density: f32) -> Self {
        Self {
            density,
            friction: 0.6,
            restitution: 0.1,
            dynamic: false,
            passable: false,
            ..Default::default()
        }
    }

    pub fn liquid(density: f32) -> Self {
        Self {
            density,
            friction: 0.0,
            restitution: 0.0,
            dynamic: true,
            passable: true,
            ..Default::default()
        }
    }

    pub fn gas() -> Self {
        Self {
            density: 1.2, // Air density
            friction: 0.0,
            restitution: 0.0,
            dynamic: true,
            passable: true,
            ..Default::default()
        }
    }

    pub fn mass(&self, volume: f32) -> f32 {
        self.density * volume
    }
}



#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    // ========================
    // Initialization
    // ========================
    
    /// Creates an empty registry
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
    
    /// Pre-populates with default blocks
    pub fn initialize_default() -> Self {
        let mut registry = Self::new();
        
        // Load from generated blocks data
        for def in BLOCKS.iter().cloned() {
            registry.add_block(def);
        }
        
        // Build texture atlas
        registry.rebuild_texture_atlas();
        
        registry
    }
    
    // ========================
    // Core Block Management
    // ========================
    
    /// Adds a block definition to the registry
    pub fn add_block(&mut self, def: BlockDefinition) -> Result<(), BlockError> {
        // Validate ID uniqueness
        if self.blocks.contains_key(&def.id) {
            return Err(BlockError::DuplicateId(def.id));
        }
        
        // Validate name uniqueness
        if self.name_to_id.contains_key(&def.name) {
            return Err(BlockError::DuplicateName(def.name.clone()));
        }
        
        let id = def.id;
        
        // Main storage
        self.blocks.insert(id, def.clone());
        self.name_to_id.insert(def.name.clone(), id);
        
        // Add to category index
        self.category_index.entry(def.category)
            .or_default()
            .insert(id);
        
        // Handle base ID mapping
        self.base_id_to_variants.entry(id.base_id)
            .or_default()
            .push(id);
        
        // Process variants
        self.process_variants(&def)?;
        
        // Process color variations
        self.process_color_variations(&def)?;
        
        // Cache physics
        self.physics_cache.insert(id, def.physics.clone());
        
        Ok(())
    }
    
    /// Processes all block variants
    fn process_variants(&mut self, base_def: &BlockDefinition) -> Result<(), BlockError> {
        for variant in &base_def.variations {
            let variant_id = BlockId {
                base_id: base_def.id.base_id,
                variation: variant.id,
                color_id: 0,
            };
            
            // Create variant definition
            let mut variant_def = base_def.clone();
            variant_def.id = variant_id;
            variant_def.name = format!("{} {}", base_def.name, variant.name);
            
            // Apply texture overrides
            for (face, tex) in &variant.texture_overrides {
                variant_def.texture_faces.insert(*face, tex.clone());
            }
            
            // Process material
            let mut variant_material = base_def.material.clone();
            variant_material.apply_modifiers(&variant.material_modifiers);
            
            // Store
            self.blocks.insert(variant_id, variant_def);
            self.material_cache.insert(variant_id, variant_material);
            
            // Add to name mapping
            self.name_to_id.insert(
                format!("{}:{}", base_def.name, variant.id),
                variant_id
            );
        }
        
        Ok(())
    }
    
    /// Processes all color variations
    fn process_color_variations(&mut self, base_def: &BlockDefinition) -> Result<(), BlockError> {
        for color_variant in &base_def.color_variations {
            let color_id = BlockId {
                base_id: base_def.id.base_id,
                variation: 0,
                color_id: color_variant.id,
            };
            
            // Create color variant definition
            let mut color_def = base_def.clone();
            color_def.id = color_id;
            color_def.name = format!("{} {}", base_def.name, color_variant.name);
            
            // Process material with tint
            let mut color_material = base_def.material.clone();
            color_material.apply_tint(color_variant.color, &base_def.tint_settings);
            color_material.apply_modifiers(&color_variant.material_modifiers);
            
            // Store
            self.blocks.insert(color_id, color_def);
            self.material_cache.insert(color_id, color_material);
            
            // Add to name mapping
            self.name_to_id.insert(
                format!("{}:C{}", base_def.name, color_variant.id),
                color_id
            );
        }
        
        Ok(())
    }
    
    // ========================
    // Query Methods
    // ========================
    
    /// Gets a block definition by ID
    pub fn get(&self, id: BlockId) -> Option<&BlockDefinition> {
        self.blocks.get(&id)
    }
    
    /// Gets a block definition by name
    pub fn get_by_name(&self, name: &str) -> Option<&BlockDefinition> {
        self.name_to_id.get(name).and_then(|id| self.get(*id))
    }
    
    /// Gets the base definition (ignoring variants/colors)
    pub fn get_base(&self, base_id: u32) -> Option<&BlockDefinition> {
        self.get(BlockId::new(base_id))
    }
    
    /// Gets a specific variant
    pub fn get_variant(&self, id: BlockId) -> Option<&BlockVariant> {
        self.get(id)?
            .variations
            .iter()
            .find(|v| v.id == id.variation)
    }
    
    /// Gets a specific color variant
    pub fn get_color_variant(&self, id: BlockId) -> Option<&ColorVariant> {
        self.get(id)?
            .color_variations
            .iter()
            .find(|v| v.id == id.color_id)
    }
    
    /// Gets the material for a block (with all modifications applied)
    pub fn get_material(&self, id: BlockId) -> Option<BlockMaterial> {
        self.material_cache.get(&id).cloned().or_else(|| {
            // Fallback for uncached materials
            let mut material = self.get(id)?.material.clone();
            
            if let Some(variant) = self.get_variant(id) {
                material.apply_modifiers(&variant.material_modifiers);
            }
            
            if id.color_id != 0 {
                if let Some(color_variant) = self.get_color_variant(id) {
                    if let Some(def) = self.get(id) {
                        material.apply_tint(color_variant.color, &def.tint_settings);
                        material.apply_modifiers(&color_variant.material_modifiers);
                    }
                }
            }
            
            Some(material)
        })
    }
    
    /// Gets physics properties for a block
    pub fn get_physics(&self, id: BlockId) -> BlockPhysics {
        self.physics_cache.get(&id)
            .cloned()
            .unwrap_or_default()
    }
    
    // ========================
    // Bulk Operations
    // ========================
    
    /// Gets all variants of a base block
    pub fn get_all_variants(&self, base_id: u32) -> Vec<BlockId> {
        self.base_id_to_variants.get(&base_id)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Gets all color variants with their colors
    pub fn get_all_colors(&self, base_id: u32) -> Vec<(BlockId, Vec4)> {
        self.base_id_to_variants.get(&base_id)
            .map(|ids| ids.iter()
                .filter_map(|id| {
                    if id.color_id != 0 {
                        self.get_color_variant(*id)
                            .map(|v| (*id, v.color))
                    } else {
                        None
                    }
                })
                .collect())
            .unwrap_or_default()
    }
    
    /// Gets all blocks in a category
    pub fn get_by_category(&self, category: BlockCategory) -> Vec<BlockId> {
        self.category_index.get(&category)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    // ========================
    // Texture Management
    // ========================
    
    /// Rebuilds the texture atlas index
    pub fn rebuild_texture_atlas(&mut self) {
        let mut texture_paths = HashSet::new();
        
        // Collect all unique texture paths
        for def in self.blocks.values() {
            for path in def.texture_faces.values() {
                texture_paths.insert(path.clone());
            }
            
            for variant in &def.variations {
                for path in variant.texture_overrides.values() {
                    texture_paths.insert(path.clone());
                }
            }
        }
        
        // Assign indices
        self.texture_atlas_indices.clear();
        for (idx, path) in texture_paths.into_iter().enumerate() {
            self.texture_atlas_indices.insert(path, idx as u32);
        }
    }
    
    /// Gets the atlas index for a texture path
    pub fn get_texture_index(&self, path: &str) -> Option<u32> {
        self.texture_atlas_indices.get(path).copied()
    }
    
    // ========================
    // Serialization
    // ========================
    
    /// Serializes the registry for saving
    pub fn serialize(&self) -> Result<Vec<u8>, BlockError> {
        bincode::serialize(self).map_err(|_| BlockError::SerializationFailed)
    }
    
    /// Deserializes the registry
    pub fn deserialize(data: &[u8]) -> Result<Self, BlockError> {
        let mut registry: Self = bincode::deserialize(data)
            .map_err(|_| BlockError::DeserializationFailed)?;
        
        // Rebuild caches and indexes
        registry.rebuild_caches();
        
        Ok(registry)
    }
    
    /// Rebuilds all internal caches
    fn rebuild_caches(&mut self) {
        self.material_cache.clear();
        self.physics_cache.clear();
        self.base_id_to_variants.clear();
        self.category_index.clear();
        
        for (id, def) in &self.blocks {
            // Rebuild material cache
            let mut material = def.material.clone();
            
            if id.variation != 0 {
                if let Some(variant) = self.get_variant(*id) {
                    material.apply_modifiers(&variant.material_modifiers);
                }
            }
            
            if id.color_id != 0 {
                if let Some(color_variant) = self.get_color_variant(*id) {
                    material.apply_tint(color_variant.color, &def.tint_settings);
                    material.apply_modifiers(&color_variant.material_modifiers);
                }
            }
            
            self.material_cache.insert(*id, material);
            
            // Rebuild physics cache
            self.physics_cache.insert(*id, def.physics.clone());
            
            // Rebuild indexes
            self.base_id_to_variants.entry(id.base_id)
                .or_default()
                .push(*id);
            
            self.category_index.entry(def.category)
                .or_default()
                .insert(*id);
        }
        
        self.rebuild_texture_atlas();
    }
}

// ========================
// Error Handling
// ========================

#[derive(Debug, Error)]
pub enum BlockError {
    #[error("Duplicate block ID: {0:?}")]
    DuplicateId(BlockId),
    
    #[error("Duplicate block name: {0}")]
    DuplicateName(String),
    
    #[error("Invalid variant data")]
    InvalidVariant,
    
    #[error("Serialization failed")]
    SerializationFailed,
    
    #[error("Deserialization failed")]
    DeserializationFailed,
    
    #[error("Texture not found: {0}")]
    TextureNotFound(String),
}
