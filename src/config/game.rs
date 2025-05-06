pub struct TerrainConfig {
    pub seed: u32,
    pub world_scale: f64,
    pub terrain_amplitude: f64,
    pub cave_threshold: f64,
    pub world_type: WorldType,
    pub flat_world_layers: Vec<(BlockId, i32)>, // For flat world generation
}