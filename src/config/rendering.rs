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
