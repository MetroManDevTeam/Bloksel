use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainConfig {
    pub block_size: f32,
    pub gravity: f32,
    pub player_height: f32,
    pub player_width: f32,
    pub player_speed: f32,
    pub jump_force: f32,
}
