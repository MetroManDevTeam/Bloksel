use serde::{Deserialize, Serialize};
use crate::world::block::Block;
use crate::world::BlockFacing;
use crate::world::BlockOrientation;

pub type BlockId = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockData {
    pub id: BlockId,
    pub metadata: u16,
}

pub use 

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
    pub fn new(id: u16) -> Self {
        Self(id)
    }

    pub fn to_block(self) -> Block {
        Block::new(self.0)
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
pub struct Block {
    pub sub_blocks: HashMap<(u8, u8, u8), SubBlock>,
    pub resolution: u8,
    pub current_connections: ConnectedDirections,

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
