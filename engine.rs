// engine.rs - Large World Core

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::thread;
use parking_lot::Mutex;
use crossbeam_channel::{bounded, Sender, Receiver};
use glam::{Vec3, IVec3};
use chunk::{terrain_generator::Chunk, terrain_generator::ChunkCoord, SerializedChunk,  terrain_generator::ChunkMesh},

pub struct WorldEngine {
    // Chunk management
    active_chunks: Arc<RwLock<HashMap<ChunkCoord, Arc<Chunk>>>>,
    chunk_load_queue: Sender<ChunkCoord>,
    chunk_unload_queue: Sender<ChunkCoord>,
    
    // Thread pools
    generation_pool: rayon::ThreadPool,
    io_pool: rayon::ThreadPool,
    
    // Spatial partitioning
    spatial_grid: Arc<Mutex<QuadTree>>,
    
    // Player context
    player_position: Arc<Mutex<Vec3>>,
    
    // Configuration
    pub render_distance: u32,
    pub lod_levels: [u32; 3],
}

struct ChunkCoord(IVec3);

impl WorldEngine {
    pub fn new() -> Self {
        let (load_send, load_recv) = bounded(1024);
        let (unload_send, unload_recv) = bounded(1024);
        
        let engine = Self {
            active_chunks: Arc::new(RwLock::new(HashMap::new())),
            chunk_load_queue: load_send,
            chunk_unload_queue: unload_send,
            generation_pool: rayon::ThreadPoolBuilder::new()
                .num_threads(4).build().unwrap(),
            io_pool: rayon::ThreadPoolBuilder::new()
                .num_threads(2).build().unwrap(),
            spatial_grid: Arc::new(Mutex::new(QuadTree::new())),
            player_position: Arc::new(Mutex::new(Vec3::ZERO)),
            render_distance: 8,
            lod_levels: [16, 8, 4],
        };
        
        engine.start_workers(load_recv, unload_recv);
        engine
    }

    fn start_workers(&self, load_recv: Receiver<ChunkCoord>, unload_recv: Receiver<ChunkCoord>) {
        // Chunk generation worker
        self.generation_pool.spawn(move || {
            for coord in load_recv.iter() {
                let chunk = Self::generate_chunk(coord);
                Self::insert_chunk(coord, chunk);
            }
        });
        
        // Chunk saving worker
        self.io_pool.spawn(move || {
            for coord in unload_recv.iter() {
                if let Some(chunk) = Self::remove_chunk(coord) {
                    Self::save_chunk(coord, chunk);
                }
            }
        });
    }

    pub fn update(&mut self, delta_time: f32) {
        let player_pos = self.player_position.lock().clone();
        self.update_spatial_grid(player_pos);
        self.manage_chunks(player_pos);
    }

    fn update_spatial_grid(&self, center: Vec3) {
        let mut grid = self.spatial_grid.lock();
        grid.update_center(center);
        
        // Query relevant chunks based on LOD levels
        for lod in &self.lod_levels {
            let area = self.calculate_lod_area(*lod);
            grid.query(area, |coords| {
                self.chunk_load_queue.send(coords).unwrap();
            });
        }
    }

    fn manage_chunks(&self, center: Vec3) {
        let loaded_coords: Vec<ChunkCoord> = self.active_chunks.read()
            .unwrap()
            .keys()
            .cloned()
            .collect();
            
        for coord in loaded_coords {
            let distance = self.calculate_chunk_distance(center, coord);
            
            if distance > self.render_distance as f32 {
                self.chunk_unload_queue.send(coord).unwrap();
            }
        }
    }

    // Chunk generation with LOD
    fn generate_chunk(coord: ChunkCoord) -> Arc<Chunk> {
        let lod_level = Self::determine_lod_level(coord);
        let mut chunk = Chunk::new(
    self.world_config.chunk_size,
    self.world_config.sub_resolution,
    coord  // Add this parameter
);
        
        // Parallel terrain generation
        rayon::scope(|s| {
            for x in 0..32 {
                for z in 0..32 {
                    s.spawn(|_| {
                        let height = generate_height(x, z, lod_level);
                        for y in 0..height {
                            chunk.set_block(x, y, z, determine_block(y));
                        }
                    });
                }
            }
        });
        
        Arc::new(chunk)
    }

    // Memory-conscious spatial partitioning
    struct QuadTree {
        nodes: [Option<Box<QuadTree>>; 4],
        chunks: Vec<ChunkCoord>,
        bounds: WorldArea,
    }
    
    impl QuadTree {
         pub fn new(render_distance: u32) -> Self {
        Self {
            nodes: [None, None, None, None],
            chunks: Vec::new(),
            bounds: AABB {
                min: Vec3::new(-render_distance as f32, 0.0, -render_distance as f32),
                max: Vec3::new(render_distance as f32, 256.0, render_distance as f32),
            },
            depth: 0,
        }
    }

    pub fn insert(&mut self, coord: ChunkCoord) {
        self.chunks.push(coord);
    }
        fn query(&self, area: WorldArea, mut callback: impl FnMut(ChunkCoord)) {
            if self.bounds.intersects(&area) {
                for chunk in &self.chunks {
                    callback(*chunk);
                }
                
                for node in &self.nodes {
                    if let Some(child) = node {
                        child.query(area, &mut callback);
                    }
                }
            }
        }
    }
}

// Usage example for async loading
impl WorldEngine {
    pub fn load_world_region(&self, center: Vec3) {
        let lod_areas = self.lod_levels.iter()
            .map(|lod| self.calculate_lod_area(*lod))
            .collect();
            
        self.schedule_loading(center, lod_areas);
    }
    
    fn schedule_loading(&self, center: Vec3, areas: Vec<WorldArea>) {
        self.generation_pool.spawn(move || {
            let mut spatial_index = SpatialIndex::new(center);
            
            for area in areas {
                spatial_index.query(area, |coords| {
                    self.chunk_load_queue.send(coords).unwrap();
                });
            }
        });
    }
}
