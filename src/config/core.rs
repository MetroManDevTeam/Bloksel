// core.rs
pub mod rendering;
pub mod worldgen;

pub use rendering::EngineConfig;
pub use worldgen::WorldGenConfig;

/// Common derive macro for configuration types
macro_rules! config_derive {
    ($($t:ty),+) => {
        $(
            #[derive(Clone, Serialize, Deserialize)]
            pub struct $t;
        )+
    };
}
