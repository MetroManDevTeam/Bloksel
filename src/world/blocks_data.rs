// blocks_data.rs - Complete Block Definitions for Voxel Game

use super::block_facing::BlockFacing;
use super::block_flags::BlockFlags;
use super::block_id::{BlockCategory, BlockDefinition, BlockId, BlockVariant, ColorVariant};
use super::block_material::{BlockMaterial, MaterialModifiers, TintSettings};
use super::block_orientation::BlockOrientation;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockCategory {
    Solid,
    Liquid,
    Transparent,
    Flora,
    Decorative,
}

impl BlockFlags {
    pub fn new() -> Self {
        Self {
            is_solid: false,
            is_transparent: false,
            is_liquid: false,
            is_flora: false,
            is_decorative: false,
            light_level: 0,
            break_resistance: 0,
        }
    }

    pub fn with_solid(mut self, solid: bool) -> Self {
        self.is_solid = solid;
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.is_transparent = transparent;
        self
    }

    pub fn with_liquid(mut self, liquid: bool) -> Self {
        self.is_liquid = liquid;
        self
    }

    pub fn with_flora(mut self, flora: bool) -> Self {
        self.is_flora = flora;
        self
    }

    pub fn with_decorative(mut self, decorative: bool) -> Self {
        self.is_decorative = decorative;
        self
    }

    pub fn with_light_level(mut self, light_level: u8) -> Self {
        self.light_level = light_level;
        self
    }

    pub fn with_break_resistance(mut self, break_resistance: u8) -> Self {
        self.break_resistance = break_resistance;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDefinition {
    pub id: BlockId,
    pub name: String,
    pub flags: BlockFlags,
    pub material: BlockMaterial,
}

impl BlockDefinition {
    pub fn new(id: BlockId, name: String) -> Self {
        Self {
            id,
            name,
            flags: BlockFlags::default(),
            material: BlockMaterial::default(),
        }
    }

    pub fn with_flags(mut self, flags: BlockFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_material(mut self, material: BlockMaterial) -> Self {
        self.material = material;
        self
    }
}

#[derive(Debug, Clone)]
pub struct BlockRegistry {
    blocks: HashMap<String, BlockDefinition>,
    id_to_name: HashMap<BlockId, String>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            id_to_name: HashMap::new(),
        }
    }

    pub fn register(&mut self, definition: BlockDefinition) {
        let name = definition.name.clone();
        let id = definition.id;
        self.blocks.insert(name.clone(), definition);
        self.id_to_name.insert(id, name);
    }

    pub fn get_by_name(&self, name: &str) -> Option<&BlockDefinition> {
        self.blocks.get(name)
    }

    pub fn get_by_id(&self, id: BlockId) -> Option<&BlockDefinition> {
        self.id_to_name.get(&id).and_then(|name| self.blocks.get(name))
    }

    pub fn get_block_material(&self, id: BlockId) -> Option<&BlockMaterial> {
        self.get_by_id(id).map(|def| &def.material)
    }

    pub fn get_block_flags(&self, id: BlockId) -> Option<BlockFlags> {
        self.get_by_id(id).map(|def| def.flags)
    }
}

impl Default for BlockRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        // Air
        registry.register(BlockDefinition {
            id: BlockId::new(0, 0, 0),
            name: "air".to_string(),
            category: BlockCategory::Gas,
            default_facing: BlockFacing::default(),
            default_orientation: BlockOrientation::default(),
            connects_to: HashSet::new(),
            texture_faces: HashMap::new(),
            material: BlockMaterial::default(),
            flags: BlockFlags::default().with_transparent(true),
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
        });

