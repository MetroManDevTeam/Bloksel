use anyhow::Result;
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    prelude::*,
    surface::{Surface, WindowSurface},
};
use glutin_winit::{DisplayBuilder, GlWindow};
use log::{info, LevelFilter};
use raw_window_handle::HasRawWindowHandle;
use simple_logger::SimpleLogger;
use std::num::NonZeroU32;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

use ourvoxelworldproject::{
    config::{
        chunksys::ChunkSysConfig, core::EngineConfig, game::TerrainConfig,
        gameplay::GameplayConfig, rendering::RenderConfig, worldgen::WorldGenConfig,
    },
    engine::VoxelEngine,
};

struct App {
    window: Window,
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
    engine: VoxelEngine,
}

impl App {
    fn new(engine: VoxelEngine) -> Self {
        Self {
            window: Window::new(&EventLoopBuilder::new().build().unwrap()).unwrap(),
            gl_context: PossiblyCurrentContext::new(
                ContextAttributesBuilder::new()
                    .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 3)))
                    .build(Some(unsafe {
                        raw_window_handle::HasRawWindowHandle::raw_window_handle(
                            &Window::new(&EventLoopBuilder::new().build().unwrap()).unwrap(),
                        )
                    })),
            )
            .unwrap(),
            gl_surface: Surface::new(
                &gl_context,
                &window,
                &ConfigTemplateBuilder::new()
                    .with_alpha_size(8)
                    .with_depth_size(24)
                    .with_stencil_size(8)
                    .with_transparency(true)
                    .build(),
            )
            .unwrap(),
            engine,
        }
    }

    fn init(&mut self, event_loop: &EventLoop<()>) {
        self.window.set_inner_size(LogicalSize::new(800, 600));
        self.window.set_title("Bloksel");
        self.window.set_visible(true);

        unsafe {
            self.gl_context.make_current(&self.gl_surface).unwrap();
            gl::load_with(|s| self.gl_context.get_proc_address(s) as *const _);
        }
    }

    fn handle_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                // TODO: Handle window close
            }
            WindowEvent::Resized(size) => {
                self.gl_surface.resize(
                    &self.gl_context,
                    NonZeroU32::new(size.width).unwrap(),
                    NonZeroU32::new(size.height).unwrap(),
                );
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        // TODO: Implement update and render logic
        self.gl_surface.swap_buffers(&self.gl_context).unwrap();
    }
}

fn main() -> Result<()> {
    SimpleLogger::new().with_level(LevelFilter::Info).init()?;
    info!("Starting voxel engine...");

    let event_loop = EventLoopBuilder::new().build()?;
    let mut app = App::new(VoxelEngine::new(EngineConfig {
        world_seed: 12345,
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
        terrain: TerrainConfig::default(),
        gameplay: GameplayConfig::default(),
        rendering: RenderConfig::default(),
        chunksys: ChunkSysConfig::default(),
        worldgen: WorldGenConfig::default(),
    })?);

    app.init(&event_loop);

    event_loop.run(move |event, _| match event {
        Event::WindowEvent {
            event: window_event,
            ..
        } => {
            app.handle_window_event(window_event);
        }
        Event::AboutToWait => {
            app.update();
        }
        _ => (),
    })?;

    Ok(())
}
