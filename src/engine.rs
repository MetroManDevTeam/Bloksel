use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::path::{Path, PathBuf};
use std::time::{Instant, Duration};
use std::f32::consts::PI;
use parking_lot::{Mutex, RwLock};
use crossbeam_channel::{bounded, Sender, Receiver};
use rayon::{ThreadPool, ThreadPoolBuilder};
use glam::{Vec3, Vec4, Mat4, IVec3, Vec3Swizzles};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use glam::Vec2;
use crate::{
    player::{Player, PlayerInput, PlayerState},
    chunk::{Chunk, ChunkCoord, SerializedChunk, ChunkMesh},
    block::{BlockRegistry, BlockId},
    chunk_renderer::{ChunkRenderer, RenderStats},
    terrain_generator::{TerrainGenerator, BiomeType},
    shader::ShaderProgram,
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
    load_receiver: Receiver<ChunkCoord>,
    unload_receiver: Receiver<ChunkCoord>,
    
    // State
    running: Arc<AtomicBool>,
    frame_counter: Arc<Mutex<u64>>,
    last_tick: Instant,
    last_save: Instant,
    
    // Configuration
    pub config: EngineConfig,
    pub shader: Arc<ShaderProgram>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub world_seed: u64,
    pub render_distance: u32,
    pub lod_levels: [u32; 3],
    pub chunk_size: u32,
    pub texture_atlas_size: u32,
    pub max_chunk_pool_size: usize,
    pub vsync: bool,
    pub async_loading: bool,
    pub fov: f32,
    pub view_distance: f32,
    pub save_interval: f32,
}

// ========================
// Memory Pooling System
// ========================

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

// ========================
// Spatial Partitioning
// ========================

struct SpatialPartition {
    quadtree: QuadTree,
    lod_state: HashMap<ChunkCoord, u32>,
    spatial_index: BTreeMap<u64, ChunkCoord>,
    last_player_pos: Vec3,
}

impl SpatialPartition {
    fn new(config: &EngineConfig) -> Self {
        Self {
            quadtree: QuadTree::new(config.render_distance),
            lod_state: HashMap::new(),
            spatial_index: BTreeMap::new(),
            last_player_pos: Vec3::ZERO,
        }
    }

    fn update(&mut self, player_pos: Vec3, view_frustum: &ViewFrustum, config: &EngineConfig) {
        if player_pos.distance(self.last_player_pos) > config.chunk_size as f32 * 0.5 {
            self.rebuild_quadtree(player_pos, config);
            self.last_player_pos = player_pos;
        }

        let visible = self.quadtree.query(view_frustum);
        self.update_lod(player_pos, &visible, config);
    }

    fn rebuild_quadtree(&mut self, center: Vec3, config: &EngineConfig) {
        self.quadtree = QuadTree::new(config.render_distance);
        self.spatial_index.clear();
        
        let radius = config.render_distance as i32;
        let center_chunk = ChunkCoord::from_world_pos(center, config.chunk_size);
        
        for x in -radius..=radius {
            for z in -radius..=radius {
                let coord = ChunkCoord {
                    x: center_chunk.x + x,
                    y: center_chunk.y,
                    z: center_chunk.z + z,
                };
                let key = self.spatial_key(coord);
                self.spatial_index.insert(key, coord);
                self.quadtree.insert(coord);
            }
        }
    }

    fn update_lod(&mut self, center: Vec3, visible: &[ChunkCoord], config: &EngineConfig) {
        for coord in visible {
            let distance = self.calculate_distance(center, coord, config.chunk_size);
            let lod = self.calculate_lod_level(distance, config);
            self.lod_state.insert(*coord, lod);
        }
    }

    fn calculate_distance(&self, pos: Vec3, coord: &ChunkCoord, chunk_size: u32) -> f32 {
        let chunk_center = coord.to_world_center(chunk_size);
        pos.distance(chunk_center)
    }

