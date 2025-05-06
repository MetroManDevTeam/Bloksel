use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct WorldGenConfig {
    // World Generation
    pub world_seed: u64,
    pub generate_structures: bool,
    pub generate_caves: bool,
    pub generate_ores: bool,
    pub generate_trees: bool,
    pub tree_density: f32,
    pub ore_density: f32,
    pub structure_frequency: f32,
}
