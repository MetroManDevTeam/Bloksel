use anyhow::Result;
use log::{info, LevelFilter};
use simple_logger::SimpleLogger;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod engine;

use engine::{EngineConfig, VoxelEngine};

fn main() -> Result<()> {
    // Initialize logging
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()?;

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
    };

    // Create window and event loop
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Voxel Engine")
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)?;

    // Initialize the engine
    let mut engine = VoxelEngine::new(config)?;

    // Main event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                if let Err(e) = engine.run() {
                    log::error!("Engine error: {}", e);
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    });
}
