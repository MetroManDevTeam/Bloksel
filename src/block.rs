use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BlockIntegrity {
    Full,
    Half,
    Quarter,
    Special,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BlockOrientation {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BlockDensity {
    Light,
    Medium,
    Heavy,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BlockPhysics {
    Steady,
    Gravity,
    Passable,
    Slow,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone)]
pub struct Block {
    pub id: u32,
    pub integrity: BlockIntegrity,
    pub orientation: BlockOrientation,
    pub density: BlockDensity,
    pub physics: BlockPhysics,
    pub definition: BlockDefinition,
}

impl Block {
    pub fn from_code(code: &str, registry: &BlockRegistry) -> Option<Self> {
        let parts: Vec<&str> = code.split('-').collect();
        if parts.len() != 5 {
            return None;
        }

        let id = parts[0].parse().ok()?;
        let definition = registry.get(id)?;

        Some(Self {
            id,
            integrity: match parts[1] {
                "F" => BlockIntegrity::Full,
                "H" => BlockIntegrity::Half,
                "Q" => BlockIntegrity::Quarter,
                "S" => BlockIntegrity::Special,
                _ => definition.default_integrity,
            },
            orientation: match parts[2] {
                "N" => BlockOrientation::North,
                "E" => BlockOrientation::East,
                "S" => BlockOrientation::South,
                "W" => BlockOrientation::West,
                "U" => BlockOrientation::Up,
                "D" => BlockOrientation::Down,
                _ => definition.default_orientation,
            },
            density: match parts[3] {
                "L" => BlockDensity::Light,
                "M" => BlockDensity::Medium,
                "H" => BlockDensity::Heavy,
                "S" => BlockDensity::Solid,
                _ => definition.default_density,
            },
            physics: match parts[4] {
                "S" => BlockPhysics::Steady,
                "G" => BlockPhysics::Gravity,
                "P" => BlockPhysics::Passable,
                "L" => BlockPhysics::Slow,
                _ => definition.default_physics,
            },
            definition: definition.clone(),
        })
    }

    pub fn to_code(&self) -> String {
        format!(
            "{}-{}-{}-{}-{}",
            self.id,
            self.integrity_code(),
            self.orientation_code(),
            self.density_code(),
            self.physics_code()
        )
    }

    fn integrity_code(&self) -> &str {
        match self.integrity {
            BlockIntegrity::Full => "F",
            BlockIntegrity::Half => "H",
            BlockIntegrity::Quarter => "Q",
            BlockIntegrity::Special => "S",
        }
    }

    fn orientation_code(&self) -> &str {
        match self.orientation {
            BlockOrientation::North => "N",
            BlockOrientation::East => "E",
            BlockOrientation::South => "S",
            BlockOrientation::West => "W",
            BlockOrientation::Up => "U",
            BlockOrientation::Down => "D",
        }
    }

    fn density_code(&self) -> &str {
        match self.density {
            BlockDensity::Light => "L",
            BlockDensity::Medium => "M",
            BlockDensity::Heavy => "H",
            BlockDensity::Solid => "S",
        }
    }

    fn physics_code(&self) -> &str {
        match self.physics {
            BlockPhysics::Steady => "S",
            BlockPhysics::Gravity => "G",
            BlockPhysics::Passable => "P",
            BlockPhysics::Slow => "L",
        }
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

    pub fn create_default_block(&self, id: u32) -> Option<Block> {
        self.get(id).map(|definition| Block {
            id,
            integrity: definition.default_integrity,
            orientation: definition.default_orientation,
            density: definition.default_density,
            physics: definition.default_physics,
            definition: definition.clone(),
        })
    }
}
