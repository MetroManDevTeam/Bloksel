use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct WorldGenConfig {
    // World Generation
    pub world_seed: u64,
}
