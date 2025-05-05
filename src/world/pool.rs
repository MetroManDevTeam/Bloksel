use std::sync::Arc;
use std::collections::{HashMap, VecDeque};
use parking_lot::Mutex;
use anyhow::{Result, anyhow};
use super::chunk::{Chunk, ChunkCoord};

/// Thread-safe pool for reusing chunk memory
pub struct ChunkPool {
    available: Mutex<VecDeque<Arc<Chunk>>>,
    in_use: Mutex<HashMap<ChunkCoord, Arc<Chunk>>>,
    template: Arc<Chunk>,
    max_size: usize,
}

impl ChunkPool {
    /// Creates a new pool with base template and maximum size
    pub fn new(base_chunk: Arc<Chunk>, max_size: usize) -> Self {
        Self {
            available: Mutex::new(VecDeque::with_capacity(max_size)),
            in_use: Mutex::new(HashMap::with_capacity(max_size)),
            template: base_chunk,
            max_size,
        }
    }

    /// Acquires a chunk from the pool or creates a new one
    pub fn acquire(&self, coord: ChunkCoord) -> Result<Arc<Chunk>> {
        let mut available = self.available.lock();
        let mut in_use = self.in_use.lock();

        // Try to reuse an available chunk
        if let Some(chunk) = available.pop_front() {
            in_use.insert(coord, chunk.clone());
            return Ok(chunk);
        }

        // Create new chunk if pool isn't full
        if in_use.len() + available.len() < self.max_size {
            let new_chunk = Arc::new(Chunk::from_template(&self.template, coord));
            in_use.insert(coord, new_chunk.clone());
            Ok(new_chunk)
        } else {
            Err(anyhow!("Chunk pool exhausted (max size: {})", self.max_size))
        }
    }

    /// Returns a chunk to the pool
    pub fn release(&self, coord: ChunkCoord) -> Result<()> {
        let mut in_use = self.in_use.lock();
        
        if let Some(chunk) = in_use.remove(&coord) {
            let mut available = self.available.lock();
            
            // Only keep chunk if pool isn't full
            if available.len() < self.max_size {
                chunk.reset(coord);
                available.push_back(chunk);
            }
            Ok(())
        } else {
            Err(anyhow!("Chunk not in use at {:?}", coord))
        }
    }

    /// Pre-allocates chunks in the pool
    pub fn warmup(&self, count: usize) {
        let mut available = self.available.lock();
        let target = count.min(self.max_size - available.len());
        
        for _ in 0..target {
            available.push_back(Arc::new(Chunk::from_template(
                &self.template,
                ChunkCoord::new(0, 0, 0) // Temporary coord
            )));
        }
    }

    /// Calculates current memory usage in bytes
    pub fn current_memory_usage(&self) -> usize {
        let available = self.available.lock().len();
        let in_use = self.in_use.lock().len();
        (available + in_use) * std::mem::size_of::<Chunk>()
    }

    /// Gets current utilization metrics
    pub fn stats(&self) -> PoolStats {
        let available = self.available.lock().len();
        let in_use = self.in_use.lock().len();
        
        PoolStats {
            total_capacity: self.max_size,
            available,
            in_use,
            memory_bytes: self.current_memory_usage(),
        }
    }
}

/// Statistics about pool utilization
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_capacity: usize,
    pub available: usize,
    pub in_use: usize,
    pub memory_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_acquire_release() {
        let template = Arc::new(Chunk::empty(16));
        let pool = ChunkPool::new(template, 10);
        let coord = ChunkCoord::new(1, 2, 3);
        
        // First acquire should create new chunk
        let chunk1 = pool.acquire(coord).unwrap();
        assert_eq!(pool.stats().in_use, 1);
        
        // Release should return to available
        pool.release(coord).unwrap();
        assert_eq!(pool.stats().available, 1);
        
        // Second acquire should reuse
        let chunk2 = pool.acquire(coord).unwrap();
        assert!(Arc::ptr_eq(&chunk1, &chunk2));
    }

    #[test]
    fn test_pool_exhaustion() {
        let template = Arc::new(Chunk::empty(16));
        let pool = ChunkPool::new(template, 2);
        
        let _c1 = pool.acquire(ChunkCoord::new(1, 0, 0)).unwrap();
        let _c2 = pool.acquire(ChunkCoord::new(2, 0, 0)).unwrap();
        
        assert!(pool.acquire(ChunkCoord::new(3, 0, 0)).is_err());
    }
}
