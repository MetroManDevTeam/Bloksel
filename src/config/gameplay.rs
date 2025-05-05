use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct EngineConfig {

    // Gameplay
    pub save_interval: f32,
}
