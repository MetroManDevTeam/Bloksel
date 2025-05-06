use crate::{
    config::{
        chunksys::ChunkSysConfig, core::EngineConfig, gameplay::GameplayConfig,
        worldgen::WorldGenConfig,
    },
    player::{
        input::PlayerInput,
        physics::{Player, PlayerState},
    },
    render::{
        pipeline::{ChunkRenderer, RenderPipeline},
        shaders::ShaderProgram,
    },
    utils::{
        error::BlockError,
        math::{Plane, ViewFrustum},
    },
    world::{
        block_id::BlockRegistry,
        chunk::{Chunk, SerializedChunk},
        chunk_coord::ChunkCoord,
        generator::terrain::TerrainGenerator,
        pool::ChunkPool,
        spatial::SpatialPartition,
    },
};
use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender, bounded};
use glam::{Mat4, Vec2, Vec3};
use log::{LevelFilter, info};
use rayon::{ThreadPool, ThreadPoolBuilder};
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

        Ok(engine)
    }

    pub fn create_world_config(&mut self, name: String, seed: u64) -> EngineConfig {
        EngineConfig {
            world_name: name,
            world_seed: seed,
            render_distance: 8,
            lod_levels: [4, 8, 16],
            chunk_size: 32,
            texture_atlas_size: 1024,
            max_chunk_pool_size: 1000,
            vsync: true,
            async_loading: true,
            fov: 70.0,
            view_distance: 1000.0,
            save_interval: 300.0,
            terrain: self.config.terrain.clone(),
            gameplay: self.config.gameplay.clone(),
            rendering: self.config.rendering.clone(),
            chunksys: self.config.chunksys.clone(),
            worldgen: self.config.worldgen.clone(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        // ... existing implementation ...
        Ok(())
    }

    pub fn save_world(&self, path: &Path) -> Result<()> {
        // ... existing implementation ...
        Ok(())
    }

    pub fn load_world(&mut self, path: &Path) -> Result<()> {
        // ... existing implementation ...
        Ok(())
    }

    pub fn get_stats(&self) -> EngineStats {
        // ... existing implementation ...
        EngineStats::default()
    }
}

impl Drop for VoxelEngine {
    fn drop(&mut self) {
        // ... existing implementation ...
    }
}

#[derive(Debug, Default)]
pub struct EngineStats {
    frame_count: u64,
    active_chunks: usize,
    render_stats: RenderStats,
    memory_usage: usize,
    thread_stats: ThreadPoolStats,
}

#[derive(Debug, Default)]
pub struct ThreadPoolStats {
    active_threads: usize,
    queued_tasks: usize,
}

#[derive(Debug, Default)]
pub struct RenderStats {
    // ... existing implementation ...
}
