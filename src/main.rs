use anyhow::{Context, Result};
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, GlProfile, PossiblyCurrentContext, Version},
    display::{GetGlDisplay, GlDisplay},
    prelude::*,
    surface::{Surface, WindowSurface},
};
use glutin_winit::{DisplayBuilder, GlWindow};
use log::{debug, error, info, warn, LevelFilter};
use raw_window_handle::HasRawWindowHandle;
use simple_logger::SimpleLogger;
use std::{
    ffi::{CStr, CString},
    num::NonZeroU32,
    sync::Arc,
    time::Instant,
};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

use bloksel::{
    config::{
        chunksys::ChunkSysConfig, core::EngineConfig, game::TerrainConfig,
        gameplay::GameplayConfig, rendering::RenderConfig, worldgen::WorldGenConfig,
    },
    engine::VoxelEngine,
    render::texture::Texture,
    ui::menu::MenuState,
};

struct App {
    window: Window,
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
    engine: Option<VoxelEngine>,
    loading_texture: Option<Texture>,
    loading_start: Instant,
    egui_ctx: Option<egui::Context>,
    egui_winit: Option<egui_winit::State>,
    glow_context: Option<Arc<glow::Context>>,
    painter: Option<egui_glow::Painter>,
    menu_state: MenuState,
    is_loading: bool,
    window_size: (u32, u32),
}

