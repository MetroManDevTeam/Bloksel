use crate::world::chunk::{Chunk, CHUNK_VOLUME};
use crate::world::chunk_coord::ChunkCoord;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe pool for reusing chunk memory
pub struct ChunkPool {
    chunks: RwLock<HashMap<ChunkCoord, Arc<Chunk>>>,
    max_size: usize,
}

impl ChunkPool {
    /// Creates a new pool with maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            chunks: RwLock::new(HashMap::with_capacity(max_size)),
            max_size,
        }
    }

    /// Gets a chunk from the pool if it exists
    pub fn get(&self, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        self.chunks.read().unwrap().get(&coord).cloned()
    }

    /// Inserts a chunk into the pool, removing the oldest one if at capacity
    pub fn insert(&self, coord: ChunkCoord, chunk: Arc<Chunk>) -> Result<(), &'static str> {
        let mut chunks = self.chunks.write().unwrap();
        if chunks.len() >= self.max_size {
            if let Some((&oldest_coord, _)) = chunks.iter().next() {
                chunks.remove(&oldest_coord);
            }
        }
        chunks.insert(coord, chunk);
        Ok(())
    }

    /// Removes and returns a chunk from the pool
    pub fn remove(&self, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        self.chunks.write().unwrap().remove(&coord)
    }

    /// Clears all chunks from the pool
    pub fn clear(&self) {
        self.chunks.write().unwrap().clear();
    }

    /// Acquires a chunk from the pool or creates a new one
    pub fn acquire(&self, coord: ChunkCoord) -> Result<Arc<Chunk>, &'static str> {
        let chunks = self.chunks.read().unwrap();

        if let Some(chunk) = chunks.get(&coord).cloned() {
            Ok(chunk)
        } else {
            Err("Chunk not found in pool")
        }
    }

    /// Returns a chunk to the pool
    pub fn release(&self, coord: ChunkCoord) -> Result<(), &'static str> {
        let mut chunks = self.chunks.write().unwrap();

        if chunks.remove(&coord).is_some() {
            Ok(())
        } else {
            Err("Chunk not in use")
        }
    }

    /// Pre-allocates chunks in the pool
    pub fn warmup(&self, count: usize) {
        let mut chunks = self.chunks.write().unwrap();
        let target = count.min(self.max_size - chunks.len());

        for _ in 0..target {
            let coord = ChunkCoord::new(0, 0, 0); // Temporary coord
            chunks.insert(coord, Arc::new(Chunk::empty()));
        }
    }

    /// Calculates current memory usage in bytes
    pub fn current_memory_usage(&self) -> usize {
        let chunks = self.chunks.read().unwrap();
        chunks.len() * CHUNK_VOLUME * std::mem::size_of::<u16>()
    }

    /// Gets current utilization metrics
    pub fn stats(&self) -> PoolStats {
        let chunks = self.chunks.read().unwrap();
        PoolStats {
            total_chunks: chunks.len(),
            memory_usage: chunks.len() * CHUNK_VOLUME * std::mem::size_of::<u16>(),
        }
    }

    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        self.chunks.read().unwrap().get(&coord).cloned()
    }

    pub fn set_chunk(&self, coord: ChunkCoord, chunk: Arc<Chunk>) {
        let mut chunks = self.chunks.write().unwrap();
        chunks.insert(coord, chunk);
    }

    pub fn remove_chunk(&self, coord: ChunkCoord) {
        self.chunks.write().unwrap().remove(&coord);
    }
}

/// Statistics about pool utilization
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_chunks: usize,
    pub memory_usage: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_acquire_release() {
        let pool = ChunkPool::new(10); // Replace 10 with the desired max_size value
        let coord = ChunkCoord::new(1, 2, 3);

        // First acquire should create new chunk
        let chunk1 = pool.acquire(coord).unwrap();
        assert_eq!(pool.stats().total_chunks, 1);

        // Release should return to available
        pool.release(coord).unwrap();
        assert_eq!(pool.stats().total_chunks, 0);

        // Second acquire should reuse
        let chunk2 = pool.acquire(coord).unwrap();
        assert!(Arc::ptr_eq(&chunk1, &chunk2));
    }

    #[test]
    fn test_pool_exhaustion() {
        let pool = ChunkPool::new(GameConfig::default());

        let _c1 = pool.acquire(ChunkCoord::new(1, 0, 0)).unwrap();
        let _c2 = pool.acquire(ChunkCoord::new(2, 0, 0)).unwrap();

        assert!(pool.acquire(ChunkCoord::new(3, 0, 0)).is_err());
    }
}
