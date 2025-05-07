use anyhow::Result;
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextAttributesBuilder, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    prelude::*,
    surface::{Surface, WindowSurface},
};
use glutin_winit::{DisplayBuilder, GlWindow};
use log::{info, LevelFilter};
use raw_window_handle::HasRawWindowHandle;
use simple_logger::SimpleLogger;
use std::{ffi::CString, num::NonZeroU32, path::Path, time::Instant};
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
    render::texture::Texture,
};

struct App {
    window: Window,
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
    engine: Option<VoxelEngine>,
    loading_texture: Option<Texture>,
    loading_start: Instant,
}

impl App {
    fn new() -> Result<(Self, EventLoop<()>)> {
        let event_loop = EventLoopBuilder::new().build()?;
        let window_builder = WindowBuilder::new()
            .with_title("Bloksel")
            .with_inner_size(LogicalSize::new(800, 600));

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_depth_size(24)
            .with_stencil_size(8)
            .with_transparency(true);

        let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));

        let (window, gl_config) = display_builder
            .build(&event_loop, template, |configs| {
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

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::OpenGl(None))
            .build(Some(raw_window_handle));

        let gl_display = gl_config.display();

        let gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .expect("Failed to create OpenGL context")
        };

        let attrs = window.build_surface_attributes(<_>::default());
        let gl_surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .expect("Failed to create GL surface")
        };

        let gl_context = gl_context
            .make_current(&gl_surface)
            .expect("Failed to make context current");

        // Load OpenGL functions
        gl::load_with(|symbol| {
            let symbol = CString::new(symbol).unwrap();
            gl_display.get_proc_address(symbol.as_c_str()) as *const _
        });

        // Initialize OpenGL state
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
            gl::FrontFace(gl::CCW);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::ClearColor(0.2, 0.3, 0.3, 1.0);
        }

        // Load loading screen texture
        let loading_texture = Texture::from_file("assets/images/organization.jpg")?;

        Ok((
            Self {
                window,
                gl_context,
                gl_surface,
                engine: None,
                loading_texture: Some(loading_texture),
                loading_start: Instant::now(),
            },
            event_loop,
        ))
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
                unsafe {
                    gl::Viewport(0, 0, size.width as i32, size.height as i32);
                }
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        // Show loading screen if engine is not initialized
        if self.engine.is_none() {
            self.render_loading_screen();
            
            // Initialize engine after a minimum loading time (e.g., 3 seconds)
            if self.loading_start.elapsed().as_secs() >= 3 {
                match VoxelEngine::new(EngineConfig {
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
                }) {
                    Ok(engine) => {
                        self.engine = Some(engine);
                        self.loading_texture = None; // Free loading texture
                    }
                    Err(e) => {
                        log::error!("Failed to initialize engine: {}", e);
                        // TODO: Handle initialization error
                    }
                }
            }
        } else {
            // Normal game rendering
            // TODO: Implement game rendering
        }

        self.gl_surface.swap_buffers(&self.gl_context).unwrap();
    }

    fn render_loading_screen(&self) {
        if let Some(texture) = &self.loading_texture {
            unsafe {
                gl::ClearColor(0.1, 0.1, 0.1, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT);
                
                // Simple quad rendering for the logo
                gl::Enable(gl::TEXTURE_2D);
                texture.bind();
                
                gl::Begin(gl::QUADS);
                gl::TexCoord2f(0.0, 0.0); gl::Vertex2f(-0.5, -0.5);
                gl::TexCoord2f(1.0, 0.0); gl::Vertex2f(0.5, -0.5);
                gl::TexCoord2f(1.0, 1.0); gl::Vertex2f(0.5, 0.5);
                gl::TexCoord2f(0.0, 1.0); gl::Vertex2f(-0.5, 0.5);
                gl::End();
                
                gl::Disable(gl::TEXTURE_2D);
            }
        }
    }
}

fn main() -> Result<()> {
    SimpleLogger::new().with_level(LevelFilter::Info).init()?;
    info!("Starting voxel engine...");

    let (mut app, event_loop) = App::new()?;

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
