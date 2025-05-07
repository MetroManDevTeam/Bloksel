use anyhow::Result;
use log::{info, LevelFilter};
use simple_logger::SimpleLogger;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

use ourvoxelworldproject::{
    config::{
        chunksys::ChunkSysConfig, core::EngineConfig, game::TerrainConfig,
        gameplay::GameplayConfig, rendering::RenderConfig, worldgen::WorldGenConfig,
    },
    engine::VoxelEngine,
};

struct App {
    window: Option<Window>,
    engine: VoxelEngine,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes {
            title: "Voxel Engine".to_string(), // Fixed: title is String, not Option<String>
            inner_size: Some(LogicalSize::new(1280.0, 720.0).into()),
            ..Default::default()
        };
        let window = event_loop.create_window(window_attributes).unwrap();
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                info!("Window resized to: {:?}", size);
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // TODO: Implement update and render
        // self.engine.update();
        // self.engine.render();
    }
}

fn main() -> Result<()> {
    SimpleLogger::new().with_level(LevelFilter::Info).init()?;

    info!("Starting voxel engine...");

    let config = EngineConfig {
        world_seed: 12345,
        render_distance: 16,
        lod_levels: [4, 8, 16], // Change to vec![4, 8, 16] if needed by your EngineConfig
        chunk_size: 32,
        texture_atlas_size: 1024,
        max_chunk_pool_size: 1024,
        vsync: true,
        async_loading: true,
        fov: 70.0,
        view_distance: 1000.0,
        save_interval: 300.0,
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

    let event_loop = EventLoop::new()?;
    let engine = VoxelEngine::new(config)?;
    let mut app = App {
        window: None,
        engine,
    };

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app)?;

    Ok(())
}
