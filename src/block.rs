// block.rs

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockIntegrity {
    Full,
    Half,
    Quarter,
    Special,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockOrientation {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockDensity {
    Light,
    Medium,
    Heavy,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockPhysics {
    Steady,
    Gravity,
    Passable,
    Slow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDefinition {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub default_integrity: BlockIntegrity,
    #[serde(default)]
    pub default_orientation: BlockOrientation,
    #[serde(default)]
    pub default_density: BlockDensity,
    #[serde(default)]
    pub default_physics: BlockPhysics,
    pub texture_path: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SubBlock {
    pub id: u32,
    pub integrity: BlockIntegrity,
    pub orientation: BlockOrientation,
    pub density: BlockDensity,
    pub physics: BlockPhysics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    grid: HashMap<(u8, u8, u8), SubBlock>, // 3D grid of sub-blocks
    resolution: u8, // Grid size (e.g., 4x4x4)
}

impl Block {
    pub fn uniform(id: u32, resolution: u8, def: &BlockDefinition) -> Self {
        let sub = SubBlock {
            id,
            integrity: def.default_integrity,
            orientation: def.default_orientation,
            density: def.default_density,
            physics: def.default_physics,
        };
        
        let mut grid = HashMap::new();
        for x in 0..resolution {
            for y in 0..resolution {
                for z in 0..resolution {
                    grid.insert((x, y, z), sub.clone());
                }
            }
        }
        
        Self { grid, resolution }
    }

    pub fn encode(&self) -> String {
        if self.is_uniform() {
            return self.grid.values().next().map(|s| s.id.to_string()).unwrap_or_default();
        }

        let mut parts = Vec::new();
        for ((x, y, z), sub) in &self.grid {
            let state = format!(
                "{}{}{}{}",
                sub.integrity_code(),
                sub.orientation_code(),
                sub.density_code(),
                sub.physics_code()
            );
            parts.push(format!("{},{},{}:{}:{}", x, y, z, sub.id, state));
        }
        parts.join("|")
    }

    pub fn decode(s: &str, resolution: u8, registry: &BlockRegistry) -> Self {
        if !s.contains('|') && !s.contains(':') {
            let id = s.parse().unwrap_or(0);
            let def = registry.get(id).unwrap();
            return Self::uniform(id, resolution, def);
        }

        let mut grid = HashMap::new();
        for part in s.split('|') {
            let [pos, id_state] = part.splitn(2, ':').collect::<Vec<_>>();
            let [x, y, z] = pos.splitn(3, ',')
                .map(|v| v.parse().unwrap())
                .collect::<Vec<u8>>();
            
            let [id, state] = id_state.splitn(2, ':').collect::<Vec<_>>();
            let sub = SubBlock {
                id: id.parse().unwrap(),
                integrity: BlockIntegrity::from_code(&state[0..1]),
                orientation: BlockOrientation::from_code(&state[1..2]),
                density: BlockDensity::from_code(&state[2..3]),
                physics: BlockPhysics::from_code(&state[3..4]),
            };
            
            grid.insert((x, y, z), sub);
        }
        
        Self { grid, resolution }
    }

    pub fn is_uniform(&self) -> bool {
        let first = match self.grid.values().next() {
            Some(v) => v,
            None => return true, // All air
        };
        
        self.grid.values().all(|v| v == first)
    }

    pub fn place_sub_block(&mut self, x: u8, y: u8, z: u8, sub: SubBlock) {
        if x >= self.resolution || y >= self.resolution || z >= self.resolution {
            return;
        }
        self.grid.insert((x, y, z), sub);
    }

    pub fn get_sub_block(&self, x: u8, y: u8, z: u8) -> Option<&SubBlock> {
        self.grid.get(&(x, y, z))
    }
}

#[derive(Debug, Clone)]
pub struct BlockRegistry {
    blocks: HashMap<u32, BlockDefinition>,
}

impl BlockRegistry {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let definitions: Vec<BlockDefinition> = serde_json::from_str(&content)?;
        
        let mut blocks = HashMap::new();
        for def in definitions {
            blocks.insert(def.id, def);
        }
        
        Ok(Self { blocks })
    }

    pub fn get(&self, id: u32) -> Option<&BlockDefinition> {
        self.blocks.get(&id)
    }
}
