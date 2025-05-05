use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct GameplayConfig {

    // Gameplay
    pub save_interval: f32,
}
