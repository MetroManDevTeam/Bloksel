use crate::world::chunk::Chunk;
use crate::world::chunk_coord::ChunkCoord;
use crate::world::storage::core::ChunkStorage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct FileChunkStorage {
    chunks: HashMap<ChunkCoord, Arc<Chunk>>,
    base_path: PathBuf,
}

impl FileChunkStorage {
    pub fn new(base_path: &str) -> Self {
        Self {
            chunks: HashMap::new(),
            base_path: PathBuf::from(base_path),
        }
    }
}

impl ChunkStorage for FileChunkStorage {
    fn get_chunk(&self, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        self.chunks.get(&coord).cloned()
    }

    fn get_chunk_mut(&mut self, coord: ChunkCoord) -> Option<&mut Arc<Chunk>> {
        self.chunks.get_mut(&coord)
    }

    fn set_chunk(&mut self, coord: ChunkCoord, chunk: Arc<Chunk>) {
        self.chunks.insert(coord, chunk);
    }

    fn remove_chunk(&mut self, coord: ChunkCoord) {
        self.chunks.remove(&coord);
    }
}
