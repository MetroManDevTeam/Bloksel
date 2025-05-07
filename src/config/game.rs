use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainConfig {
    pub noise_scale: f64,
    pub noise_octaves: u32,
    pub noise_persistence: f64,
    pub noise_lacunarity: f64,
    pub height_scale: f64,
    pub sea_level: i32,
    pub mountain_threshold: f64,
    pub plains_threshold: f64,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            noise_scale: 100.0,
            noise_octaves: 4,
            noise_persistence: 0.5,
            noise_lacunarity: 2.0,
            height_scale: 64.0,
            sea_level: 64,
            mountain_threshold: 0.7,
            plains_threshold: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameplayConfig {
    pub player_speed: f32,
    pub sprint_multiplier: f32,
    pub crouch_multiplier: f32,
    pub jump_force: f32,
    pub gravity: f32,
    pub mouse_sensitivity: f32,
}

impl Default for GameplayConfig {
    fn default() -> Self {
        Self {
            player_speed: 5.0,
            sprint_multiplier: 1.5,
            crouch_multiplier: 0.5,
            jump_force: 8.0,
            gravity: 20.0,
            mouse_sensitivity: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderConfig {
    pub shadow_quality: u32,
    pub shadow_distance: f32,
    pub ambient_occlusion: bool,
    pub fog_distance: f32,
    pub fog_density: f32,
    pub bloom_strength: f32,
    pub dof_strength: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            shadow_quality: 2048,
            shadow_distance: 100.0,
            ambient_occlusion: true,
            fog_distance: 500.0,
            fog_density: 0.01,
            bloom_strength: 0.5,
            dof_strength: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkSysConfig {
    pub chunk_size: u32,
    pub max_chunks: usize,
    pub load_distance: u32,
    pub unload_distance: u32,
    pub generation_threads: usize,
    pub io_threads: usize,
}

impl Default for ChunkSysConfig {
    fn default() -> Self {
        Self {
            chunk_size: 32,
            max_chunks: 1000,
            load_distance: 8,
            unload_distance: 12,
            generation_threads: 4,
            io_threads: 2,
        }
    }
}
