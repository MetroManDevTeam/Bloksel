// Core Rust
use std::{
    collections::HashMap,
    ops::ControlFlow,
    path::Path,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

// External Crates
use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender, bounded};
use glam::{Mat4, Vec2, Vec3};
use log::{LevelFilter, info};
use rayon::{ThreadPool, ThreadPoolBuilder};
use simple_logger::SimpleLogger;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

// Internal Modules
use crate::{
    config::{
        chunksys::ChunkSysConfig, core::EngineConfig, gameplay::GameplayConfig,
        worldgen::WorldGenConfig,
    },
    engine::VoxelEngine,
    player::{
        input::PlayerInput,
        physics::{Player, PlayerState},
    },
    render::{pipeline::RenderPipeline, shaders::ShaderProgram},
    utils::{
        error::BlockError,
        math::{Plane, Ray, ViewFrustum},
    },
    world::{
        block_id::BlockRegistry,
        chunk::{Chunk, ChunkCoord, SerializedChunk},
        generator::terrain::TerrainGenerator,
        spatial::SpatialPartition,
    },
};

// Re-exports for cleaner usage
pub use crate::{render::pipeline::RenderPipeline, utils::Orientation};
pub use VoxelEngine;

fn main() -> Result<()> {
    // Initialize logging
    SimpleLogger::new().with_level(LevelFilter::Info).init()?;

    info!("Starting voxel engine...");

    // Create engine configuration
    let config = EngineConfig {
        name: "Test World".to_string(),
        seed: 12345,
        render_distance: 16,
        lod_levels: [4, 8, 16],
        chunk_size: 32,
        texture_atlas_size: 1024,
        max_chunk_pool_size: 1024,
        vsync: true,
        async_loading: true,
        fov: 70.0,
        view_distance: 1000.0,
        save_interval: 300.0, // 5 minutes
        terrain: GameplayConfig::default(),
        gameplay: GameplayConfig::default(),
        rendering: GameplayConfig::default(),
        chunksys: ChunkSysConfig::default(),
        worldgen: WorldGenConfig::default(),
    };

    // Create window and event loop
    let event_loop = EventLoop::new()?;
    let window = Window::new(&event_loop)?;

    // Initialize the engine
    let mut engine = VoxelEngine::new(config)?;

    event_loop.run_app(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                elwt.exit();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                engine.resize(size.width, size.height);
            }
            Event::MainEventsCleared => {
                engine.update();
                engine.render();
            }
            _ => (),
        }
    })?;

    Ok(())
}

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

// ========================
// Engine Implementation
// ========================

impl VoxelEngine {
    pub fn new(config: EngineConfig) -> Result<Self> {
        // Initialize core systems
        let block_registry = Arc::new(BlockRegistry::initialize_default());
        let terrain_generator = Arc::new(TerrainGenerator::new(
            config.world_seed as u32,
            block_registry.clone(),
        ));

        let chunk_renderer = Arc::new(ChunkRenderer::new()?);
        let player = Arc::new(Mutex::new(Player::default()));

        // Setup threading
        let generation_pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .context("Failed to create generation pool")?,
        );

        let io_pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(2)
                .build()
                .context("Failed to create IO pool")?,
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
            "shaders/voxel.frag",
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

        spatial.update(player_pos, &view_frustum, &self.config);

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
        let to_unload: Vec<_> = active
            .keys()
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
        let chunks_to_render: Vec<_> = visible_chunks
            .iter()
            .filter_map(|c| active.get(c))
            .collect();

        // Prepare render batch
        self.chunk_renderer.begin_frame(&view_matrix, &proj_matrix);
        for chunk in chunks_to_render {
            self.chunk_renderer
                .render_chunk(chunk, &self.shader, &view_matrix, &proj_matrix)?;
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

        player.update(delta_time, &*self.terrain_generator, &input);

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
            self.config.view_distance,
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
        let chunks: Vec<_> = active
            .iter()
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
            Vec2::new(10.0, 10.0),
        );
        // ... more debug info ...
    }
}

impl Drop for VoxelEngine {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Err(e) = self.save_world(Path::new("worlds/current")) {
            log::error!("Failed to save world on shutdown: {}", e);
        }
    }
}

#[derive(Default)]
struct ThreadPoolStats {
    active_threads: usize,
    queued_tasks: usize,
}

#[derive(Default)]
struct EngineStats {
    frame_count: u64,
    active_chunks: usize,
    render_stats: RenderStats,
    memory_usage: usize,
    thread_stats: ThreadPoolStats,
}
