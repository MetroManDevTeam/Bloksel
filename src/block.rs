// block.rs - Realistic Voxel Block Definitions (Full Implementation)

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use thiserror::Error;
use bitflags::bitflags;
use crate::chunk_renderer::BlockMaterial;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDefinition {
    pub id: u32,
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
    #[serde(default)]
    pub material: BlockMaterial,
    #[serde(default)]
    pub flags: BlockFlags,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubBlock {
    pub id: u32,
    pub variation: u8,
    pub metadata: u8,
    pub facing: BlockFacing,
    pub orientation: BlockOrientation,
    pub connections: ConnectedDirections,
    pub material_mod: f32,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub sub_blocks: HashMap<(u8, u8, u8), SubBlock>,
    pub resolution: u8,
    pub current_connections: ConnectedDirections,
}

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

impl ConnectedDirections {
    pub fn from_surroundings(neighbors: &[(BlockFacing, bool)]) -> Self {
        let mut connections = ConnectedDirections::empty();
        for (facing, exists) in neighbors {
            if *exists {
                connections.insert(match facing {
                    BlockFacing::North => ConnectedDirections::NORTH,
                    BlockFacing::South => ConnectedDirections::SOUTH,
                    BlockFacing::East => ConnectedDirections::EAST,
                    BlockFacing::West => ConnectedDirections::WEST,
                    BlockFacing::Up => ConnectedDirections::UP,
                    BlockFacing::Down => ConnectedDirections::DOWN,
                    _ => ConnectedDirections::empty(),
                });
            }
        }
        connections
    }
}

impl Block {
    pub fn update_connections(&mut self, registry: &BlockRegistry, neighbors: &[(BlockFacing, Option<&Block>)]) {
        let mut new_connections = ConnectedDirections::empty();
        for (facing, neighbor) in neighbors {
            if let Some(neighbor_block) = neighbor {
                let neighbor_def = registry.get(neighbor_block.get_primary_id());
                if let Some(def) = neighbor_def {
                    if def.connects_to.contains(&self.get_primary_category(registry)) {
                        new_connections.insert(match facing {
                            BlockFacing::North => ConnectedDirections::NORTH,
                            BlockFacing::South => ConnectedDirections::SOUTH,
                            BlockFacing::East => ConnectedDirections::EAST,
                            BlockFacing::West => ConnectedDirections::WEST,
                            BlockFacing::Up => ConnectedDirections::UP,
                            BlockFacing::Down => ConnectedDirections::DOWN,
                            _ => ConnectedDirections::empty(),
                        });
                    }
                }
            }
        }
        self.current_connections = new_connections;
    }

    pub fn get_primary_id(&self) -> u32 {
        self.sub_blocks.values().next().map(|sb| sb.id).unwrap_or(0)
    }

    pub fn get_primary_category(&self, registry: &BlockRegistry) -> BlockCategory {
        registry.get(self.get_primary_id()).map(|b| b.category).unwrap_or(BlockCategory::Solid)
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.resolution);
        data.push(self.sub_blocks.len() as u8);

        for ((x, y, z), sub) in &self.sub_blocks {
            data.extend(&[*x, *y, *z]);
            data.extend(sub.id.to_le_bytes());
            data.push(sub.variation);
            data.push(sub.metadata);
            data.push(sub.facing as u8);
            data.push(sub.orientation.serialize());
            data.push(sub.connections.bits());
        }

        data
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, BlockError> {
        let mut index = 0;
        if data.len() < 2 {
            return Err(BlockError::ConnectionError("Insufficient data".into()));
        }
        let resolution = data[index]; index += 1;
        let count = data[index]; index += 1;

        let mut sub_blocks = HashMap::new();
        for _ in 0..count {
            if index + 10 > data.len() {
                return Err(BlockError::ConnectionError("Corrupt data stream".into()));
            }
            let x = data[index]; index += 1;
            let y = data[index]; index += 1;
            let z = data[index]; index += 1;
            let id = u32::from_le_bytes([data[index], data[index+1], data[index+2], data[index+3]]); index += 4;
            let variation = data[index]; index += 1;
            let metadata = data[index]; index += 1;
            let facing = match data[index] {
                0 => BlockFacing::North,
                1 => BlockFacing::South,
                2 => BlockFacing::East,
                3 => BlockFacing::West,
                4 => BlockFacing::Up,
                5 => BlockFacing::Down,
                _ => BlockFacing::None,
            }; index += 1;
            let orientation = BlockOrientation::deserialize(data[index]); index += 1;
            let connections = ConnectedDirections::from_bits_truncate(data[index]); index += 1;

            sub_blocks.insert((x, y, z), SubBlock {
                id,
                variation,
                metadata,
                facing,
                orientation,
                connections,
                material_mod: 1.0,
            });
        }

        Ok(Block {
            sub_blocks,
            resolution,
            current_connections: ConnectedDirections::empty(),
        })
    }
}

impl BlockOrientation {
    fn serialize(&self) -> u8 {
        match self {
            Self::Wall => 0,
            Self::Floor => 1,
            Self::Ceiling => 2,
            Self::Corner => 3,
            Self::Edge => 4,
            Self::Custom(v) => *v,
        }
    }

    fn deserialize(byte: u8) -> Self {
        match byte {
            0 => Self::Wall,
            1 => Self::Floor,
            2 => Self::Ceiling,
            3 => Self::Corner,
            4 => Self::Edge,
            v => Self::Custom(v),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockRegistry {
    blocks: HashMap<u32, BlockDefinition>,
    name_to_id: HashMap<String, u32>,
    category_map: HashMap<BlockCategory, HashSet<u32>>,
}

impl BlockRegistry {
    pub fn initialize_default() -> Self {
        let mut registry = BlockRegistry {
            blocks: HashMap::new(),
            name_to_id: HashMap::new(),
            category_map: HashMap::new(),
        };

        let blocks = include!("blocks_data.rs");
        for def in blocks.into_iter() {
            registry.add_block(def);
        }

        registry
    }

    pub fn add_block(&mut self, def: BlockDefinition) {
        self.name_to_id.insert(def.name.clone(), def.id);
        self.category_map.entry(def.category).or_default().insert(def.id);
        self.blocks.insert(def.id, def);
    }

    pub fn get(&self, id: u32) -> Option<&BlockDefinition> {
        self.blocks.get(&id)
    }
}

#[derive(Error, Debug)]
pub enum BlockError {
    #[error("Invalid block facing")]
    InvalidFacing,
    #[error("Connection error: {0}")]
    ConnectionError(String),
}