    fn calculate_lod_level(&self, distance: f32, config: &EngineConfig) -> u32 {
        if distance < config.view_distance * 0.3 {
            0
        } else if distance < config.view_distance * 0.6 {
            1
        } else {
            2
        }
    }

    fn spatial_key(&self, coord: ChunkCoord) -> u64 {
        ((coord.x as u64) << 32) | (coord.z as u64)
    }

    fn get_visible_chunks(&self) -> Vec<ChunkCoord> {
        self.spatial_index.values().cloned().collect()
    }

    fn get_loading_priority(&self, player_pos: Vec3, chunk_size: u32) -> Vec<ChunkCoord> {
        let mut chunks: Vec<_> = self.spatial_index.values().cloned().collect();
        chunks.sort_by_key(|c| {
            let center = c.to_world_center(chunk_size);
            (player_pos.distance(center) * 1000.0) as u32
        });
        chunks
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
        let terrain_generator = Arc::new(TerrainGenerator::new(
            config.world_seed as u32,
            block_registry.clone()
        ));
        
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
        let base_chunk = Arc::new(Chunk::empty(config.chunk_size as usize));
        let chunk_pool = Arc::new(ChunkPool::new(base_chunk, config.max_chunk_pool_size));
        let spatial_partition = Arc::new(Mutex::new(SpatialPartition::new(&config)));
        
        // Load shader
        let shader = Arc::new(ShaderProgram::new(
            "shaders/voxel.vert",
            "shaders/voxel.frag"
        )?);

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
            load_receiver: load_recv,
            unload_receiver: unload_recv,
            running: Arc::new(AtomicBool::new(true)),
            frame_counter: Arc::new(Mutex::new(0)),
            last_tick: Instant::now(),
            last_save: Instant::now(),
            config,
            shader,
        };
        
