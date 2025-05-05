
pub mod chunk;
pub mod block;
pub mod storage;
pub mod generator;

use crate::{
    render::MeshBuilder,
    config::GameConfig,
    utils::math::{Vec3, Mat4}
};

pub use chunk::{Chunk, ChunkCoord};
pub use block::{Block, BlockRegistry};

struct ChunkPool {
    available: Mutex<VecDeque<Arc<Chunk>>>,
    in_use: Mutex<HashMap<ChunkCoord, Arc<Chunk>>>,
    template: Arc<Chunk>,
    max_size: usize,
}

impl ChunkPool {
    fn new(base_chunk: Arc<Chunk>, max_size: usize) -> Self {
        Self {
            available: Mutex::new(VecDeque::with_capacity(max_size)),
            in_use: Mutex::new(HashMap::with_capacity(max_size)),
            template: base_chunk,
            max_size,
        }
    }

    fn acquire(&self, coord: ChunkCoord) -> Result<Arc<Chunk>> {
        let mut available = self.available.lock();
        let mut in_use = self.in_use.lock();

        if let Some(chunk) = available.pop_front() {
            in_use.insert(coord, chunk.clone());
            return Ok(chunk);
        }

        if in_use.len() + available.len() < self.max_size {
            let new_chunk = Arc::new(Chunk::from_template(&self.template, coord));
            in_use.insert(coord, new_chunk.clone());
            Ok(new_chunk)
        } else {
            Err(anyhow::anyhow!("Chunk pool exhausted"))
        }
    }

    fn release(&self, coord: ChunkCoord) -> Result<()> {
        let mut in_use = self.in_use.lock();
        if let Some(chunk) = in_use.remove(&coord) {
            let mut available = self.available.lock();
            if available.len() < self.max_size {
                chunk.reset(coord);
                available.push_back(chunk);
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Chunk not in use"))
        }
    }

    fn warmup(&self, count: usize) {
        let mut available = self.available.lock();
        while available.len() < count.min(self.max_size) {
            available.push_back(Arc::new(Chunk::from_template(
                &self.template,
                ChunkCoord::new(0, 0, 0)
            )));
        }
    }

    fn current_memory_usage(&self) -> usize {
        let available = self.available.lock().len();
        let in_use = self.in_use.lock().len();
        (available + in_use) * std::mem::size_of::<Chunk>()
    }
}
