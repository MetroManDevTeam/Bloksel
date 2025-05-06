use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameplayConfig {
    pub max_inventory_slots: u32,
    pub max_stack_size: u32,
    pub break_speed_multiplier: f32,
    pub place_speed_multiplier: f32,
}
