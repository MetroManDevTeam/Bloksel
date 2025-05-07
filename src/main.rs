use anyhow::Result;
use log::{LevelFilter, info};
use simple_logger::SimpleLogger;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use ourvoxelworldproject::{
    config::{
        chunksys::ChunkSysConfig, core::EngineConfig, gameplay::GameplayConfig,
        worldgen::WorldGenConfig,
    },
    engine::VoxelEngine,
};

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

    event_loop.run(move |event, elwt| match event {
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
    })?;

    Ok(())
}
