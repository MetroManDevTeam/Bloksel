use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkSysConfig {
    pub chunk_size: u32,
    pub max_chunks: usize,
    pub load_distance: u32,
    pub unload_distance: u32,
    pub generation_threads: usize,
    pub io_threads: usize,
}

impl Default for ChunkSysConfig {
    fn default() -> Self {
        Self {
            chunk_size: 32,
            max_chunks: 1000,
            load_distance: 8,
            unload_distance: 12,
            generation_threads: 4,
            io_threads: 2,
        }
    }
}
