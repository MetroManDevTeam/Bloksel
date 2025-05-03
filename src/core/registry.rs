use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use crate::core::block::{Block, BlockProperties, BlockError, BlockIntegrity, BlockOrientation, BlockDensity, BlockPhysics};

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Failed to read block file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse block definition: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Block definition error: {0}")]
    DefinitionError(String),
    #[error("Texture not found: {0}")]
    MissingTexture(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawBlockDefinition {
    base_id: String,
    default_properties: RawBlockProperties,
    variants: Option<HashMap<String, RawVariantOverride>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawBlockProperties {
    friction: f32,
    restitution: f32,
    density: f32,
    viscosity: f32,
    is_solid: bool,
    is_liquid: bool,
    is_transparent: bool,
    break_time: f32,
    light_emission: u8,
    texture: String,
    collision_box: [f32; 6], // min_x, min_y, min_z, max_x, max_y, max_z
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawVariantOverride {
    #[serde(default)]
    friction: Option<f32>,
    #[serde(default)]
    restitution: Option<f32>,
    #[serde(default)]
    density: Option<f32>,
    #[serde(default)]
    is_solid: Option<bool>,
    #[serde(default)]
    texture: Option<String>,
    #[serde(default)]
    collision_box: Option<[f32; 6]>,
}

pub struct BlockRegistry {
    templates: HashMap<String, BlockTemplate>,
    texture_dir: PathBuf,
}

impl BlockRegistry {
    pub fn new(texture_dir: &Path) -> Result<Self, RegistryError> {
        let mut registry = Self {
            templates: HashMap::new(),
            texture_dir: texture_dir.to_path_buf(),
        };

        registry.load_default_blocks()?;
        Ok(registry)
    }

    pub fn load_from_dir(&mut self, dir: &Path) -> Result<(), RegistryError> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                self.load_block_file(&path)?;
            }
        }
        Ok(())
    }

    fn load_block_file(&mut self, path: &Path) -> Result<(), RegistryError> {
        let contents = fs::read_to_string(path)?;
        let raw: RawBlockDefinition = serde_json::from_str(&contents)?;

        // Validate base ID
        if raw.base_id.len() != 1 || !raw.base_id.chars().next().unwrap().is_ascii_digit() {
            return Err(RegistryError::DefinitionError(
                format!("Invalid base ID: {}", raw.base_id)
            ));
        }

        // Convert raw properties
        let default_props = self.convert_properties(&raw.default_properties, None)?;

        // Process variants
        let mut variants = HashMap::new();
        if let Some(raw_variants) = raw.variants {
            for (id_suffix, raw_variant) in raw_variants {
                if !self.validate_variant_id(&id_suffix) {
                    return Err(RegistryError::DefinitionError(
                        format!("Invalid variant ID suffix: {}", id_suffix)
                    ));
                }

                let variant_props = self.convert_properties(
                    &raw.default_properties,
                    Some(&raw_variant)
                )?;
                variants.insert(id_suffix, variant_props);
            }
        }

        self.templates.insert(
            raw.base_id.clone(),
            BlockTemplate {
                base_id: raw.base_id,
                default_properties: default_props,
                variants,
            }
        );

        Ok(())
    }

    fn convert_properties(
        &self,
        base: &RawBlockProperties,
        variant: Option<&RawVariantOverride>,
    ) -> Result<BlockProperties, RegistryError> {
        let mut props = BlockProperties {
            friction: base.friction,
            restitution: base.restitution,
            density: base.density,
            viscosity: base.viscosity,
            is_solid: base.is_solid,
            is_liquid: base.is_liquid,
            is_transparent: base.is_transparent,
            break_time: base.break_time,
            light_emission: base.light_emission,
            texture_path: self.texture_dir.join(&base.texture),
            collision_box: (
                glam::Vec3::new(base.collision_box[0], base.collision_box[1], base.collision_box[2]),
                glam::Vec3::new(base.collision_box[3], base.collision_box[4], base.collision_box[5]),
            ),
        };

        if let Some(v) = variant {
            if let Some(f) = v.friction {
                props.friction = f;
            }
            if let Some(r) = v.restitution {
                props.restitution = r;
            }
            if let Some(d) = v.density {
                props.density = d;
            }
            if let Some(s) = v.is_solid {
                props.is_solid = s;
            }
            if let Some(t) = &v.texture {
                props.texture_path = self.texture_dir.join(t);
            }
            if let Some(b) = v.collision_box {
                props.collision_box = (
                    glam::Vec3::new(b[0], b[1], b[2]),
                    glam::Vec3::new(b[3], b[4], b[5]),
                );
            }
        }

        // Validate texture exists
        if !props.texture_path.exists() && !props.is_transparent {
            return Err(RegistryError::MissingTexture(props.texture_path.clone()));
        }

        Ok(props)
    }

    fn validate_variant_id(&self, id: &str) -> bool {
        // Validate format: [F|H|Q|S][N|S|E|W|U|D][L|M|H|I][0-9.]*[S|P|C|L|B|Y][0-9]*
        let mut chars = id.chars();
        
        // First char: Integrity
        match chars.next() {
            Some('F' | 'H' | 'Q' | 'S') => {},
            _ => return false,
        };
        
        // Second char: Orientation
        match chars.next() {
            Some('N' | 'S' | 'E' | 'W' | 'U' | 'D') => {},
            _ => return false,
        };
        
        // Third char: Density
        match chars.next() {
            Some('L' | 'M' | 'H' | 'I') => {},
            _ => return false,
        };
        
        // Remaining: Physics
        let rest: String = chars.collect();
        if rest.is_empty() {
            return false;
        }
        
        match rest.chars().next().unwrap() {
            'S' | 'P' | 'C' | 'L' | 'B' | 'Y' => {},
            _ => return false,
        }
        
        true
    }

    pub fn get_properties(&self, full_id: &str) -> Option<&BlockProperties> {
        let base_id = full_id.get(0..1)?;
        let variant_suffix = full_id.get(1..)?;
        
        self.templates.get(base_id).and_then(|template| {
            template.variants.get(variant_suffix)
                .or(Some(&template.default_properties))
        })
    }

    pub fn create_block(
        &self,
        full_id: &str,
        position: (i32, i32, i32),
    ) -> Result<Block, BlockError> {
        let properties = self.get_properties(full_id)
            .ok_or(BlockError::UnknownType)?;
        
        Block::new(
            full_id.to_string(),
            position,
            properties.clone(),
        )
    }

    fn load_default_blocks(&mut self) -> Result<(), RegistryError> {
        // Air
        self.templates.insert("0".to_string(), BlockTemplate {
            base_id: "0".to_string(),
            default_properties: BlockProperties {
                friction: 0.0,
                restitution: 0.0,
                density: 0.0,
                viscosity: 0.0,
                is_solid: false,
                is_liquid: false,
                is_transparent: true,
                break_time: 0.0,
                light_emission: 0,
                texture_path: PathBuf::new(),
                collision_box: (glam::Vec3::ZERO, glam::Vec3::ZERO),
            },
            variants: HashMap::new(),
        });

        // Stone (default)
        self.templates.insert("1".to_string(), BlockTemplate {
            base_id: "1".to_string(),
            default_properties: BlockProperties {
                friction: 0.6,
                restitution: 0.1,
                density: 2.7,
                viscosity: 0.0,
                is_solid: true,
                is_liquid: false,
                is_transparent: false,
                break_time: 1.5,
                light_emission: 0,
                texture_path: self.texture_dir.join("stone.png"),
                collision_box: (glam::Vec3::ZERO, glam::Vec3::ONE),
            },
            variants: HashMap::from([
                ("HNM1.0S".to_string(), BlockProperties {
                    is_solid: false,
                    collision_box: (glam::Vec3::ZERO, glam::Vec3::new(1.0, 0.5, 1.0)),
                    ..self.templates["1"].default_properties.clone()
                }),
            ]),
        });

        Ok(())
    }
}

struct BlockTemplate {
    base_id: String,
    default_properties: BlockProperties,
    variants: HashMap<String, BlockProperties>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    fn setup_test_registry() -> BlockRegistry {
        let temp_dir = temp_dir();
        BlockRegistry::new(&temp_dir).unwrap()
    }

    #[test]
    fn test_property_lookup() {
        let registry = setup_test_registry();
        
        // Default stone
        assert!(registry.get_properties("1FNM1.0S").is_some());
        
        // Half stone variant
        assert!(registry.get_properties("1HNM1.0S").is_some());
        
        // Invalid ID
        assert!(registry.get_properties("XZZZ").is_none());
    }

    #[test]
    fn test_block_creation() {
        let registry = setup_test_registry();
        let block = registry.create_block("1FNM1.0S", (0, 0, 0)).unwrap();
        assert_eq!(block.id, "1FNM1.0S");
    }
}
