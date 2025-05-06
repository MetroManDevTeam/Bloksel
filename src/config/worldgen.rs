use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldGenConfig {
    // World Generation
    pub world_seed: u64,
    pub terrain_height: u32,
    pub water_level: u32,
    pub biome_scale: f32,
    pub noise_scale: f32,
    pub cave_density: f32,
}
