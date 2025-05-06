// blocks_data.rs - Complete Block Definitions for Voxel Game

use super::{
    block_id::{BlockDefinition, BlockVariant, ColorVariant},
    block_mat::{BlockMaterial, MaterialModifiers},
    block_tech::BlockFlags,
    block_visual::{BlockFacing, BlockOrientation},
};
use crate::world::{BlockDefinition, BlockId};
use std::collections::{HashMap, HashSet};

// Initialize a default block material
fn default_material() -> BlockMaterial {
    BlockMaterial {
        id: 0,
        name: "default".into(),
        albedo: [1.0, 1.0, 1.0, 1.0], // Using [f32; 4] instead of Vec4 for serialization
        roughness: 0.5,
        metallic: 0.0,
        emissive: [0.0, 0.0, 0.0],
        texture_path: None,
        normal_map_path: None,
        occlusion_map_path: None,
        tintable: false,
        grayscale_base: false,
        tint_mask_path: None,
        vertex_colored: false,
    }
}

pub const BLOCKS: &[BlockDefinition] = &[
    // 1: Stone
    BlockDefinition {
        id: BlockId::new(1),
        name: "stone".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([
            (BlockFacing::North, "stone_side.png".into()),
            (BlockFacing::South, "stone_side.png".into()),
            (BlockFacing::East, "stone_side.png".into()),
            (BlockFacing::West, "stone_side.png".into()),
            (BlockFacing::Up, "stone_top.png".into()),
            (BlockFacing::Down, "stone_bottom.png".into()),
        ]),
        material: default_material(),
        flags: BlockFlags {
            solid: true,
            occludes: true,
            break_resistance: 1.5,
            ..Default::default()
        },
        variations: vec![BlockVariant {
            id: 1,
            name: "cracked".into(),
            texture_overrides: HashMap::from([(BlockFacing::All, "stone_cracked.png".into())]),
            material_modifiers: MaterialModifiers {
                roughness_offset: Some(0.1),
                ..Default::default()
            },
        }],
        color_variations: vec![ColorVariant {
            id: 1,
            name: "mossy".into(),
            color: [0.4, 0.5, 0.3, 1.0],
            material_modifiers: Default::default(),
        }],
        tint_settings: Default::default(),
    },
    // 2: Grass Block
    BlockDefinition {
        id: BlockId::new(2),
        name: "grass".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([
            (BlockFacing::Up, "grass_top.png".into()),
            (BlockFacing::Down, "dirt.png".into()),
            (BlockFacing::North, "grass_side.png".into()),
            (BlockFacing::South, "grass_side.png".into()),
            (BlockFacing::East, "grass_side.png".into()),
            (BlockFacing::West, "grass_side.png".into()),
        ]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.4, 0.8, 0.3, 1.0];
            mat.roughness = 0.7;
            mat
        },
        flags: BlockFlags {
            solid: true,
            occludes: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 3: Dirt
    BlockDefinition {
        id: BlockId::new(3),
        name: "dirt".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([(BlockFacing::All, "dirt.png".into())]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.6, 0.4, 0.2, 1.0];
            mat.roughness = 0.8;
            mat
        },
        flags: BlockFlags {
            solid: true,
            occludes: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 4: Cobblestone
    BlockDefinition {
        id: BlockId::new(4),
        name: "cobblestone".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([(BlockFacing::All, "cobblestone.png".into())]),
        material: {
            let mut mat = default_material();
            mat.roughness = 0.9;
            mat
        },
        flags: BlockFlags {
            solid: true,
            occludes: true,
            break_resistance: 2.0,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 5: Oak Log
    BlockDefinition {
        id: BlockId::new(5),
        name: "oak_log".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::Up,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([
            (BlockFacing::Up, "log_oak_top.png".into()),
            (BlockFacing::Down, "log_oak_top.png".into()),
            (BlockFacing::North, "log_oak.png".into()),
            (BlockFacing::South, "log_oak.png".into()),
            (BlockFacing::East, "log_oak.png".into()),
            (BlockFacing::West, "log_oak.png".into()),
        ]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.5, 0.3, 0.1, 1.0];
            mat.roughness = 0.6;
            mat
        },
        flags: BlockFlags {
            solid: true,
            occludes: true,
            flammable: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 6: Oak Planks
    BlockDefinition {
        id: BlockId::new(6),
        name: "oak_planks".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([(BlockFacing::All, "planks_oak.png".into())]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.8, 0.6, 0.4, 1.0];
            mat.roughness = 0.7;
            mat
        },
        flags: BlockFlags {
            solid: true,
            occludes: true,
            flammable: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: vec![ColorVariant {
            id: 1,
            name: "dark".into(),
            color: [0.4, 0.3, 0.2, 1.0],
            material_modifiers: Default::default(),
        }],
        tint_settings: TintSettings {
            enabled: true,
            strength: 0.5,
            affects_albedo: true,
            blend_mode: TintBlendMode::Multiply,
            ..Default::default()
        },
    },
    // 7: Water
    BlockDefinition {
        id: BlockId::new(7),
        name: "water".into(),
        category: BlockCategory::Liquid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::from([BlockCategory::Liquid]),
        texture_faces: HashMap::from([(BlockFacing::All, "water.png".into())]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.2, 0.4, 0.8, 0.7];
            mat.roughness = 0.1;
            mat.metallic = 0.3;
            mat
        },
        flags: BlockFlags {
            liquid: true,
            passable: true,
            transparent: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 8: Lava
    BlockDefinition {
        id: BlockId::new(8),
        name: "lava".into(),
        category: BlockCategory::Liquid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::from([BlockCategory::Liquid]),
        texture_faces: HashMap::from([(BlockFacing::All, "lava.png".into())]),
        material: {
            let mut mat = default_material();
            mat.albedo = [1.0, 0.5, 0.1, 0.9];
            mat.roughness = 0.8;
            mat.emissive = [1.0, 0.6, 0.2];
            mat
        },
        flags: BlockFlags {
            liquid: true,
            passable: true,
            emissive: true,
            light_level: 15,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 9: Sand
    BlockDefinition {
        id: BlockId::new(9),
        name: "sand".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([(BlockFacing::All, "sand.png".into())]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.9, 0.8, 0.5, 1.0];
            mat.roughness = 0.9;
            mat
        },
        flags: BlockFlags {
            solid: true,
            occludes: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 10: Glass
    BlockDefinition {
        id: BlockId::new(10),
        name: "glass".into(),
        category: BlockCategory::Transparent,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([(BlockFacing::All, "glass.png".into())]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.9, 0.9, 0.95, 0.2];
            mat.roughness = 0.05;
            mat.metallic = 0.1;
            mat
        },
        flags: BlockFlags {
            transparent: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: vec![ColorVariant {
            id: 1,
            name: "tinted".into(),
            color: [0.1, 0.1, 0.1, 0.8],
            material_modifiers: Default::default(),
        }],
        tint_settings: TintSettings {
            enabled: true,
            strength: 0.8,
            affects_albedo: true,
            blend_mode: TintBlendMode::Multiply,
            mask_channel: TintMaskChannel::Alpha,
            ..Default::default()
        },
    },
    // 11: Brick
    BlockDefinition {
        id: BlockId::new(11),
        name: "brick".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([(BlockFacing::All, "brick.png".into())]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.8, 0.4, 0.3, 1.0];
            mat.roughness = 0.6;
            mat
        },
        flags: BlockFlags {
            solid: true,
            occludes: true,
            break_resistance: 2.0,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 12: Stone Slab
    BlockDefinition {
        id: BlockId::new(12),
        name: "stone_slab".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Floor,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([
            (BlockFacing::Up, "stone_slab_top.png".into()),
            (BlockFacing::Down, "stone_bottom.png".into()),
            (BlockFacing::North, "stone_slab_side.png".into()),
            (BlockFacing::South, "stone_slab_side.png".into()),
            (BlockFacing::East, "stone_slab_side.png".into()),
            (BlockFacing::West, "stone_slab_side.png".into()),
        ]),
        material: default_material(),
        flags: BlockFlags {
            solid: true,
            occludes: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 13: Wooden Slab
    BlockDefinition {
        id: BlockId::new(13),
        name: "wooden_slab".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Floor,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([
            (BlockFacing::Up, "planks_oak.png".into()),
            (BlockFacing::Down, "planks_oak.png".into()),
            (BlockFacing::North, "planks_oak.png".into()),
            (BlockFacing::South, "planks_oak.png".into()),
            (BlockFacing::East, "planks_oak.png".into()),
            (BlockFacing::West, "planks_oak.png".into()),
        ]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.7, 0.5, 0.3, 1.0];
            mat.roughness = 0.7;
            mat
        },
        flags: BlockFlags {
            solid: true,
            occludes: true,
            flammable: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 14: Stone Brick
    BlockDefinition {
        id: BlockId::new(14),
        name: "stone_brick".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([(BlockFacing::All, "stonebrick.png".into())]),
        material: default_material(),
        flags: BlockFlags {
            solid: true,
            occludes: true,
            break_resistance: 1.8,
            ..Default::default()
        },
        variations: vec![
            BlockVariant {
                id: 1,
                name: "cracked".into(),
                texture_overrides: HashMap::from([(
                    BlockFacing::All,
                    "stonebrick_cracked.png".into(),
                )]),
                material_modifiers: MaterialModifiers {
                    roughness_offset: Some(0.15),
                    ..Default::default()
                },
            },
            BlockVariant {
                id: 2,
                name: "mossy".into(),
                texture_overrides: HashMap::from([(
                    BlockFacing::All,
                    "stonebrick_mossy.png".into(),
                )]),
                material_modifiers: MaterialModifiers {
                    albedo_factor: Some([0.8, 0.9, 0.7]),
                    ..Default::default()
                },
            },
        ],
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 15: Bedrock
    BlockDefinition {
        id: BlockId::new(15),
        name: "bedrock".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([(BlockFacing::All, "bedrock.png".into())]),
        material: default_material(),
        flags: BlockFlags {
            solid: true,
            occludes: true,
            break_resistance: 9999.0,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 16: Snow
    BlockDefinition {
        id: BlockId::new(16),
        name: "snow".into(),
        category: BlockCategory::Flora,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([(BlockFacing::All, "snow.png".into())]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.95, 0.95, 0.98, 1.0];
            mat.roughness = 0.9;
            mat
        },
        flags: BlockFlags {
            passable: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 17: Cactus
    BlockDefinition {
        id: BlockId::new(17),
        name: "cactus".into(),
        category: BlockCategory::Flora,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([
            (BlockFacing::Up, "cactus_top.png".into()),
            (BlockFacing::Down, "cactus_bottom.png".into()),
            (BlockFacing::North, "cactus_side.png".into()),
            (BlockFacing::South, "cactus_side.png".into()),
            (BlockFacing::East, "cactus_side.png".into()),
            (BlockFacing::West, "cactus_side.png".into()),
        ]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.4, 0.7, 0.4, 1.0];
            mat.roughness = 0.8;
            mat
        },
        flags: BlockFlags {
            solid: true,
            occludes: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 18: Tall Grass
    BlockDefinition {
        id: BlockId::new(18),
        name: "tall_grass".into(),
        category: BlockCategory::Flora,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([(BlockFacing::All, "tallgrass.png".into())]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.3, 0.6, 0.2, 0.9];
            mat.roughness = 0.9;
            mat
        },
        flags: BlockFlags {
            transparent: true,
            passable: true,
            flammable: true,
            ..Default::default()
        },
        variations: vec![BlockVariant {
            id: 1,
            name: "fern".into(),
            texture_overrides: HashMap::from([(BlockFacing::All, "fern.png".into())]),
            material_modifiers: MaterialModifiers {
                albedo_factor: Some([0.8, 1.0, 0.8]),
                ..Default::default()
            },
        }],
        color_variations: Vec::new(),
        tint_settings: Default::default(),
    },
    // 19: Poppy
    BlockDefinition {
        id: BlockId::new(19),
        name: "poppy".into(),
        category: BlockCategory::Flora,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([(BlockFacing::All, "flower_poppy.png".into())]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.9, 0.2, 0.2, 0.9];
            mat.roughness = 0.9;
            mat
        },
        flags: BlockFlags {
            transparent: true,
            passable: true,
            flammable: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: vec![ColorVariant {
            id: 1,
            name: "blue".into(),
            color: [0.2, 0.2, 0.9, 1.0],
            material_modifiers: Default::default(),
        }],
        tint_settings: TintSettings {
            enabled: true,
            strength: 0.7,
            affects_albedo: true,
            blend_mode: TintBlendMode::Replace,
            ..Default::default()
        },
    },
    // 20: Glass Pane
    BlockDefinition {
        id: BlockId::new(20),
        name: "glass_pane".into(),
        category: BlockCategory::Decorative,
        default_facing: BlockFacing::North,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::from([BlockCategory::Solid, BlockCategory::Decorative]),
        texture_faces: HashMap::from([
            (BlockFacing::North, "glass_pane.png".into()),
            (BlockFacing::South, "glass_pane.png".into()),
            (BlockFacing::East, "glass_pane.png".into()),
            (BlockFacing::West, "glass_pane.png".into()),
            (BlockFacing::Up, "glass.png".into()),
            (BlockFacing::Down, "glass.png".into()),
        ]),
        material: {
            let mut mat = default_material();
            mat.albedo = [0.9, 0.9, 0.95, 0.3];
            mat.roughness = 0.05;
            mat.metallic = 0.1;
            mat
        },
        flags: BlockFlags {
            transparent: true,
            ..Default::default()
        },
        variations: Vec::new(),
        color_variations: vec![ColorVariant {
            id: 1,
            name: "stained".into(),
            color: [0.8, 0.5, 0.8, 0.5],
            material_modifiers: Default::default(),
        }],
        tint_settings: TintSettings {
            enabled: true,
            strength: 0.9,
            affects_albedo: true,
            blend_mode: TintBlendMode::Multiply,
            mask_channel: TintMaskChannel::Alpha,
            ..Default::default()
        },
    },
];

#[derive(Debug, Clone)]
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
}
