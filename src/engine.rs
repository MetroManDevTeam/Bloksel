// engine.rs - Complete Voxel Engine Core

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::path::{Path, PathBuf};
use std::time::{Instant, Duration};
use parking_lot::{Mutex, RwLock, RwLockUpgradableReadGuard};
use crossbeam_channel::{bounded, Sender, Receiver, select};
use rayon::{ThreadPool, ThreadPoolBuilder};
use glam::{Vec3, Vec4, Mat4, IVec3, EulerRot};
use image::{RgbaImage, ImageBuffer};
use anyhow::{Result, Context};
use crate::{
    player::Player,
    chunk::{Chunk, ChunkCoord, SerializedChunk},
    block::{BlockRegistry, BlockId},
    renderer::{ChunkRenderer, RenderStats},
    terrain::TerrainGenerator,
};

// ========================
// Core Engine Structure
// ========================

pub struct VoxelEngine {
    // Core systems
    pub block_registry: Arc<BlockRegistry>,
    pub terrain_generator: Arc<TerrainGenerator>,
    pub chunk_renderer: Arc<ChunkRenderer>,
    pub player: Arc<Mutex<Player>>,
    
    // Chunk management
    active_chunks: Arc<RwLock<HashMap<ChunkCoord, Arc<Chunk>>>>,
    chunk_pool: Arc<ChunkPool>,
    spatial_partition: Arc<Mutex<SpatialPartition>>,
    
    // Threading
    generation_pool: Arc<ThreadPool>,
    io_pool: Arc<ThreadPool>,
    load_queue: Sender<ChunkCoord>,
    unload_queue: Sender<ChunkCoord>,
    
    // State
    running: Arc<AtomicBool>,
    frame_counter: Arc<Mutex<u64>>,
    last_tick: Instant,
    
    // Configuration
    pub config: EngineConfig,
}

#[derive(Clone)]
pub struct EngineConfig {
    pub world_seed: u64,
    pub render_distance: u32,
    pub lod_levels: [u32; 3],
    pub chunk_size: u32,
    pub texture_atlas_size: u32,
    pub max_chunk_pool_size: usize,
    pub vsync: bool,
    pub async_loading: bool,
}

// ========================
// Memory Pooling System
// ========================

struct ChunkPool {
    available: Mutex<VecDeque<Arc<Chunk>>>,
    in_use: Mutex<HashMap<ChunkCoord, Arc<Chunk>>>,
    template: Arc<Chunk>,
}

impl ChunkPool {
    fn new(base_chunk: Arc<Chunk>, max_size: usize) -> Self {
        Self {
            available: Mutex::new(VecDeque::with_capacity(max_size)),
            in_use: Mutex::new(HashMap::with_capacity(max_size)),
            template: base_chunk,
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
            let new_chunk = Arc::new(Chunk::from_template(&self.template));
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
                available.push_back(chunk);
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Chunk not in use"))
        }
    }

    fn warmup(&self, count: usize) {
        let mut available = self.available.lock();
        while available.len() < count {
            available.push_back(Arc::new(Chunk::from_template(&self.template)));
        }
    }
}

// ========================
// Spatial Partitioning
// ========================

struct SpatialPartition {
    quadtree: QuadTree,
    lod_state: HashMap<ChunkCoord, u32>,
    spatial_index: BTreeMap<u64, ChunkCoord>,
}

impl SpatialPartition {
    fn update(&mut self, player_pos: Vec3, view_frustum: &ViewFrustum, config: &EngineConfig) {
        let visible = self.quadtree.query(view_frustum);
        self.update_lod(player_pos, &visible, config);
        self.rebalance_tree();
    }

    fn update_lod(&mut self, center: Vec3, visible: &[ChunkCoord], config: &EngineConfig) {
        for coord in visible {
            let distance = self.calculate_distance(center, coord);
            let lod = self.calculate_lod_level(distance, config);
            self.lod_state.insert(*coord, lod);
        }
    }

    fn rebalance_tree(&mut self) {
        // Implementation of quad tree rebalancing
    }
}

struct QuadTree {
    nodes: [Option<Box<QuadTree>>; 4],
    chunks: Vec<ChunkCoord>,
    bounds: AABB,
    depth: u8,
}

// ========================
// Engine Implementation
// ========================

