use anyhow::Result;
use glutin::{
    context::{ContextAttributesBuilder, PossiblyCurrentContext},
    display::GetGlDisplay,
    prelude::*,
};
use glutin_winit::GlWindow;
use log::{LevelFilter, info};
use simple_logger::SimpleLogger;
use std::sync::Arc;
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
    gl_context: Option<PossiblyCurrentContext>,
    engine: VoxelEngine,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_title("Voxel Engine")
            .with_inner_size(LogicalSize::new(1280.0, 720.0));
        let window = event_loop.create_window(window_attributes).unwrap();

        // Create OpenGL context
        let context_attributes = ContextAttributesBuilder::new()
            .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 3)))
            .build(Some(window.id()));

        let gl_display = glutin::display::Display::gl();
        let gl_context = unsafe {
            gl_display
                .create_context(&context_attributes)
                .expect("Failed to create OpenGL context")
        };

        // Load OpenGL functions
        gl::load_with(|s| gl_display.get_proc_address(s) as *const _);

        self.window = Some(window);
        self.gl_context = Some(gl_context);
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
                if let Some(context) = &self.gl_context {
                    context.resize(size.into());
                }
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

    let event_loop = EventLoop::new()?;
    let engine = VoxelEngine::new(EngineConfig::default())?;
    let app = App {
        window: None,
        gl_context: None,
        engine,
    };

    event_loop.run_app(Box::new(app))?;
    Ok(())
}