        // Stone
        registry.register(BlockDefinition {
            id: BlockId::new(1, 0, 0),
            name: "stone".to_string(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::default(),
            default_orientation: BlockOrientation::default(),
            connects_to: HashSet::new(),
            texture_faces: HashMap::new(),
            material: BlockMaterial {
                color: [0.5, 0.5, 0.5, 1.0],
                ..Default::default()
            },
            flags: BlockFlags::default().with_solid(true),
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
        });

        // Grass
        registry.register(BlockDefinition {
            id: BlockId::new(2, 0, 0),
            name: "grass".to_string(),
            category: BlockCategory::Flora,
            default_facing: BlockFacing::default(),
            default_orientation: BlockOrientation::default(),
            connects_to: HashSet::new(),
            texture_faces: HashMap::new(),
            material: BlockMaterial {
                color: [0.3, 0.8, 0.3, 1.0],
                ..Default::default()
            },
            flags: BlockFlags::default().with_solid(true),
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
        });

        // Water
        registry.register(BlockDefinition {
            id: BlockId::new(3, 0, 0),
            name: "water".to_string(),
            category: BlockCategory::Liquid,
            default_facing: BlockFacing::default(),
            default_orientation: BlockOrientation::default(),
            connects_to: HashSet::new(),
            texture_faces: HashMap::new(),
            material: BlockMaterial {
                color: [0.2, 0.3, 0.9, 0.8],
                ..Default::default()
            },
            flags: BlockFlags::default()
                .with_liquid(true)
                .with_transparent(true),
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
        });

        // Lava
        registry.register(BlockDefinition {
            id: BlockId::new(4, 0, 0),
            name: "lava".to_string(),
            category: BlockCategory::Liquid,
            default_facing: BlockFacing::default(),
            default_orientation: BlockOrientation::default(),
            connects_to: HashSet::new(),
            texture_faces: HashMap::new(),
            material: BlockMaterial {
                color: [1.0, 0.5, 0.0, 1.0],
                emissive: [1.0, 0.5, 0.0],
                ..Default::default()
            },
            flags: BlockFlags::default()
                .with_liquid(true)
                .with_light_level(15),
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
        });

        // Sand
        registry.register(BlockDefinition {
            id: BlockId::new(5, 0, 0),
            name: "sand".to_string(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::default(),
            default_orientation: BlockOrientation::default(),
            connects_to: HashSet::new(),
            texture_faces: HashMap::new(),
            material: BlockMaterial {
                color: [0.9, 0.9, 0.7, 1.0],
                ..Default::default()
            },
            flags: BlockFlags::default().with_solid(true),
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
        });

        // Glass
        registry.register(BlockDefinition {
            id: BlockId::new(6, 0, 0),
            name: "glass".to_string(),
            category: BlockCategory::Transparent,
            default_facing: BlockFacing::default(),
            default_orientation: BlockOrientation::default(),
            connects_to: HashSet::new(),
            texture_faces: HashMap::new(),
            material: BlockMaterial {
                color: [0.9, 0.9, 0.9, 0.5],
                ..Default::default()
            },
            flags: BlockFlags::default()
                .with_solid(true)
                .with_transparent(true),
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
        });

        registry
    }
}

pub const BLOCKS: &[BlockDefinition] = &[
    // 1: Stone
    BlockDefinition {
        id: BlockId::new(1),
        name: "stone".into(),
        category: crate::world::block_id::BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::None,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([
            (BlockFacing::PosZ, "stone_side.png".into()),
            (BlockFacing::NegZ, "stone_side.png".into()),
            (BlockFacing::PosX, "stone_side.png".into()),
            (BlockFacing::NegX, "stone_side.png".into()),
            (BlockFacing::PosY, "stone_top.png".into()),
            (BlockFacing::NegY, "stone_bottom.png".into()),
        ]),
        material: BlockMaterial::new([0.8, 0.8, 0.8, 1.0], 0.7, 0.0, 0.0),
        flags: BlockFlags::new()
            .with_solid(true)
            .with_transparent(false)
            .with_liquid(false)
            .with_flora(false)
            .with_decorative(false)
            .with_light_level(0)
            .with_break_resistance(1),
        variations: vec![BlockVariant {
            id: 1,
            name: "cracked".into(),
            texture_overrides: HashMap::from([(BlockFacing::None, "stone_cracked.png".into())]),
            material_modifiers: MaterialModifiers::default(),
        }],
        color_variations: vec![ColorVariant {
            id: 1,
            name: "mossy".into(),
            color: [0.4, 0.5, 0.3, 1.0],
            material_modifiers: MaterialModifiers::default(),
        }],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
    },
    // 2: Grass
    BlockDefinition {
        id: BlockId::new(2),
        name: "grass".into(),
        category: crate::world::block_id::BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::None,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([
            (BlockFacing::PosZ, "grass_side.png".into()),
            (BlockFacing::NegZ, "grass_side.png".into()),
            (BlockFacing::PosX, "grass_side.png".into()),
            (BlockFacing::NegX, "grass_side.png".into()),
            (BlockFacing::PosY, "grass_top.png".into()),
            (BlockFacing::NegY, "dirt.png".into()),
        ]),
        material: BlockMaterial::new([0.4, 0.8, 0.3, 1.0], 0.9, 0.0, 0.0),
        flags: BlockFlags::new()
            .with_solid(true)
            .with_transparent(false)
            .with_liquid(false)
            .with_flora(false)
            .with_decorative(false)
            .with_light_level(0)
            .with_break_resistance(1),
        variations: vec![],
        color_variations: vec![],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
    },
    // 3: Water
    BlockDefinition {
        id: BlockId::new(3),
        name: "water".into(),
        category: crate::world::block_id::BlockCategory::Liquid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::None,
        connects_to: HashSet::from([crate::world::block_id::BlockCategory::Liquid]),
        texture_faces: HashMap::from([
            (BlockFacing::PosZ, "water.png".into()),
            (BlockFacing::NegZ, "water.png".into()),
            (BlockFacing::PosX, "water.png".into()),
            (BlockFacing::NegX, "water.png".into()),
            (BlockFacing::PosY, "water.png".into()),
            (BlockFacing::NegY, "water.png".into()),
        ]),
        material: BlockMaterial::new([0.2, 0.4, 0.8, 0.8], 0.1, 0.0, 0.0),
        flags: BlockFlags::new()
            .with_solid(false)
            .with_transparent(true)
            .with_liquid(true)
            .with_flora(false)
            .with_decorative(false)
            .with_light_level(0)
            .with_break_resistance(1),
        variations: vec![],
        color_variations: vec![],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
    },
    // 4: Lava
    BlockDefinition {
        id: BlockId::new(4),
        name: "lava".into(),
        category: crate::world::block_id::BlockCategory::Liquid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::None,
        connects_to: HashSet::from([crate::world::block_id::BlockCategory::Liquid]),
        texture_faces: HashMap::from([
            (BlockFacing::PosZ, "lava.png".into()),
            (BlockFacing::NegZ, "lava.png".into()),
            (BlockFacing::PosX, "lava.png".into()),
            (BlockFacing::NegX, "lava.png".into()),
            (BlockFacing::PosY, "lava.png".into()),
            (BlockFacing::NegY, "lava.png".into()),
        ]),
        material: BlockMaterial::new([0.8, 0.2, 0.1, 0.8], 0.1, 0.0, 0.5),
        flags: BlockFlags::new()
            .with_solid(false)
            .with_transparent(true)
            .with_liquid(true)
            .with_flora(false)
            .with_decorative(false)
            .with_light_level(15)
            .with_break_resistance(2),
        variations: vec![],
        color_variations: vec![],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
    },
    // 5: Sand
    BlockDefinition {
        id: BlockId::new(5),
        name: "sand".into(),
        category: crate::world::block_id::BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::None,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([
            (BlockFacing::PosZ, "sand.png".into()),
            (BlockFacing::NegZ, "sand.png".into()),
            (BlockFacing::PosX, "sand.png".into()),
            (BlockFacing::NegX, "sand.png".into()),
            (BlockFacing::PosY, "sand.png".into()),
            (BlockFacing::NegY, "sand.png".into()),
        ]),
        material: BlockMaterial::new([0.9, 0.9, 0.7, 1.0], 0.9, 0.0, 0.0),
        flags: BlockFlags::new()
            .with_solid(true)
            .with_transparent(false)
            .with_liquid(false)
            .with_flora(false)
            .with_decorative(false)
            .with_light_level(0)
            .with_break_resistance(1),
        variations: vec![],
        color_variations: vec![],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
    },
    // 6: Glass
    BlockDefinition {
        id: BlockId::new(6),
        name: "glass".into(),
        category: crate::world::block_id::BlockCategory::Transparent,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::None,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([
            (BlockFacing::PosZ, "glass.png".into()),
            (BlockFacing::NegZ, "glass.png".into()),
            (BlockFacing::PosX, "glass.png".into()),
            (BlockFacing::NegX, "glass.png".into()),
            (BlockFacing::PosY, "glass.png".into()),
            (BlockFacing::NegY, "glass.png".into()),
        ]),
        material: BlockMaterial::new([0.9, 0.9, 0.9, 0.3], 0.1, 0.0, 0.0),
        flags: BlockFlags::new()
            .with_solid(true)
            .with_transparent(true)
            .with_liquid(false)
            .with_flora(false)
            .with_decorative(false)
            .with_light_level(0)
            .with_break_resistance(1),
        variations: vec![],
        color_variations: vec![],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
    },
];

pub fn create_default_block() -> (BlockFlags, BlockMaterial) {
    let flags = BlockFlags::empty()
        .with_solid(false)
        .with_transparent(false)
        .with_liquid(false)
        .with_flora(false)
        .with_decorative(false)
        .with_light_level(0)
        .with_break_resistance(0);

    let material = BlockMaterial::default();

    (flags, material)
}

pub fn create_stone_block() -> (BlockFlags, BlockMaterial) {
    let flags = BlockFlags::empty()
        .with_solid(true)
        .with_transparent(false)
        .with_liquid(false)
        .with_flora(false)
        .with_decorative(false)
        .with_light_level(0)
        .with_break_resistance(5);

    let material = BlockMaterial::default();

    (flags, material)
}

pub fn create_grass_block() -> (BlockFlags, BlockMaterial) {
    let flags = BlockFlags::empty()
        .with_solid(true)
        .with_transparent(false)
        .with_liquid(false)
        .with_flora(false)
        .with_decorative(false)
        .with_light_level(0)
        .with_break_resistance(2);

    let material = BlockMaterial::default();

    (flags, material)
}

pub fn create_water_block() -> (BlockFlags, BlockMaterial) {
    let flags = BlockFlags::empty()
        .with_solid(false)
        .with_transparent(true)
        .with_liquid(true)
        .with_flora(false)
        .with_decorative(false)
        .with_light_level(0)
        .with_break_resistance(0);

    let material = BlockMaterial::default();

    (flags, material)
}

pub fn create_lava_block() -> (BlockFlags, BlockMaterial) {
    let flags = BlockFlags::empty()
        .with_solid(false)
        .with_transparent(false)
        .with_liquid(true)
        .with_flora(false)
        .with_decorative(false)
        .with_light_level(15)
        .with_break_resistance(0);

    let material = BlockMaterial::default();

    (flags, material)
}

pub fn create_sand_block() -> (BlockFlags, BlockMaterial) {
    let flags = BlockFlags::empty()
        .with_solid(true)
        .with_transparent(false)
        .with_liquid(false)
        .with_flora(false)
        .with_decorative(false)
        .with_light_level(0)
        .with_break_resistance(1);

    let material = BlockMaterial::default();

    (flags, material)
}

pub fn create_glass_block() -> (BlockFlags, BlockMaterial) {
    let flags = BlockFlags::empty()
        .with_solid(true)
        .with_transparent(true)
        .with_liquid(false)
        .with_flora(false)
        .with_decorative(false)
        .with_light_level(0)
        .with_break_resistance(1);

    let material = BlockMaterial::default();

    (flags, material)
}
