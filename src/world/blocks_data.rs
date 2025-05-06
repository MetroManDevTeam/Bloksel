// blocks_data.rs - Complete Block Definitions for Voxel Game

use super::block_facing::BlockFacing;
use super::block_flags::BlockFlags;
use super::block_id::{BlockCategory, BlockDefinition, BlockId, BlockVariant, ColorVariant};
use super::block_material::{BlockMaterial, MaterialModifiers, TintSettings};
use super::block_orientation::BlockOrientation;
use crate::world::block_tech::{BlockFlags as TechBlockFlags, BlockPhysics};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
        self.id_to_name
            .get(&id)
            .and_then(|name| self.blocks.get(name))
    }

    pub fn get_block_material(&self, id: BlockId) -> Option<&BlockMaterial> {
        self.get_by_id(id).map(|def| &def.material)
    }

    pub fn get_block_flags(&self, id: BlockId) -> Option<TechBlockFlags> {
        self.get_by_id(id).map(|def| def.flags)
    }

    pub fn get_block_physics(&self, id: BlockId) -> BlockPhysics {
        self.get_by_id(id).map(|def| BlockPhysics::from(def.flags))
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
            flags: TechBlockFlags::NONE,
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
            physics: BlockPhysics::from(TechBlockFlags::NONE),
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
            material: BlockMaterial::new([0.5, 0.5, 0.5, 1.0], 0.8, 0.0, 0.0),
            flags: TechBlockFlags::SOLID,
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
            physics: BlockPhysics::solid(),
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
            material: BlockMaterial::new([0.3, 0.8, 0.3, 1.0], 0.6, 0.0, 0.0),
            flags: TechBlockFlags::SOLID,
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
            physics: BlockPhysics::solid(),
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
            material: BlockMaterial::new([0.2, 0.3, 0.9, 0.8], 0.1, 0.0, 0.0),
            flags: TechBlockFlags::LIQUID,
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
            physics: BlockPhysics::liquid(),
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
            material: BlockMaterial::new([1.0, 0.5, 0.0, 1.0], 0.3, 0.0, 1.0),
            flags: TechBlockFlags::LIQUID,
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
            physics: BlockPhysics::liquid(),
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
            material: BlockMaterial::new([0.9, 0.9, 0.7, 1.0], 0.9, 0.0, 0.0),
            flags: TechBlockFlags::SOLID,
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
            physics: BlockPhysics::solid(),
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
            material: BlockMaterial::new([0.9, 0.9, 0.9, 0.5], 0.1, 0.0, 0.0),
            flags: TechBlockFlags::SOLID,
            variations: Vec::new(),
            color_variations: Vec::new(),
            tint_settings: Default::default(),
            physics: BlockPhysics::solid(),
        });

        registry
    }
}

pub const BLOCKS: &[BlockDefinition] = &[
    // 1: Stone
    BlockDefinition {
        id: BlockId::new(1, 0, 0),
        name: String::new(), // Will be initialized at runtime
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::None,
        connects_to: HashSet::new(),   // Will be initialized at runtime
        texture_faces: HashMap::new(), // Will be initialized at runtime
        material: BlockMaterial::new([0.8, 0.8, 0.8, 1.0], 0.7, 0.0, 0.0),
        flags: TechBlockFlags::SOLID,
        variations: Vec::new(),       // Will be initialized at runtime
        color_variations: Vec::new(), // Will be initialized at runtime
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
        physics: BlockPhysics::solid(),
    },
    // 2: Grass
    BlockDefinition {
        id: BlockId::new(2, 0, 0),
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
        flags: TechBlockFlags::SOLID,
        variations: vec![],
        color_variations: vec![],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
        physics: BlockPhysics::solid(),
    },
    // 3: Water
    BlockDefinition {
        id: BlockId::new(3, 0, 0),
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
        flags: TechBlockFlags::LIQUID,
        variations: vec![],
        color_variations: vec![],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
        physics: BlockPhysics::liquid(),
    },
    // 4: Lava
    BlockDefinition {
        id: BlockId::new(4, 0, 0),
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
        flags: TechBlockFlags::LIQUID,
        variations: vec![],
        color_variations: vec![],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
        physics: BlockPhysics::liquid(),
    },
    // 5: Sand
    BlockDefinition {
        id: BlockId::new(5, 0, 0),
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
        flags: TechBlockFlags::SOLID,
        variations: vec![],
        color_variations: vec![],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
        physics: BlockPhysics::solid(),
    },
    // 6: Glass
    BlockDefinition {
        id: BlockId::new(6, 0, 0),
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
        flags: TechBlockFlags::SOLID,
        variations: vec![],
        color_variations: vec![],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
        physics: BlockPhysics::solid(),
    },
];

pub fn create_default_block() -> (TechBlockFlags, BlockMaterial) {
    let flags = TechBlockFlags::NONE;
    let material = BlockMaterial::default();
    (flags, material)
}

pub fn create_stone_block() -> (TechBlockFlags, BlockMaterial) {
    let flags = TechBlockFlags::SOLID;
    let material = BlockMaterial::new([0.8, 0.8, 0.8, 1.0], 0.7, 0.0, 0.0);
    (flags, material)
}

pub fn create_grass_block() -> (TechBlockFlags, BlockMaterial) {
    let flags = TechBlockFlags::SOLID;
    let material = BlockMaterial::new([0.4, 0.8, 0.3, 1.0], 0.9, 0.0, 0.0);
    (flags, material)
}

pub fn create_water_block() -> (TechBlockFlags, BlockMaterial) {
    let flags = TechBlockFlags::LIQUID;
    let material = BlockMaterial::new([0.2, 0.4, 0.8, 0.8], 0.1, 0.0, 0.0);
    (flags, material)
}

pub fn create_lava_block() -> (TechBlockFlags, BlockMaterial) {
    let flags = TechBlockFlags::LIQUID;
    let material = BlockMaterial::new([0.8, 0.2, 0.1, 0.8], 0.1, 0.0, 0.5);
    (flags, material)
}

pub fn create_sand_block() -> (TechBlockFlags, BlockMaterial) {
    let flags = TechBlockFlags::SOLID;
    let material = BlockMaterial::new([0.9, 0.9, 0.7, 1.0], 0.9, 0.0, 0.0);
    (flags, material)
}

pub fn create_glass_block() -> (TechBlockFlags, BlockMaterial) {
    let flags = TechBlockFlags::SOLID;
    let material = BlockMaterial::new([0.9, 0.9, 0.9, 0.3], 0.1, 0.0, 0.0);
    (flags, material)
}
