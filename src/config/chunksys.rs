use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ChunkSysConfig {
    pub chunk_size: u32,
    pub render_distance: u32,
    pub max_chunks_per_frame: u32,
    pub chunk_unload_delay: f32,
    pub chunk_load_threads: u32,
}
