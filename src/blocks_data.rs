// blocks_data.rs - Block Definitions for Voxel Game

use crate::block::{BlockDefinition, BlockCategory, BlockFacing, BlockOrientation, BlockFlags};
use std::collections::{HashMap, HashSet};

// Utility function to generate texture names dynamically based on block ID and face
fn generate_texture_name(block_id: u32, facing: BlockFacing) -> String {
    format!("{}_{}.png", block_id, facing.to_string().to_lowercase())
}

pub fn blocks_data() -> Vec<BlockDefinition> {
    vec![
        // 1: Basic Stone
        BlockDefinition {
            id: 1,
            name: "stone".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(1, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },
        
        // 2: Grass Block
        BlockDefinition {
            id: 2,
            name: "grass".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::Up, generate_texture_name(2, BlockFacing::Up)),
                (BlockFacing::Down, generate_texture_name(2, BlockFacing::Down)),
                (BlockFacing::All, generate_texture_name(2, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 3: Dirt Block
        BlockDefinition {
            id: 3,
            name: "dirt".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(3, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 4: Stone Variations
        BlockDefinition {
            id: 4,
            name: "stone_variation".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(4, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 5: Oak Log
        BlockDefinition {
            id: 5,
            name: "oak_log".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::Up,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::Up, generate_texture_name(5, BlockFacing::Up)),
                (BlockFacing::Down, generate_texture_name(5, BlockFacing::Down)),
                (BlockFacing::North, generate_texture_name(5, BlockFacing::North)),
                (BlockFacing::South, generate_texture_name(5, BlockFacing::South)),
                (BlockFacing::East, generate_texture_name(5, BlockFacing::East)),
                (BlockFacing::West, generate_texture_name(5, BlockFacing::West)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags { flammable: true, ..Default::default() },
        },

        // 6: Oak Planks
        BlockDefinition {
            id: 6,
            name: "oak_planks".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(6, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 7: Water
        BlockDefinition {
            id: 7,
            name: "water".into(),
            category: BlockCategory::Liquid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(7, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags { liquid: true, ..Default::default() },
        },

        // 8: Lava
        BlockDefinition {
            id: 8,
            name: "lava".into(),
            category: BlockCategory::Liquid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(8, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags { liquid: true, ..Default::default() },
        },

        // 9: Sand
        BlockDefinition {
            id: 9,
            name: "sand".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(9, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 10: Glass
        BlockDefinition {
            id: 10,
            name: "glass".into(),
            category: BlockCategory::Transparent,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(10, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags { transparent: true, ..Default::default() },
        },

        // 11: Brick
        BlockDefinition {
            id: 11,
            name: "brick".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(11, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 12: Stone Slab
        BlockDefinition {
            id: 12,
            name: "stone_slab".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Floor,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::Up, generate_texture_name(12, BlockFacing::Up)),
                (BlockFacing::Down, generate_texture_name(12, BlockFacing::Down)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 13: Wooden Slab
        BlockDefinition {
            id: 13,
            name: "wooden_slab".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Floor,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::Up, generate_texture_name(13, BlockFacing::Up)),
                (BlockFacing::Down, generate_texture_name(13, BlockFacing::Down)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 14: Stone Brick
        BlockDefinition {
            id: 14,
            name: "stone_brick".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(14, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 15: Bedrock
        BlockDefinition {
            id: 15,
            name: "bedrock".into(),
            category: BlockCategory::Solid,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(15, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 16: Snow
        BlockDefinition {
            id: 16,
            name: "snow".into(),
            category: BlockCategory::Flora,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(16, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 17: Cactus
        BlockDefinition {
            id: 17,
            name: "cactus".into(),
            category: BlockCategory::Flora,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(17, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 18: Tall Grass
        BlockDefinition {
            id: 18,
            name: "tall_grass".into(),
            category: BlockCategory::Flora,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(18, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 19: Poppy
        BlockDefinition {
            id: 19,
            name: "poppy".into(),
            category: BlockCategory::Flora,
            default_facing: BlockFacing::None,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::new(),
            texture_faces: HashMap::from([
                (BlockFacing::All, generate_texture_name(19, BlockFacing::All)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags::default(),
        },

        // 20: Glass Pane
        BlockDefinition {
            id: 20,
            name: "glass_pane".into(),
            category: BlockCategory::Decorative,
            default_facing: BlockFacing::North,
            default_orientation: BlockOrientation::Wall,
            connects_to: HashSet::from([BlockCategory::Solid, BlockCategory::Decorative]),
            texture_faces: HashMap::from_iter([
                (BlockFacing::North, generate_texture_name(20, BlockFacing::North)),
                (BlockFacing::South, generate_texture_name(20, BlockFacing::South)),
                (BlockFacing::East, generate_texture_name(20, BlockFacing::East)),
                (BlockFacing::West, generate_texture_name(20, BlockFacing::West)),
            ]),
            material: BlockMaterial::default(),
            flags: BlockFlags { transparent: true, ..Default::default() },
        },

        // 21-45 would follow a similar structure for other blocks...
    ]
}
