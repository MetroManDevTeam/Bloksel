use anyhow::Result;
use glutin::{
    config::{Config, ConfigTemplateBuilder},
    context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    prelude::*,
    surface::{Surface, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use log::{LevelFilter, info};
use simple_logger::SimpleLogger;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use ourvoxelworldproject::{config::core::EngineConfig, engine::VoxelEngine};

struct App {
    window: Option<Window>,
    gl_context: Option<PossiblyCurrentContext>,
    gl_surface: Option<Surface<WindowSurface>>,
    engine: VoxelEngine,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_builder = WindowBuilder::new()
            .with_title("Voxel Engine")
            .with_inner_size(LogicalSize::new(1280.0, 720.0));

        // Create window and OpenGL context
        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(true);

        let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));

        let (window, gl_config) = display_builder
            .build(event_loop, template, |configs| {
                configs
                    .reduce(|accum, config| {
                        let transparency_check = config.supports_transparency().unwrap_or(false)
                            & !accum.supports_transparency().unwrap_or(false);
                        if transparency_check || config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .unwrap();

        let window = window.unwrap();
        let raw_window_handle = window.raw_window_handle();

        // Create GL context
        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(None))
            .build(Some(raw_window_handle));

        let gl_display = gl_config.display();
        let gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .expect("Failed to create OpenGL context")
        };

        // Create GL surface
        let attrs = window.build_surface_attributes(<_>::default());
        let gl_surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .expect("Failed to create GL surface")
        };

        // Make context current
        let gl_context = gl_context
            .make_current(&gl_surface)
            .expect("Failed to make context current");

        // Load GL functions
        gl::load_with(|symbol| gl_display.get_proc_address(symbol) as *const _);

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
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
                if let Some(surface) = &self.gl_surface {
                    surface.resize(
                        &self.gl_context.as_ref().unwrap(),
                        size.width as u32,
                        size.height as u32,
                    );
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // TODO: Implement update and render
        // self.engine.update();
        // self.engine.render();
        if let Some(surface) = &self.gl_surface {
            surface
                .swap_buffers(&self.gl_context.as_ref().unwrap())
                .expect("Failed to swap buffers");
        }
    }
}

fn main() -> Result<()> {
    SimpleLogger::new().with_level(LevelFilter::Info).init()?;
    info!("Starting voxel engine...");

    let event_loop = EventLoop::new()?;
    let engine = VoxelEngine::new(EngineConfig::default())?;
    let mut app = App {
        window: None,
        gl_context: None,
        gl_surface: None,
        engine,
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}
