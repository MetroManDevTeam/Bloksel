use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ChunkSysConfig {

    
    // Chunk System
    pub chunk_size: u32,
    pub max_chunk_pool_size: usize,
    pub async_loading: bool,
    
  
}
