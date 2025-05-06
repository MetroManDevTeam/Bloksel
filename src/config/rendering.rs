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
    pub enable_shadows: bool,
    pub shadow_resolution: u32,
    pub enable_ssao: bool,
    pub enable_fxaa: bool,
    pub enable_bloom: bool,
    pub max_fps: u32,
}
