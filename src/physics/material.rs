use std::collections::HashMap;
use crate::registry::BlockRegistry;

#[derive(Debug, Clone)]
pub struct MaterialProperties {
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
    pub viscosity: f32,
    pub break_threshold: f32,
}

pub struct PhysicsMaterials {
    materials: HashMap<String, MaterialProperties>,
}

impl PhysicsMaterials {
    pub fn new(registry: &BlockRegistry) -> Self {
        let mut materials = HashMap::new();
        
        // Default materials
        materials.insert("stone".to_string(), MaterialProperties {
            density: 2.7,
            friction: 0.6,
            restitution: 0.1,
            viscosity: 0.0,
            break_threshold: 30.0,
        });
        
        materials.insert("dirt".to_string(), MaterialProperties {
            density: 1.5,
            friction: 0.5,
            restitution: 0.05,
            viscosity: 0.0,
            break_threshold: 10.0,
        });
        
        materials.insert("water".to_string(), MaterialProperties {
            density: 1.0,
            friction: 0.1,
            restitution: 0.0,
            viscosity: 0.7,
            break_threshold: 0.0,
        });

        Self { materials }
    }

    pub fn get(&self, block_id: &str) -> Option<&MaterialProperties> {
        // Extract base material name from block ID
        let base_id = block_id.split('.').next()?;
        self.materials.get(base_id)
    }

    pub fn get_for_block(&self, block_id: &str) -> MaterialProperties {
        self.get(block_id)
            .cloned()
            .unwrap_or_else(|| MaterialProperties {
                density: 1.0,
                friction: 0.5,
                restitution: 0.2,
                viscosity: 0.0,
                break_threshold: 15.0,
            })
    }
}
