use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct GameplayConfig {
    pub player_speed: f32,
    pub jump_force: f32,
    pub gravity: f32,
    pub reach_distance: f32,
    pub day_night_cycle_speed: f32,
    pub enable_hunger: bool,
    pub enable_weather: bool,
}