impl VoxelEngine {
    pub fn new(config: EngineConfig) -> Result<Self> {
        // Initialize core systems
        let block_registry = Arc::new(BlockRegistry::initialize_default());
        let terrain_generator = Arc::new(TerrainGenerator::new(config.world_seed));
        let chunk_renderer = Arc::new(ChunkRenderer::new(config.texture_atlas_size)?);
        let player = Arc::new(Mutex::new(Player::default()));
        
        // Setup threading
        let generation_pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .context("Failed to create generation pool")?
        );
        
        let io_pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(2)
                .build()
                .context("Failed to create IO pool")?
        );
        
        // Create communication channels
        let (load_send, load_recv) = bounded(1024);
        let (unload_send, unload_recv) = bounded(1024);
        
        // Initialize chunk systems
        let base_chunk = Arc::new(Chunk::empty(config.chunk_size));
        let chunk_pool = Arc::new(ChunkPool::new(base_chunk, config.max_chunk_pool_size));
        let spatial_partition = Arc::new(Mutex::new(SpatialPartition::new()));
        
        let engine = Self {
            block_registry,
            terrain_generator,
            chunk_renderer,
            player,
            active_chunks: Arc::new(RwLock::new(HashMap::new())),
            chunk_pool,
            spatial_partition,
            generation_pool,
            io_pool,
            load_queue: load_send,
            unload_queue: unload_send,
            running: Arc::new(AtomicBool::new(true))),
            frame_counter: Arc::new(Mutex::new(0))),
            last_tick: Instant::now(),
            config,
        };
        
        // Start worker threads
        engine.start_workers(load_recv, unload_recv);
        Ok(engine)
    }

    fn start_workers(&self, load_recv: Receiver<ChunkCoord>, unload_recv: Receiver<ChunkCoord>) {
        // Chunk generation worker
        let terrain = self.terrain_generator.clone();
        let pool = self.generation_pool.clone();
        let active = self.active_chunks.clone();
        
        pool.spawn(move || {
            for coord in load_recv.iter() {
                let chunk = match terrain.generate_chunk(coord) {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("Failed to generate chunk: {}", e);
                        continue;
                    }
                };
                
                let mut active = active.write();
                active.insert(coord, Arc::new(chunk));
            }
        });
        
        // Chunk saving worker
        let io_pool = self.io_pool.clone();
        let active = self.active_chunks.clone();
        
        io_pool.spawn(move || {
            for coord in unload_recv.iter() {
                let mut active = active.write();
                if let Some(chunk) = active.remove(&coord) {
                    if let Err(e) = Self::save_chunk(coord, &chunk) {
                        log::error!("Failed to save chunk: {}", e);
                    }
                }
            }
        });
    }

    pub fn run(&mut self) -> Result<()> {
        let target_frame_time = Duration::from_secs_f32(1.0 / 60.0);
        
        while self.running.load(Ordering::SeqCst) {
            let frame_start = Instant::now();
            
            // Update systems
            self.handle_input()?;
            self.update_world()?;
            self.render_frame()?;
            
            // Maintain target frame rate
            let elapsed = frame_start.elapsed();
            if elapsed < target_frame_time {
                std::thread::sleep(target_frame_time - elapsed);
            }
            
            *self.frame_counter.lock() += 1;
        }
        
        Ok(())
    }

    fn update_world(&mut self) -> Result<()> {
        let delta_time = self.last_tick.elapsed().as_secs_f32();
        self.last_tick = Instant::now();
        
        // Update player
        let player_pos = self.player.lock().position;
        self.update_spatial_partition(player_pos)?;
        self.stream_chunks(player_pos)?;
        
        // Update physics
        self.update_physics(delta_time)?;
        
        Ok(())
    }

    fn update_spatial_partition(&self, player_pos: Vec3) -> Result<()> {
        let view_frustum = self.calculate_view_frustum();
        let mut spatial = self.spatial_partition.lock();
        
        spatial.update(
            player_pos,
            &view_frustum,
            &self.config
        );
        
        Ok(())
    }

    fn stream_chunks(&self, player_pos: Vec3) -> Result<()> {
        let spatial = self.spatial_partition.lock();
        let visible = spatial.get_visible_chunks();
        let priority_list = spatial.get_loading_priority();
        
        // Request loading of high-priority chunks
        for coord in priority_list.iter().take(16) {
            self.load_queue.send(*coord)?;
        }
        
        // Unload distant chunks
        let mut active = self.active_chunks.write();
        let to_unload: Vec<_> = active.keys()
            .filter(|c| !visible.contains(c))
            .cloned()
            .collect();
        
        for coord in to_unload {
            active.remove(&coord);
            self.unload_queue.send(coord)?;
        }
        
        Ok(())
    }

    fn render_frame(&self) -> Result<()> {
        let player = self.player.lock();
        let view_matrix = player.get_view_matrix();
        let proj_matrix = self.calculate_projection_matrix();
        
        let visible_chunks = {
            let spatial = self.spatial_partition.lock();
            spatial.get_visible_chunks()
        };
        
        let active = self.active_chunks.read();
        let chunks_to_render: Vec<_> = visible_chunks.iter()
            .filter_map(|c| active.get(c))
            .collect();
        
        // Prepare render batch
        self.chunk_renderer.begin_frame(view_matrix, proj_matrix);
        self.chunk_renderer.render_batch(chunks_to_render);
        
        if self.config.debug_mode {
            self.render_debug_info();
        }
        
        self.chunk_renderer.end_frame();
        Ok(())
    }

    // ========================
    // Utility Methods
    // ========================
    
    fn calculate_view_frustum(&self) -> ViewFrustum {
        let player = self.player.lock();
        player.get_view_frustum()
    }
    
    fn calculate_projection_matrix(&self) -> Mat4 {
        // Implementation based on window size and FOV
        Mat4::perspective_rh(
            45.0f32.to_radians(),
            self.window_aspect_ratio(),
            0.1,
            1000.0
        )
    }
    
    fn window_aspect_ratio(&self) -> f32 {
        // Implementation based on window manager
        16.0 / 9.0
    }
}

