use anyhow::Result;
use log::{LevelFilter, info};
use simple_logger::SimpleLogger;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use ourvoxelworldproject::{
    config::{
        chunksys::ChunkSysConfig, core::EngineConfig, game::TerrainConfig,
        gameplay::GameplayConfig, rendering::RenderConfig, worldgen::WorldGenConfig,
    },
    engine::VoxelEngine,
};

fn main() -> Result<()> {
    // Initialize logging
    SimpleLogger::new().with_level(LevelFilter::Info).init()?;

    info!("Starting voxel engine...");

    // Create engine configuration
    let config = EngineConfig {
        world_seed: 12345,
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
        terrain: TerrainConfig {
            block_size: 1.0,
            gravity: 9.81,
            player_height: 1.8,
            player_width: 0.6,
            player_speed: 5.0,
            jump_force: 8.0,
        },
        gameplay: GameplayConfig {
            max_inventory_slots: 36,
            max_stack_size: 64,
            break_speed_multiplier: 1.0,
            place_speed_multiplier: 1.0,
        },
        rendering: RenderConfig {
            enable_shadows: true,
            shadow_resolution: 2048,
            enable_ssao: true,
            enable_fxaa: true,
            enable_bloom: true,
            max_fps: 60,
        },
        chunksys: ChunkSysConfig {
            max_chunk_updates_per_frame: 4,
            chunk_generation_threads: 4,
            chunk_loading_threads: 2,
        },
        worldgen: WorldGenConfig {
            world_seed: 12345,
            terrain_height: 256,
            water_level: 62,
            biome_scale: 0.01,
            noise_scale: 0.01,
            cave_density: 0.5,
            world_name: "Test World".to_string(),
            chunk_size: 32,
            sub_resolution: 8,
        },
    };

    // Create event loop and window
    let event_loop = EventLoop::new()?;
    let window: Window = WindowBuilder::new()
        .with_title("Voxel Engine")
        .with_inner_size(LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)?;

    // Initialize the engine
    let mut engine = VoxelEngine::new(config)?;

    event_loop.run_app(move |event, elwt| {
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
                // TODO: Implement resize handling
                info!("Window resized to: {:?}", size);
            }
            Event::AboutToWait => {
                // TODO: Implement update and render
                // engine.update();
                // engine.render();
            }
            _ => (),
        }
    })?;

    Ok(())
}
