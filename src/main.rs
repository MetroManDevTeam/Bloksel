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
                    .expect("No suitable OpenGL configurations found")
            })
            .map_err(|e| anyhow::anyhow!("Failed to build display: {}", e))?;

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


    // This is a replacement for the `render_loading_screen` method that uses modern OpenGL
// instead of the fixed-function pipeline that may not be supported in core profile contexts.

fn render_loading_screen(&self) -> Result<()> {
    // Simple shader program for rendering textured quads
    static mut SHADER_PROGRAM: Option<u32> = None;
    static mut VAO: Option<u32> = None;
    static mut VBO: Option<u32> = None;
    
    unsafe {
        // Initialize shaders if not already done
        if SHADER_PROGRAM.is_none() {
            // Vertex shader
            let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
            let vertex_src = CString::new(r#"
                #version 330 core
                layout (location = 0) in vec3 aPos;
                layout (location = 1) in vec2 aTexCoord;
                
                out vec2 TexCoord;
                
                void main() {
                    gl_Position = vec4(aPos, 1.0);
                    TexCoord = aTexCoord;
                }
            "#).unwrap();
            
            gl::ShaderSource(vertex_shader, 1, &vertex_src.as_ptr(), std::ptr::null());
            gl::CompileShader(vertex_shader);
            
            // Check for vertex shader compilation errors
            let mut success = 0;
            gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut info_log = vec![0u8; 512];
                let mut log_len = 0;
                gl::GetShaderInfoLog(vertex_shader, 512, &mut log_len, info_log.as_mut_ptr() as *mut i8);
                let error_msg = String::from_utf8_lossy(&info_log[0..log_len as usize]);
                error!("Vertex shader compilation failed: {}", error_msg);
                return Err(anyhow::anyhow!("Shader compilation failed"));
            }
            
            // Fragment shader
            let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            let fragment_src = CString::new(r#"
                #version 330 core
                out vec4 FragColor;
                
                in vec2 TexCoord;
                
                uniform sampler2D texture1;
                
                void main() {
                    FragColor = texture(texture1, TexCoord);
                }
            "#).unwrap();
            
            gl::ShaderSource(fragment_shader, 1, &fragment_src.as_ptr(), std::ptr::null());
            gl::CompileShader(fragment_shader);
            
            // Check for fragment shader compilation errors
            gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut info_log = vec![0u8; 512];
                let mut log_len = 0;
                gl::GetShaderInfoLog(fragment_shader, 512, &mut log_len, info_log.as_mut_ptr() as *mut i8);
                let error_msg = String::from_utf8_lossy(&info_log[0..log_len as usize]);
                error!("Fragment shader compilation failed: {}", error_msg);
                return Err(anyhow::anyhow!("Shader compilation failed"));
            }
            
            // Link shaders
            let shader_program = gl::CreateProgram();
            gl::AttachShader(shader_program, vertex_shader);
            gl::AttachShader(shader_program, fragment_shader);
            gl::LinkProgram(shader_program);
            
            // Check for linking errors
            gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
            if success == 0 {
                let mut info_log = vec![0u8; 512];
                let mut log_len = 0;
                gl::GetProgramInfoLog(shader_program, 512, &mut log_len, info_log.as_mut_ptr() as *mut i8);
                let error_msg = String::from_utf8_lossy(&info_log[0..log_len as usize]);
                error!("Shader program linking failed: {}", error_msg);
                return Err(anyhow::anyhow!("Shader linking failed"));
            }
            
            // Clean up shaders
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
            
            // Save program ID
            SHADER_PROGRAM = Some(shader_program);
            
            // Set up vertex data
            let vertices: [f32; 20] = [
                // positions     // texture coords
                0.2, 0.2, 0.0,   0.0, 0.0, // bottom left
                0.8, 0.2, 0.0,   1.0, 0.0, // bottom right
                0.8, 0.8, 0.0,   1.0, 1.0, // top right
                0.2, 0.8, 0.0,   0.0, 1.0  // top left
            ];
            
            let indices: [u32; 6] = [
                0, 1, 2,  // first triangle
                2, 3, 0   // second triangle
            ];
            
            // Create VAO, VBO, EBO
            let (mut vao, mut vbo, mut ebo) = (0, 0, 0);
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);
            
            // Bind VAO
            gl::BindVertexArray(vao);
            
            // Bind VBO and copy vertex data
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<f32>()) as isize,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW
            );
            
            // Bind EBO and copy index data
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * std::mem::size_of::<u32>()) as isize,
                indices.as_ptr() as *const _,
                gl::STATIC_DRAW
            );
            
            // Set up vertex attributes
            // Position attribute
            gl::VertexAttribPointer(
                0, 
                3, 
                gl::FLOAT, 
                gl::FALSE, 
                5 * std::mem::size_of::<f32>() as i32, 
                std::ptr::null()
            );
            gl::EnableVertexAttribArray(0);
            
            // Texture coord attribute
            gl::VertexAttribPointer(
                1, 
                2, 
                gl::FLOAT, 
                gl::FALSE, 
                5 * std::mem::size_of::<f32>() as i32, 
                (3 * std::mem::size_of::<f32>()) as *const () as *const _
            );
            gl::EnableVertexAttribArray(1);
            
            // Unbind
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
            
            // Save VAO and VBO
            VAO = Some(vao);
            VBO = Some(vbo);
        }
        
        // Actual drawing
        if let Some(texture) = &self.loading_texture {
            // Use shader program
            gl::UseProgram(SHADER_PROGRAM.unwrap());
            
            // Bind texture
            texture.bind();
            
            // Draw
            gl::BindVertexArray(VAO.unwrap());
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
            gl::BindVertexArray(0);
        } else {
            // No texture available, draw simple progress bar
            // Use shader program (but without texture)
            gl::UseProgram(SHADER_PROGRAM.unwrap());
            
            // Calculate progress
            let progress = (self.loading_start.elapsed().as_secs_f32() / 3.0).min(1.0);
            
            // Create progress bar vertices
            let vertices: [f32; 12] = [
                // positions (no texture coords for simple colored bar)
                0.3, 0.48, 0.0,                    // bottom left
                0.3 + 0.4 * progress, 0.48, 0.0,   // bottom right
                0.3 + 0.4 * progress, 0.52, 0.0,   // top right
                0.3, 0.52, 0.0                     // top left
            ];
            
            // Update VBO with new vertices
            gl::BindBuffer(gl::ARRAY_BUFFER, VBO.unwrap());
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                (12 * std::mem::size_of::<f32>()) as isize,
                vertices.as_ptr() as *const _
            );
            
            // Draw progress bar
            gl::BindVertexArray(VAO.unwrap());
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
            gl::BindVertexArray(0);
        }
        
        // Unbind shader
        gl::UseProgram(0);
    }
    
    Ok(())
}

    fn cleanup(&mut self) {
        info!("Cleaning up resources...");
        
        // Make sure context is current before cleanup
        let _ = self.gl_context.make_current(&self.gl_surface);
        
        // Clean up egui painter
        if let Some(mut painter) = self.painter.take() {
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
        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            if let Err(e) = app.update() {
                error!("Error during update: {}", e);
                if e.to_string().contains("failed to swap buffers") || 
                   e.to_string().contains("context lost") {
                    error!("Critical rendering error, exiting");
                    app.cleanup();
                    elwt.exit();
                }
            }
        }
        Event::LoopExiting => {
            app.cleanup();
        }
        _ => (),
    }
}).context("Event loop terminated unexpectedly")?;

    Ok(())
            }
