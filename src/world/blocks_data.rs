// blocks_data.rs - Complete Block Definitions for Voxel Game

use crate::world::block_facing::BlockFacing;
use crate::world::block_id::{BlockDefinition, BlockId, BlockVariant, ColorVariant};
use crate::world::block_material::{BlockMaterial, MaterialModifiers, TintSettings};
use crate::world::block_orientation::BlockOrientation;
use crate::world::block_tech::{BlockFlags, BlockPhysics};
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

// Initialize a default block material
fn default_material() -> BlockMaterial {
    BlockMaterial {
        id: 0,
        name: "default".into(),
        albedo: [1.0, 1.0, 1.0, 1.0], // Using [f32; 4] instead of Vec4 for serialization
        roughness: 0.5,
        metallic: 0.0,
        emission: [0.0, 0.0, 0.0],
        texture_path: None,
        normal_map_path: None,
        occlusion_map_path: None,
        tintable: false,
        grayscale_base: false,
        tint_mask_path: None,
        vertex_colored: false,
    }
}

#[derive(Debug, Clone, Default)]
pub struct BlockRegistry {
    blocks: HashMap<BlockId, BlockDefinition>,
    name_to_id: HashMap<String, BlockId>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            name_to_id: HashMap::new(),
        }
    }

    pub fn register(&mut self, block: BlockDefinition) {
        self.blocks.insert(block.id, block.clone());
        self.name_to_id.insert(block.name.clone(), block.id);
    }

    pub fn get(&self, id: BlockId) -> Option<&BlockDefinition> {
        self.blocks.get(&id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&BlockDefinition> {
        self.name_to_id.get(name).and_then(|id| self.blocks.get(id))
    }

    pub fn iter(&self) -> impl Iterator<Item = &BlockDefinition> {
        self.blocks.values()
    }

    pub fn get_material(&self, id: BlockId) -> Option<BlockMaterial> {
        self.get(id).map(|def| def.material.clone())
    }

    pub fn get_physics(&self, id: BlockId) -> BlockPhysics {
        self.get(id)
            .map(|def| BlockPhysics::from(def.flags))
            .unwrap_or_default()
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
        flags: BlockFlags::SOLID,
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
        flags: BlockFlags::SOLID,
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
        flags: BlockFlags::LIQUID,
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
        flags: BlockFlags::LIQUID,
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
        material: BlockMaterial::new([0.9, 0.8, 0.6, 1.0], 0.9, 0.0, 0.0),
        flags: BlockFlags::SOLID,
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
        flags: BlockFlags::SOLID,
        variations: vec![],
        color_variations: vec![],
        tint_settings: TintSettings {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        },
    },
];
