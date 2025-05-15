use crate::{
    config::{core::EngineConfig, worldgen::WorldGenConfig},
    player::physics::Player,
    render::{
        pipeline::{ChunkRenderer, RenderError},
        shaders::ShaderProgram,
    },
    world::{
        block_id::BlockRegistry as BlockIdRegistry,
        blocks_data::BlockRegistry,
        chunk::Chunk,
        chunk_coord::ChunkCoord,
        generator::terrain::{TerrainGenerator, WorldGenConfig as TerrainWorldGenConfig},
        pool::ChunkPool,
        spatial::SpatialPartition,
    },
};
use anyhow::{Context, Result};
 use ash::vk;
use crossbeam_channel::{Receiver, Sender, bounded};
use glam::Vec3;
use log::warn;
use parking_lot::Mutex;
use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;
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
        let block_registry = Arc::new(BlockRegistry::default());
        let terrain_config = TerrainWorldGenConfig {
            world_seed: config.worldgen.world_seed,
            terrain_height: config.worldgen.terrain_height as i32,
            water_level: config.worldgen.water_level as i32,
            biome_scale: config.worldgen.biome_scale as f64,
            noise_scale: config.worldgen.noise_scale as f64,
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
            height_multiplier: 1.0,
            world_type: crate::world::generator::terrain::WorldType::Normal,
            terrain_amplitude: 1.0,
            cave_threshold: config.worldgen.cave_density as f64,
            flat_world_layers: vec![
                (crate::world::block_id::BlockId::new(1, 0, 0), 1), // Bedrock
                (crate::world::block_id::BlockId::new(2, 0, 0), 3), // Stone
                (crate::world::block_id::BlockId::new(3, 0, 0), 1), // Dirt
                (crate::world::block_id::BlockId::new(4, 0, 0), 1), // Grass
            ],
        };
        let terrain_generator = Arc::new(TerrainGenerator::new(
            terrain_config,
            block_registry.clone(),
        ));

        // Initialize Vulkan renderer with dummy values (will be properly initialized later)
        let chunk_renderer = Arc::new(ChunkRenderer::new(
            ShaderProgram::new("shaders/voxel.vert", "shaders/voxel.frag")?,
            0, // texture_atlas (will be updated when textures are loaded)
            block_registry.clone(),
        ));

        let player = Arc::new(Mutex::new(Player::default()));

        // Setup threading
        let generation_pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .with_context(|| "Failed to create generation pool")?,
        );

        let io_pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(2)
                .build()
                .with_context(|| "Failed to create IO pool")?,
        );
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

    pub fn initialize_vulkan(&mut self, vulkan_context: Arc<crate::render::vulkan::VulkanContext>) -> Result<()> {
        // Reinitialize the chunk renderer with proper Vulkan context
        let new_renderer = ChunkRenderer::new(
            ShaderProgram::new("shaders/voxel.vert", "shaders/voxel.frag")?,
            0,
            self.block_registry.clone(),
        )?;
        
        *Arc::make_mut(&mut self.chunk_renderer) = new_renderer;
        
        // Load all block textures
        self.load_block_textures()?;
        
        Ok(())
    }

    fn load_block_textures(&self) -> Result<()> {
        // Load textures for all registered blocks
        for (block_id, block) in self.block_registry.blocks() {
            if let Some(texture_path) = block.material().texture_path() {
                self.chunk_renderer.load_material(*block_id, block.material().clone())
                    .with_context(|| format!("Failed to load texture for block {}", block_id))?;
            }
        }
        
        // Process the texture queue and upload to GPU
        self.chunk_renderer.process_texture_queue()?;
        
        Ok(())
    }

    pub fn create_world_config(&mut self, name: String, seed: u64) -> EngineConfig {
        EngineConfig {
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

    pub fn save_world(&self, path: &Path) -> Result<()> {
        // TODO: Implement world saving with proper serialization
        warn!("World saving not yet implemented");
        Ok(())
    }

    pub fn load_world(&mut self, path: &Path) -> Result<()> {
        // TODO: Implement world loading with proper deserialization
        warn!("World loading not yet implemented");
        Ok(())
    }

    pub fn get_stats(&self) -> EngineStats {
        EngineStats {
            frame_count: self.frame_counter.load(std::sync::atomic::Ordering::Relaxed),
            active_chunks: self.active_chunks.read().len(),
            render_stats: RenderStats {
                draw_calls: self.chunk_renderer.get_draw_call_count(),
                vertices_rendered: self.chunk_renderer.get_vertex_count(),
                triangles_rendered: self.chunk_renderer.get_triangle_count(),
            },
            memory_usage: 0, // TODO: Implement memory tracking
            thread_stats: ThreadPoolStats {
                active_threads: self.generation_pool.current_num_threads(),
                queued_tasks: self.load_receiver.len() + self.unload_receiver.len(),
            },
        }
    }

    pub fn render_frame(&self, command_buffer: vk::CommandBuffer, camera: &crate::render::core::Camera) {
        
   
        // Reset render statistics    
        self.chunk_renderer.begin_frame();
        
        // Render all active chunks
        let active_chunks = self.active_chunks.read();
        for chunk in active_chunks.values() {
            self.chunk_renderer.render_chunk(
                command_buffer,
                chunk,
                camera,
            );
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        // Update player physics
        let mut player = self.player.lock();
        player.update(delta_time);

        // Update chunk loading based on player position
        self.update_chunk_loading(player.position());
    }

    fn update_chunk_loading(&self, player_position: Vec3) {
        // TODO: Implement chunk loading/unloading logic based on player position
        // and render distance
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

#[derive(Debug)]
pub struct RenderStats {
    draw_calls: usize,
    vertices_rendered: usize,
    triangles_rendered: usize,
}

impl Default for RenderStats {
    fn default() -> Self {
        Self {
            draw_calls: 0,
            vertices_rendered: 0,
            triangles_rendered: 0,
        }
    }
}

#[derive(Debug, Default)]
pub struct ThreadPoolStats {
    active_threads: usize,
    queued_tasks: usize,
    }
