use crate::{
    config::core::EngineConfig,
    player::physics::Player,
    render::{pipeline::ChunkRenderer, shaders::ShaderProgram},
    world::{
        block_id::BlockRegistry, chunk::Chunk, chunk_coord::ChunkCoord,
        generator::TerrainGenerator, pool::ChunkPool, spatial::SpatialPartition,
    },
};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use glam::Vec3;
use log::warn;
use parking_lot::Mutex;
use rayon::ThreadPool;
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
    },
    time::Instant,
};

pub struct VoxelEngine {
    // Core systems
    pub block_registry: Arc<BlockRegistry>,
    pub terrain_generator: Arc<TerrainGenerator>,
    pub chunk_renderer: Arc<ChunkRenderer>,
    pub player: Arc<Mutex<Player>>,

    // Chunk management
    active_chunks: Arc<parking_lot::RwLock<HashMap<ChunkCoord, Arc<Chunk>>>>,
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
    frame_counter: Arc<AtomicU64>,
    last_tick: Instant,
    last_save: Instant,

    // Configuration
    pub config: EngineConfig,
}

impl VoxelEngine {
    pub fn new(config: EngineConfig) -> Result<Self> {
        // Initialize core systems
        let block_registry = Arc::new(BlockRegistry::new());
        let terrain_generator = Arc::new(TerrainGenerator::new(
            config.clone(),
            block_registry.clone(),
        ));

        let chunk_renderer = Arc::new(ChunkRenderer::new(
            ShaderProgram::new()?,
            0, // texture_atlas
            block_registry.clone(),
        )?);

        let player = Arc::new(Mutex::new(Player::default()));

        // Setup threading
        let generation_pool = Arc::new(ThreadPool::new()?);
        let io_pool = Arc::new(ThreadPool::new()?);
        let (load_sender, load_receiver) = bounded(100);
        let (unload_sender, unload_receiver) = bounded(100);

        // Initialize chunk systems
        let base_chunk = Arc::new(Chunk::empty());
        let chunk_pool = Arc::new(ChunkPool::new(config.max_chunk_pool_size));
        let spatial_partition = Arc::new(Mutex::new(SpatialPartition::new(&config)));

        Ok(Self {
            block_registry,
            terrain_generator,
            chunk_renderer,
            player,
            active_chunks: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            chunk_pool,
            spatial_partition,
            generation_pool,
            io_pool,
            load_queue: load_sender,
            unload_queue: unload_sender,
            load_receiver,
            unload_receiver,
            running: Arc::new(AtomicBool::new(true)),
            frame_counter: Arc::new(AtomicU64::new(0)),
            last_tick: Instant::now(),
            last_save: Instant::now(),
            config,
        })
    }

    pub fn create_world_config(&mut self, name: String, seed: u64) -> EngineConfig {
        EngineConfig {
            name,
            seed,
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

    pub fn save_world(&self, _path: &Path) -> Result<()> {
        // TODO: Implement world saving
        Ok(())
    }

    pub fn load_world(&mut self, _path: &Path) -> Result<()> {
        // TODO: Implement world loading
        Ok(())
    }

    pub fn get_stats(&self) -> EngineStats {
        EngineStats {
            frame_count: self
                .frame_counter
                .load(std::sync::atomic::Ordering::Relaxed),
            active_chunks: self.active_chunks.read().len(),
            render_stats: RenderStats::default(),
            memory_usage: 0,
            thread_stats: ThreadPoolStats {
                active_threads: 0,
                queued_tasks: 0,
            },
        }
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
    // TODO: Add render statistics
}
