use serde::{Deserialize, Serialize};

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
