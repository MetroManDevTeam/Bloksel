use crate::config::GameConfig;
use crate::render::MeshBuilder;
use crate::utils::math::{Mat4, Vec3};
use crate::world::ChunkCoord;
use crate::world::chunk::{CHUNK_SIZE, CHUNK_VOLUME, Chunk};
use anyhow::{Result, anyhow};
use parking_lot::Mutex;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Thread-safe pool for reusing chunk memory
pub struct ChunkPool {
    chunks: RwLock<HashMap<ChunkCoord, Arc<Chunk>>>,
    max_size: usize,
}

impl ChunkPool {
    /// Creates a new pool with base template and maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            chunks: RwLock::new(HashMap::new()),
            max_size,
        }
    }

    /// Acquires a chunk from the pool or creates a new one
    pub fn acquire(&self, coord: ChunkCoord) -> Result<Arc<Chunk>> {
        let chunks = self.chunks.read();

        if let Some(chunk) = chunks.get(&coord).cloned() {
            Ok(chunk)
        } else {
            Err(anyhow!("Chunk not found in pool"))
        }
    }

    /// Returns a chunk to the pool
    pub fn release(&self, coord: ChunkCoord) -> Result<()> {
        let mut chunks = self.chunks.write();

        if chunks.remove(&coord).is_some() {
            Ok(())
        } else {
            Err(anyhow!("Chunk not in use at {:?}", coord))
        }
    }

    /// Pre-allocates chunks in the pool
    pub fn warmup(&self, count: usize) {
        let mut chunks = self.chunks.write();
        let target = count.min(self.max_size - chunks.len());

        for _ in 0..target {
            let coord = ChunkCoord::new(0, 0, 0); // Temporary coord
            chunks.insert(
                coord,
                Arc::new(Chunk::from_template(&Chunk::empty(16), coord)),
            );
        }
    }

    /// Calculates current memory usage in bytes
    pub fn current_memory_usage(&self) -> usize {
        let chunks = self.chunks.read();
        chunks.len() * CHUNK_VOLUME * std::mem::size_of::<u16>()
    }

    /// Gets current utilization metrics
    pub fn stats(&self) -> PoolStats {
        let chunks = self.chunks.read();
        PoolStats {
            total_chunks: chunks.len(),
            memory_usage: chunks.len() * CHUNK_VOLUME * std::mem::size_of::<u16>(),
        }
    }

    pub fn get(&self, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        self.chunks.read().get(&coord).cloned()
    }

    pub fn insert(&self, coord: ChunkCoord, chunk: Arc<Chunk>) {
        let mut chunks = self.chunks.write();
        if chunks.len() >= self.max_size {
            // Remove the oldest chunk
            if let Some(oldest) = chunks.keys().next().cloned() {
                chunks.remove(&oldest);
            }
        }
        chunks.insert(coord, chunk);
    }

    pub fn remove(&self, coord: ChunkCoord) {
        self.chunks.write().remove(&coord);
    }

    pub fn clear(&self) {
        self.chunks.write().clear();
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
        let pool = ChunkPool::new(10);
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
        let pool = ChunkPool::new(2);

        let _c1 = pool.acquire(ChunkCoord::new(1, 0, 0)).unwrap();
        let _c2 = pool.acquire(ChunkCoord::new(2, 0, 0)).unwrap();

        assert!(pool.acquire(ChunkCoord::new(3, 0, 0)).is_err());
    }
}
