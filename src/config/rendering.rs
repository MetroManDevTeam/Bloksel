use serde::{Serialize, Deserialize};

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

pub struct RenderConfig {
    pub vsync: bool,
    pub fov: f32,
    pub view_distance: f32,
    pub shadow_quality: u32,
    pub texture_atlas_size: u32,
    pub enable_bloom: bool,
    pub enable_ssao: bool,
    pub enable_motion_blur: bool,
}
