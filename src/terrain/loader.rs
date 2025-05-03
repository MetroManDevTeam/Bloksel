use std::collections::{VecDeque, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;
use glam::IVec3;
use crate::core::{Chunk, World};
use crate::terrain::generator::TerrainGenerator;
use rayon::prelude::*;

pub struct ChunkLoader {
    generator: Arc<TerrainGenerator>,
    load_queue: VecDeque<(i32, i32)>,
    unloading_queue: VecDeque<(i32, i32)>,
    active_chunks: HashSet<(i32, i32)>,
    load_radius: i32,
}

impl ChunkLoader {
    pub fn new(generator: Arc<TerrainGenerator>, load_radius: i32) -> Self {
        Self {
            generator,
            load_queue: VecDeque::new(),
            unloading_queue: VecDeque::new(),
            active_chunks: HashSet::new(),
            load_radius,
        }
    }

    pub fn update(&mut self, center: IVec3, world: &Arc<RwLock<World>>) {
        let chunk_x = center.x.div_euclid(Chunk::SIZE as i32);
        let chunk_z = center.z.div_euclid(Chunk::SIZE as i32);

        // Queue chunks for loading
        for x in -self.load_radius..=self.load_radius {
            for z in -self.load_radius..=self.load_radius {
                let pos = (chunk_x + x, chunk_z + z);
                if !self.active_chunks.contains(&pos) {
                    self.load_queue.push_back(pos);
                    self.active_chunks.insert(pos);
                }
            }
        }

        // Queue distant chunks for unloading
        self.active_chunks.retain(|&(x, z)| {
            let should_keep = (x - chunk_x).abs() <= self.load_radius 
                && (z - chunk_z).abs() <= self.load_radius;
            if !should_keep {
                self.unloading_queue.push_back((x, z));
            }
            should_keep
        });

        // Process queues
        self.process_load_queue(world);
        self.process_unload_queue(world);
    }

    fn process_load_queue(&mut self, world: &Arc<RwLock<World>>) {
        let chunks_to_load: Vec<_> = self.load_queue.drain(..).collect();
        
        chunks_to_load.par_iter().for_each(|&(x, z)| {
            let chunk = self.generator.generate_chunk(x, z);
            world.write().load_chunk(x, z, chunk);
        });
    }

    fn process_unload_queue(&mut self, world: &Arc<RwLock<World>>) {
        while let Some((x, z)) = self.unloading_queue.pop_front() {
            world.write().unload_chunk(x, z);
        }
    }
}
