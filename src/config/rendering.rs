use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    // Rendering
    pub render_distance: u32,
    pub lod_levels: [u32; 3],
    pub texture_atlas_size: u32,
    pub fov: f32,
    pub view_distance: f32,
    pub vsync: bool,
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
