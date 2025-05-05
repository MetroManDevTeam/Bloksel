// worldgen.rs
use super::config_derive;

config_derive! {
    pub struct WorldGenConfig {
        // World Generation
        pub world_seed: u64,
    }
}