        // Start worker threads
        engine.start_workers();
        Ok(engine)
    }

    fn start_workers(&self) {
        let running = self.running.clone();
        let terrain = self.terrain_generator.clone();
        let active_chunks = self.active_chunks.clone();
        let chunk_pool = self.chunk_pool.clone();
        let load_recv = self.load_receiver.clone();

        // Generation worker
        self.generation_pool.spawn(move || {
            while running.load(Ordering::SeqCst) {
                if let Ok(coord) = load_recv.recv_timeout(Duration::from_millis(100)) {
                    match chunk_pool.acquire(coord) {
                        Ok(chunk) => {
                            if let Err(e) = terrain.generate_into_chunk(&chunk) {
                                log::error!("Failed to generate chunk: {}", e);
                            } else {
                                active_chunks.write().insert(coord, chunk);
                            }
                        }
                        Err(e) => log::error!("Failed to acquire chunk: {}", e),
                    }
                }
            }
        });
        
        // Saving worker
        let running = self.running.clone();
        let active_chunks = self.active_chunks.clone();
        let chunk_pool = self.chunk_pool.clone();
        let unload_recv = self.unload_receiver.clone();

        self.io_pool.spawn(move || {
            while running.load(Ordering::SeqCst) {
                if let Ok(coord) = unload_recv.recv_timeout(Duration::from_millis(100)) {
                    if let Some(chunk) = active_chunks.write().remove(&coord) {
                        if let Err(e) = Self::save_chunk(coord, &chunk) {
                            log::error!("Failed to save chunk: {}", e);
                        }
                        if let Err(e) = chunk_pool.release(coord) {
                            log::error!("Failed to release chunk: {}", e);
                        }
                    }
                }
            }
        });
    }

    pub fn run(&mut self) -> Result<()> {
        let target_frame_time = Duration::from_secs_f32(1.0 / 60.0);
        
        while self.running.load(Ordering::SeqCst) {
            let frame_start = Instant::now();
            
            self.handle_input()?;
            self.update_world()?;
            self.render_frame()?;
            self.auto_save()?;
            
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
        let priority_list = spatial.get_loading_priority(player_pos, self.config.chunk_size);
        
        // Request loading of high-priority chunks
        for coord in priority_list.iter().take(16) {
            if let Err(e) = self.load_queue.send(*coord) {
                log::error!("Failed to queue chunk load: {}", e);
            }
        }
        
        // Unload distant chunks
        let mut active = self.active_chunks.write();
        let to_unload: Vec<_> = active.keys()
            .filter(|c| !visible.contains(c))
            .cloned()
            .collect();
        
        for coord in to_unload {
            active.remove(&coord);
            if let Err(e) = self.unload_queue.send(coord) {
                log::error!("Failed to queue chunk unload: {}", e);
            }
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
        self.chunk_renderer.begin_frame(&view_matrix, &proj_matrix);
        for chunk in chunks_to_render {
            self.chunk_renderer.render_chunk(
                chunk,
                &self.shader,
                &view_matrix,
                &proj_matrix
            )?;
        }
        
        if self.config.debug_mode {
            self.render_debug_info();
        }
        
        self.chunk_renderer.end_frame();
        Ok(())
    }

    fn auto_save(&mut self) -> Result<()> {
        if self.last_save.elapsed().as_secs_f32() > self.config.save_interval {
            self.save_world(Path::new("worlds/current"))?;
            self.last_save = Instant::now();
        }
        Ok(())
    }

    // ========================
    // Physics System
    // ========================

    fn update_physics(&self, delta_time: f32) -> Result<()> {
        let mut player = self.player.lock();
        let input = PlayerInput::default(); // Should come from input system
        
        player.update(
            delta_time,
            &*self.terrain_generator,
            &input
        );
        
        Ok(())
    }

    // ========================
    // Utility Methods
    // ========================
    
    fn calculate_view_frustum(&self) -> ViewFrustum {
        let player = self.player.lock();
        let view_matrix = player.get_view_matrix();
        let proj_matrix = self.calculate_projection_matrix();
        ViewFrustum::from_matrices(&view_matrix, &proj_matrix)
    }
    
    fn calculate_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.config.fov.to_radians(),
            self.window_aspect_ratio(),
            0.1,
            self.config.view_distance
        )
    }
    
    fn window_aspect_ratio(&self) -> f32 {
        16.0 / 9.0 // Should come from window system
    }

    // ========================
    // Serialization
    // ========================

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
        
        self.player.lock().load_state(world_data.player_state);
        Ok(())
    }

    fn save_chunk(coord: ChunkCoord, chunk: &Chunk) -> Result<()> {
        if chunk.modified {
            let serialized = SerializedChunk::from_chunk(coord, chunk);
            serialized.save(&coord.file_path())?;
        }
        Ok(())
    }

    // ========================
    // Debug & Metrics
    // ========================

    pub fn get_stats(&self) -> EngineStats {
        let active = self.active_chunks.read();
        let render_stats = self.chunk_renderer.get_stats();
        
        EngineStats {
            frame_count: *self.frame_counter.lock(),
            active_chunks: active.len(),
            render_stats,
            memory_usage: self.chunk_pool.current_memory_usage(),
            thread_stats: ThreadPoolStats {
                active_threads: self.generation_pool.current_num_threads(),
                queued_tasks: self.load_receiver.len(),
            },
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
// Supporting Types
// ========================

#[derive(Serialize, Deserialize)]
struct WorldSave {
    config: EngineConfig,
    chunks: Vec<SerializedChunk>,
    player_state: PlayerState,
}

#[derive(Default)]
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



#[derive(Default)]
struct Plane {
    normal: Vec3,
    distance: f32,
}

struct AABB {
    min: Vec3,
    max: Vec3,
}

impl QuadTree {
    fn query(&self, frustum: &ViewFrustum) -> Vec<ChunkCoord> {
        let mut visible_chunks = Vec::new();
        
        // First check if this node is visible at all
        if !self.bounds.intersects_frustum(frustum) {
            return visible_chunks;
        }

        // Add all chunks in this node that are visible
        for chunk in &self.chunks {
            let chunk_aabb = AABB {
                min: Vec3::new(
                    chunk.x as f32 * 32.0,
                    0.0,
                    chunk.z as f32 * 32.0
                ),
                max: Vec3::new(
                    (chunk.x + 1) as f32 * 32.0,
                    256.0,
                    (chunk.z + 1) as f32 * 32.0
                ),
            };
            
            if chunk_aabb.intersects_frustum(frustum) {
                visible_chunks.push(*chunk);
            }
        }

        // Recursively check child nodes
        for node in &self.nodes {
            if let Some(child) = node {
                visible_chunks.extend(child.query(frustum));
            }
        }

        visible_chunks
    }
}

// Supporting AABB intersection implementation
impl AABB {
    fn intersects_frustum(&self, frustum: &ViewFrustum) -> bool {
        for plane in &frustum.planes {
            let mut min_point = Vec3::new(self.min.x, self.min.y, self.min.z);
            let mut max_point = Vec3::new(self.max.x, self.max.y, self.max.z);
            
            if plane.normal.x > 0.0 {
                min_point.x = self.max.x;
                max_point.x = self.min.x;
            }
            if plane.normal.y > 0.0 {
                min_point.y = self.max.y;
                max_point.y = self.min.y;
            }
            if plane.normal.z > 0.0 {
                min_point.z = self.max.z;
                max_point.z = self.min.z;
            }
            
            if plane.normal.dot(min_point) + plane.distance < 0.0 {
                return false;
            }
        }
        true
    }
}

// Supporting ViewFrustum plane extraction
impl ViewFrustum {
    fn from_matrices(view: &Mat4, proj: &Mat4) -> Self {
        let vp = proj * view;
        let mut planes = [Plane::default(); 6];
        
        // Left plane
        planes[0].normal = Vec3::new(vp.x_axis[3] + vp.x_axis[0],
                                    vp.y_axis[3] + vp.y_axis[0],
                                    vp.z_axis[3] + vp.z_axis[0]);
        planes[0].distance = vp.w_axis[3] + vp.w_axis[0];
        
        // Right plane
        planes[1].normal = Vec3::new(vp.x_axis[3] - vp.x_axis[0],
                                    vp.y_axis[3] - vp.y_axis[0],
                                    vp.z_axis[3] - vp.z_axis[0]);
        planes[1].distance = vp.w_axis[3] - vp.w_axis[0];
        
        // Bottom plane
        planes[2].normal = Vec3::new(vp.x_axis[3] + vp.x_axis[1],
                                    vp.y_axis[3] + vp.y_axis[1],
                                    vp.z_axis[3] + vp.z_axis[1]);
        planes[2].distance = vp.w_axis[3] + vp.w_axis[1];
        
        // Top plane
        planes[3].normal = Vec3::new(vp.x_axis[3] - vp.x_axis[1],
                                    vp.y_axis[3] - vp.y_axis[1],
                                    vp.z_axis[3] - vp.z_axis[1]);
        planes[3].distance = vp.w_axis[3] - vp.w_axis[1];
        
        // Near plane
        planes[4].normal = Vec3::new(vp.x_axis[3] + vp.x_axis[2],
                                    vp.y_axis[3] + vp.y_axis[2],
                                    vp.z_axis[3] + vp.z_axis[2]);
        planes[4].distance = vp.w_axis[3] + vp.w_axis[2];
        
        // Far plane
        planes[5].normal = Vec3::new(vp.x_axis[3] - vp.x_axis[2],
                                    vp.y_axis[3] - vp.y_axis[2],
                                    vp.z_axis[3] - vp.z_axis[2]);
        planes[5].distance = vp.w_axis[3] - vp.w_axis[2];
        
        // Normalize all planes
        for plane in &mut planes {
            let length = plane.normal.length();
            plane.normal /= length;
            plane.distance /= length;
        }
        
        Self { planes }
    }
}

// Implement Drop for proper cleanup
impl Drop for VoxelEngine {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Err(e) = self.save_world(Path::new("worlds/current")) {
            log::error!("Failed to save world on shutdown: {}", e);
        }
    }
    }