// ========================
// Serialization
// ========================

impl VoxelEngine {
    pub fn save_world(&self, path: &Path) -> Result<()> {
        let active = self.active_chunks.read();
        let chunks: Vec<_> = active.iter()
            .map(|(coord, chunk)| SerializedChunk::from_chunk(*coord, chunk))
            .collect();
        
        let world_data = WorldSave {
            config: self.config.clone(),
            chunks,
            player_state: self.player.lock().save_state(),
        };
        
        world_data.save(path)
    }

    pub fn load_world(&mut self, path: &Path) -> Result<()> {
        let world_data = WorldSave::load(path)?;
        self.config = world_data.config;
        
        for chunk in world_data.chunks {
            let coord = chunk.coord;
            let loaded = Chunk::from_serialized(chunk)?;
            
            let mut active = self.active_chunks.write();
            active.insert(coord, Arc::new(loaded));
        }
        
        Ok(())
    }
}

// ========================
// Debug & Metrics
// ========================

impl VoxelEngine {
    pub fn get_stats(&self) -> EngineStats {
        let active = self.active_chunks.read();
        let render_stats = self.chunk_renderer.get_stats();
        
        EngineStats {
            frame_count: *self.frame_counter.lock(),
            active_chunks: active.len(),
            render_stats,
            memory_usage: self.chunk_pool.current_memory_usage(),
            thread_stats: self.get_thread_stats(),
        }
    }

    fn render_debug_info(&self) {
        let stats = self.get_stats();
        self.chunk_renderer.draw_text(
            format!("FPS: {:.1}", 1.0 / self.last_tick.elapsed().as_secs_f32()),
            Vec2::new(10.0, 10.0)
        );
        // ... more debug info ...
    }
}

// ========================
// Input Handling
// ========================

impl VoxelEngine {
    fn handle_input(&mut self) -> Result<()> {
        // Implementation would interface with window system
        // Handle keyboard/mouse events
        Ok(())
    }
}

// ========================
// Physics System
// ========================

impl VoxelEngine {
    fn update_physics(&self, delta_time: f32) -> Result<()> {
        let mut player = self.player.lock();
        player.update_physics(
            delta_time,
            &*self.terrain_generator,
            &self.active_chunks.read()
        );
        
        Ok(())
    }
}

// ========================
// Supporting Types
// ========================

#[derive(Serialize, Deserialize)]
struct WorldSave {
    config: EngineConfig,
    chunks: Vec<SerializedChunk>,
    player_state: PlayerState,
}

struct EngineStats {
    frame_count: u64,
    active_chunks: usize,
    render_stats: RenderStats,
    memory_usage: usize,
    thread_stats: ThreadPoolStats,
}

struct ViewFrustum {
    planes: [Plane; 6],
}

struct AABB {
    min: Vec3,
    max: Vec3,
}
