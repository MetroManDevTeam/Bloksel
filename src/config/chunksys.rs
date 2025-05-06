use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkSysConfig {
    pub max_chunk_updates_per_frame: u32,
    pub chunk_generation_threads: u32,
    pub chunk_loading_threads: u32,
}