impl App {
    fn new() -> Result<(Self, EventLoop<()>)> {
        SimpleLogger::new()
            .with_level(LevelFilter::Info)
            .init()
            .context("Failed to initialize logger")?;
            
        info!("Initializing application...");

        let event_loop = EventLoopBuilder::new()
            .build()
            .context("Failed to create event loop")?;
            
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
                            && !accum.supports_transparency().unwrap_or(false);
                        if transparency_check || config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .ok_or_else(|| anyhow::anyhow!("No suitable OpenGL configurations found"))
            })
            .context("Failed to build display")?;

        let window = window.context("Failed to create window")?;
        let raw_window_handle = window.raw_window_handle();
        let window_size = (window.inner_size().width, window.inner_size().height);

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .with_profile(GlProfile::Compatibility)
            .build(Some(raw_window_handle));

        let gl_display = gl_config.display();

        let gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .context("Failed to create OpenGL context")?
        };

        let attrs = window.build_surface_attributes(<_>::default());
        let gl_surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .context("Failed to create GL surface")?
        };

        let gl_context = gl_context
            .make_current(&gl_surface)
            .context("Failed to make context current")?;

        // Load OpenGL functions
        gl::load_with(|symbol| {
            let symbol = CString::new(symbol).unwrap_or_else(|_| {
                warn!("Failed to create CString for GL symbol");
                CString::new("").unwrap()
            });
            gl_display.get_proc_address(symbol.as_c_str()) as *const _
        });

        // Initialize OpenGL state safely with error checking
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
            gl::FrontFace(gl::CCW);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::ClearColor(0.2, 0.3, 0.3, 1.0);
            
            // Check for OpenGL errors after initialization
            let gl_error = gl::GetError();
            if gl_error != gl::NO_ERROR {
                warn!("OpenGL error during initialization: 0x{:X}", gl_error);
            }
        }

        // Load loading screen texture
        let loading_texture = match Texture::from_file("assets/images/organization.png") {
            Ok(texture) => {
                info!("Loading texture loaded successfully");
                Some(texture)
            },
            Err(e) => {
                warn!("Failed to load loading texture: {}", e);
                None
            }
        };

        // Initialize egui
        let egui_ctx = egui::Context::default();
        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::from_hash_of(window.id()),
            &event_loop,
            None,
            None,
        );

        // Create glow context
        let glow_context = Arc::new(unsafe {
            glow::Context::from_loader_function(|s| {
                let c_str = std::ffi::CStr::from_ptr(s.as_ptr() as *const i8);
                gl_display.get_proc_address(c_str) as *const _
            })
        });

        Ok((
            Self {
                window,
                gl_context,
                gl_surface,
                engine: None,
                loading_texture,
                loading_start: Instant::now(),
                egui_ctx: Some(egui_ctx),
                egui_winit: Some(egui_winit),
                glow_context: Some(glow_context),
                painter: None,
                menu_state: MenuState::new(),
                is_loading: true,
                window_size,
            },
            event_loop,
        ))
    }

    fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        // First let egui handle the event
        if let Some(egui_winit) = &mut self.egui_winit {
            let response = egui_winit.on_window_event(&self.window, event);
            if response.consumed {
                return true;
            }
        }

        match event {
            WindowEvent::CloseRequested => true,
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.window_size = (size.width, size.height);
                    
                    // Create non-zero sizes safely
                    if let (Some(width), Some(height)) = (
                        NonZeroU32::new(size.width), 
                        NonZeroU32::new(size.height)
                    ) {
                        if let Err(e) = self.gl_surface.resize(&self.gl_context, width, height) {
                            error!("Failed to resize GL surface: {}", e);
                        }
                        
                        unsafe {
                            gl::Viewport(0, 0, size.width as i32, size.height as i32);
                            
                            // Check for OpenGL errors after resizing
                            let gl_error = gl::GetError();
                            if gl_error != gl::NO_ERROR {
                                warn!("OpenGL error during resize: 0x{:X}", gl_error);
                            }
                        }
                    } else {
                        warn!("Attempted to resize window to invalid dimensions: {}x{}", 
                             size.width, size.height);
                    }
                } else {
                    warn!("Ignoring resize event with zero dimension: {}x{}", 
                         size.width, size.height);
                }
                false
            }
            _ => false,
        }
    }

    fn update(&mut self) -> Result<()> {
        // Clear the screen
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            
            // Check for OpenGL errors
            let gl_error = gl::GetError();
            if gl_error != gl::NO_ERROR {
                warn!("OpenGL error during clear: 0x{:X}", gl_error);
            }
        }

        // Handle loading state
        if self.engine.is_none() {
            self.render_loading_screen()?;
            
            if self.loading_start.elapsed().as_secs() >= 3 {
                info!("Loading period elapsed, initializing engine...");
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
                        info!("Engine initialized successfully");
                        self.engine = Some(engine);
                        self.loading_texture = None; // Free up memory
                        self.is_loading = false;
                    }
                    Err(e) => {
                        error!("Engine initialization failed: {}", e);
                        // Continue showing loading screen but with error feedback
                    }
                }
            }
        } 
        // Handle menu state after loading completes
        else {
            let egui_ctx = match &self.egui_ctx {
                Some(ctx) => ctx,
                None => {
                    warn!("Missing egui context during update");
                    return Ok(());
                }
            };

            let egui_input = match &mut self.egui_winit {
                Some(winit) => winit.take_egui_input(&self.window),
                None => {
                    warn!("Missing egui winit during update");
                    return Ok(());
                }
            };

            egui_ctx.begin_frame(egui_input);

            // Initialize painter if needed
            if self.painter.is_none() {
                if let Some(glow_context) = &self.glow_context {
                    match egui_glow::Painter::new(glow_context.clone(), "", None) {
                        Ok(painter) => {
                            self.painter = Some(painter);
                        }
                        Err(e) => {
                            error!("Failed to create egui painter: {}", e);
                        }
                    }
                }
            }

            // Show menu
            if let Some(engine) = &mut self.engine {
                self.menu_state.show(egui_ctx, engine);
            }

            let full_output = egui_ctx.end_frame();
            let clipped_primitives = egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

            if let Some(painter) = &mut self.painter {
                painter.paint_and_update_textures(
                    [self.window_size.0, self.window_size.1],
                    self.window.scale_factor() as f32,
                    &clipped_primitives,
                    &full_output.textures_delta,
                );
            }

            if let Some(egui_winit) = &mut self.egui_winit {
                egui_winit.handle_platform_output(&self.window, full_output.platform_output);
            }
        }

        // Swap buffers safely
        self.gl_surface
            .swap_buffers(&self.gl_context)
            .context("Failed to swap buffers")?;
            
        Ok(())
    }

    fn render_loading_screen(&self) -> Result<()> {
        if let Some(texture) = &self.loading_texture {
            unsafe {
                // Set up orthogonal projection
                gl::MatrixMode(gl::PROJECTION);
                gl::LoadIdentity();
                gl::Ortho(0.0, 1.0, 0.0, 1.0, -1.0, 1.0);
                gl::MatrixMode(gl::MODELVIEW);
                gl::LoadIdentity();

                gl::Enable(gl::TEXTURE_2D);
                texture.bind();

                // Simple quad rendering
                gl::Begin(gl::QUADS);
                gl::TexCoord2f(0.0, 0.0); gl::Vertex2f(0.2, 0.2);
                gl::TexCoord2f(1.0, 0.0); gl::Vertex2f(0.8, 0.2);
                gl::TexCoord2f(1.0, 1.0); gl::Vertex2f(0.8, 0.8);
                gl::TexCoord2f(0.0, 1.0); gl::Vertex2f(0.2, 0.8);
                gl::End();

                gl::Disable(gl::TEXTURE_2D);
                
                // Check for OpenGL errors
                let gl_error = gl::GetError();
                if gl_error != gl::NO_ERROR {
                    warn!("OpenGL error during loading screen render: 0x{:X}", gl_error);
                }
            }
        } else {
            // Fallback loading text using a safer approach
            unsafe {
                gl::Color3f(1.0, 1.0, 1.0);
                gl::RasterPos2f(0.4, 0.5);
                
                // Check if the glutBitmap functions are available
                #[cfg(feature = "glutin_text")]
                {
                    for c in "Loading...".chars() {
                        if let Ok(c_u8) = u8::try_from(c as u32) {
                            glutin::glutin::glutBitmapCharacter(
                                glutin::glutin::glutBitmap8By13(),
                                c_u8
                            );
                        }
                    }
                }
                
                // Alternative if no glutBitmap
                #[cfg(not(feature = "glutin_text"))]
                {
                    debug!("glutBitmap functions not available, not rendering text");
                    // Draw a loading bar instead
                    gl::Begin(gl::QUADS);
                    gl::Vertex2f(0.3, 0.48);
                    gl::Vertex2f(0.3 + 0.4 * (self.loading_start.elapsed().as_secs_f32() / 3.0).min(1.0), 0.48);
                    gl::Vertex2f(0.3 + 0.4 * (self.loading_start.elapsed().as_secs_f32() / 3.0).min(1.0), 0.52);
                    gl::Vertex2f(0.3, 0.52);
                    gl::End();
                }
                
                // Check for OpenGL errors
                let gl_error = gl::GetError();
                if gl_error != gl::NO_ERROR {
                    warn!("OpenGL error during fallback loading render: 0x{:X}", gl_error);
                }
            }
        }
        
        Ok(())
    }

    fn cleanup(&mut self) {
        info!("Cleaning up resources...");
        
        // Make sure context is current before cleanup
        let _ = self.gl_context.make_current(&self.gl_surface);
        
        // Clean up egui painter
        if let Some(painter) = self.painter.take() {
            painter.destroy();
        }
        
        // Clean up engine resources
        if let Some(engine) = self.engine.take() {
            // Assuming VoxelEngine implements Drop, otherwise add explicit cleanup
            drop(engine);
        }
        
        // Clean up other resources
        self.loading_texture = None;
        self.egui_ctx = None;
        self.egui_winit = None;
        self.glow_context = None;
        
        info!("Cleanup complete");
    }
}

fn main() -> Result<()> {
    let (mut app, event_loop) = App::new().context("Failed to initialize application")?;
    info!("Application initialized, starting event loop");

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => {
                if app.handle_window_event(&event) {
                    info!("Window close requested");
                    app.cleanup();
                    elwt.exit();
                }
            }
            Event::AboutToWait => {
                app.window.request_redraw();
            }
            Event::RedrawRequested(..) => {
                if let Err(e) = app.update() {
                    error!("Error during update: {}", e);
                    // Only exit on critical errors
                    if e.to_string().contains("failed to swap buffers") || 
                       e.to_string().contains("context lost") {
                        error!("Critical rendering error, exiting");
                        app.cleanup();
                        elwt.exit();
                    }
                }
            }
            _ => (),
        }
    }).context("Event loop terminated unexpectedly")?;

    Ok(())
            }
