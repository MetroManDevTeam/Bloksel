// blocks_data.rs - Block Definitions for Voxel Game

use crate::block::{
    BlockDefinition, BlockCategory, BlockFacing, BlockOrientation, 
    BlockFlags, BlockId, BlockMaterial, MaterialModifiers, ColorVariant, BlockVariant
};
use std::collections::{HashMap, HashSet};

// Utility function to generate texture names
fn generate_texture_name(block_id: u32, facing: BlockFacing) -> String {
    format!("{}_{}.png", block_id, facing.to_string().to_lowercase())
}

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
    // 1: Basic Stone
    BlockDefinition {
        id: BlockId::new(1),
        name: "stone".into(),
        category: BlockCategory::Solid,
        default_facing: BlockFacing::None,
        default_orientation: BlockOrientation::Wall,
        connects_to: HashSet::new(),
        texture_faces: HashMap::from([
            (BlockFacing::North, generate_texture_name(1, BlockFacing::North)),
            (BlockFacing::South, generate_texture_name(1, BlockFacing::South)),
            (BlockFacing::East, generate_texture_name(1, BlockFacing::East)),
            (BlockFacing::West, generate_texture_name(1, BlockFacing::West)),
            (BlockFacing::Up, generate_texture_name(1, BlockFacing::Up)),
            (BlockFacing::Down, generate_texture_name(1, BlockFacing::Down)),
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
            mat.albedo = [0.4, 0.8, 0.3, 1.0]; // Green tint
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

    // 3-20: Other blocks follow same pattern...
    // Add more blocks with proper BlockId construction and complete data
];
