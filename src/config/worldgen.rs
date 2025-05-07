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

    // World Settings
    pub world_name: String,
    pub chunk_size: u32,
    pub sub_resolution: u32,
}

impl Default for WorldGenConfig {
    fn default() -> Self {
        Self {
            world_seed: 12345,
            terrain_height: 100,
            water_level: 50,
            biome_scale: 200.0,
            noise_scale: 10.0,
            cave_density: 0.3,
            world_name: String::new(),
            chunk_size: 16,
            sub_resolution: 1,
        }
    }
}
