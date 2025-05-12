

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
    ptr,
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
use gl::*;
// Simple shader program implementation
struct ShaderProgram {
    id: u32,
}

impl ShaderProgram {
    pub fn new(vertex_src: &str, fragment_src: &str) -> Result<Self> {
        let mut success = gl::FALSE as gl::types::GLint;
        let mut info_log = Vec::with_capacity(512);
        unsafe {
            info_log.set_len(512 - 1); // Ensure space for null terminator
        }

        unsafe {
            // Vertex shader
            let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
            let c_str_vert = CString::new(vertex_src.as_bytes()).unwrap();
            gl::ShaderSource(vertex_shader, 1, &c_str_vert.as_ptr(), ptr::null());
            gl::CompileShader(vertex_shader);
            
            // Check compilation
            gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as gl::types::GLint {
                gl::GetShaderInfoLog(
                    vertex_shader,
                    512,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut gl::types::GLchar,
                );
                return Err(anyhow::anyhow!(
                    "Vertex shader compilation failed: {}",
                    String::from_utf8_lossy(&info_log)
                ));
            }

            // Fragment shader
            let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            let c_str_frag = CString::new(fragment_src.as_bytes()).unwrap();
            gl::ShaderSource(fragment_shader, 1, &c_str_frag.as_ptr(), ptr::null());
            gl::CompileShader(fragment_shader);
            
            // Check compilation
            gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as gl::types::GLint {
                gl::GetShaderInfoLog(
                    fragment_shader,
                    512,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut gl::types::GLchar,
                );
                return Err(anyhow::anyhow!(
                    "Fragment shader compilation failed: {}",
                    String::from_utf8_lossy(&info_log)
                ));
            }

            // Link shaders
            let id = gl::CreateProgram();
            gl::AttachShader(id, vertex_shader);
            gl::AttachShader(id, fragment_shader);
            gl::LinkProgram(id);
            
            // Check linking
            gl::GetProgramiv(id, gl::LINK_STATUS, &mut success);
            if success != gl::TRUE as gl::types::GLint {
                gl::GetProgramInfoLog(
                    id,
                    512,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut gl::types::GLchar,
                );
                return Err(anyhow::anyhow!(
                    "Shader program linking failed: {}",
                    String::from_utf8_lossy(&info_log)
                ));
            }

            // Clean up
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);

            Ok(ShaderProgram { id })
        }
    }

    pub fn use_program(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn set_uniform_mat4(&self, name: &str, value: &[f32; 16]) {
        unsafe {
            let c_name = CString::new(name).unwrap();
            let location = gl::GetUniformLocation(self.id, c_name.as_ptr());
            gl::UniformMatrix4fv(location, 1, gl::FALSE, value.as_ptr());
        }
    }
    
    pub fn set_uniform_int(&self, name: &str, value: i32) {
        unsafe {
            let c_name = CString::new(name).unwrap();
            let location = gl::GetUniformLocation(self.id, c_name.as_ptr());
            gl::Uniform1i(location, value);
        }
    }
    
    pub fn set_uniform_float(&self, name: &str, value: f32) {
        unsafe {
            let c_name = CString::new(name).unwrap();
            let location = gl::GetUniformLocation(self.id, c_name.as_ptr());
            gl::Uniform1f(location, value);
        }
    }
    
    pub fn set_uniform_vec4(&self, name: &str, values: &[f32; 4]) {
        unsafe {
            let c_name = CString::new(name).unwrap();
            let location = gl::GetUniformLocation(self.id, c_name.as_ptr());
            gl::Uniform4fv(location, 1, values.as_ptr());
        }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

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
    // New fields for modern OpenGL
    loading_shader: Option<ShaderProgram>,
    loading_vao: Option<u32>,
    loading_vbo: Option<u32>,
    // Shader for progress bar
    progress_shader: Option<ShaderProgram>,
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
            // Use Core profile for modern OpenGL
            .with_profile(GlProfile::Core)
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
        let gl_loader = |symbol| {
            let symbol = CString::new(symbol).unwrap_or_else(|_| {
                warn!("Failed to create CString for GL symbol");
                CString::new("").unwrap()
            });
            gl_display.get_proc_address(symbol.as_c_str()) as *const _
        };
        
        // Initialize OpenGL bindings
        gl::load_with(gl_loader);

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
        let loading_texture = match Texture::from_file("src/assets/images/organization.jpg") {
            Ok(texture) => {
                info!("Loading texture loaded successfully, dimensions: {}x{}", 
                    texture.width, texture.height);
                Some(texture)
            },
            Err(e) => {
                error!("Failed to load loading texture: {}", e);
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
        
        // Initialize loading shaders and VAO/VBO
        let (loading_shader, loading_vao, loading_vbo) = match Self::init_loading_resources() {
            Ok((shader, vao, vbo)) => {
                info!("Loading resources initialized successfully");
                (Some(shader), Some(vao), Some(vbo))
            },
            Err(e) => {
                warn!("Failed to initialize loading resources: {}", e);
                (None, None, None)
            }
        };
        
        // Initialize progress bar shader
        let progress_shader = match Self::init_progress_shader() {
            Ok(shader) => {
                info!("Progress bar shader initialized successfully");
                Some(shader)
            },
            Err(e) => {
                warn!("Failed to initialize progress bar shader: {}", e);
                None
            }
        };

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
                loading_shader,
                loading_vao,
                loading_vbo,
                progress_shader,
            },
            event_loop,
        ))
    }
    
    // Initialize modern OpenGL resources for the loading screen
    fn init_loading_resources() -> Result<(ShaderProgram, u32, u32)> {
        // Simple vertex shader that transforms vertices and passes texture coordinates
       
        let vertex_shader_src = r#"
            #version 330
            layout (location = 0) in vec3 aPos;
            layout (location = 1) in vec2 aTexCoord;
    
            out vec2 TexCoord;
            uniform mat4 projection;
    
            void main() {
                gl_Position = projection * vec4(aPos, 1.0);
                TexCoord = aTexCoord;
            }
        "#;

        let fragment_shader_src = r#"
            #version 330
            out vec4 FragColor;
    
            in vec2 TexCoord;
            uniform sampler2D texture1;
    
            void main() {
    
                FragColor = texture(texture1, TexCoord);
                if (FragColor.a < 0.1) discard; // Check for transparency issues
            }
        "#;
        
        // Create shader program
        let shader = ShaderProgram::new(vertex_shader_src, fragment_shader_src)?;
        
        // Create VAO and VBO
        let mut vao = 0;
        let mut vbo = 0;
        
        // Set up quad vertices with positions and texture coordinates
        #[rustfmt::skip]
        let vertices: [f32; 20] = [
            // positions      // texture coords
            -1.0, -1.0, 0.0,  0.0, 0.0,  // bottom left
             1.0, -1.0, 0.0,  1.0, 0.0,  // bottom right
            -1.0,  1.0, 0.0,  0.0, 1.0,  // top left
             1.0,  1.0, 0.0,  1.0, 1.0,  // top right
        ];

        unsafe {
            // Generate and bind VAO
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);
            
            // Generate and bind VBO
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            
            // Fill buffer with vertex data
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
                vertices.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
            
            // Position attribute
            gl::VertexAttribPointer(
                0,                           // attribute location
                3,                           // size (3 floats per vertex position)
                gl::FLOAT,                   // type
                gl::FALSE,                   // normalized?
                (5 * std::mem::size_of::<f32>()) as gl::types::GLsizei, // stride
                std::ptr::null(),            // offset of first component
            );
            gl::EnableVertexAttribArray(0);
            
            // Texture coordinate attribute
            gl::VertexAttribPointer(
                1,                           // attribute location
                2,                           // size (2 floats per texture coord)
                gl::FLOAT,                   // type
                gl::FALSE,                   // normalized?
                (5 * std::mem::size_of::<f32>()) as gl::types::GLsizei, // stride
                (3 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid, // offset
            );
            gl::EnableVertexAttribArray(1);
            
            // Unbind
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
        
        Ok((shader, vao, vbo))
    }
    
    // Initialize a simple colored shader for progress bar
    fn init_progress_shader() -> Result<ShaderProgram> {
        // Simple vertex shader that transforms vertices and forwards color
            let vertex_shader_src = r#"
                #version 330
                layout (location = 0) in vec3 aPos;
                uniform mat4 projection;
        
                void main() {
                    gl_Position = projection * vec4(aPos, 1.0);
                }
            "#;
    
            let fragment_shader_src = r#"
                #version 330
                out vec4 FragColor;
                uniform vec4 color;
        
                void main() {
                    FragColor = color;
                }
            "#;
        
        // Create shader program
        ShaderProgram::new(vertex_shader_src, fragment_shader_src)
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
                        self.gl_surface.resize(&self.gl_context, width, height);

                        
                        unsafe {
                            gl::Viewport(0, 0, size.width as i32, size.height as i32);
                            gl::Viewport(0, 0, size.width as i32, size.height as i32);
                            info!("Viewport set to {}x{}", size.width, size.height);
                            
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
    unsafe {
        gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
        Self::gl_check_error("Clear")?;
    }

    // Create proper orthographic projection matrix
    let projection = [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ];

    if let (Some(shader), Some(vao), Some(texture)) = (&self.loading_shader, self.loading_vao, &self.loading_texture) {
        unsafe {
            // Set up proper OpenGL state
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            shader.use_program();
            Self::gl_check_error("UseProgram")?;

            // Set uniforms
            shader.set_uniform_mat4("projection", &projection);
            shader.set_uniform_int("texture1", 0);
            Self::gl_check_error("SetUniforms")?;

            // Bind texture
            gl::ActiveTexture(gl::TEXTURE0);
            texture.bind();
            Self::gl_check_error("BindTexture")?;

            // Draw quad
            gl::BindVertexArray(vao);
            Self::gl_check_error("BindVAO")?;

            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            Self::gl_check_error("DrawArrays")?;

            // Cleanup
            gl::BindVertexArray(0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    } else {
        warn!("Loading resources not available for rendering");
    }

    Ok(())
}


    fn gl_check_error(context: &str) -> Result<()> {
        unsafe {
            let error = gl::GetError();
            if error != gl::NO_ERROR {
                let error_str = match error {
                    gl::INVALID_ENUM => "GL_INVALID_ENUM",
                    gl::INVALID_VALUE => "GL_INVALID_VALUE",
                    gl::INVALID_OPERATION => "GL_INVALID_OPERATION",
                    gl::OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
                    _ => "Unknown GL error",
                };
                return Err(anyhow::anyhow!("OpenGL error in {}: {}", context, error_str));
            }
        }
        Ok(())
    }

    fn cleanup(&mut self) {
        info!("Cleaning up resources...");
        
        // Make sure context is current before cleanup
        let _ = self.gl_context.make_current(&self.gl_surface);
        
        // Clean up OpenGL resources
        unsafe {
            // Clean up VAO and VBO
            if let Some(vao) = self.loading_vao {
                gl::DeleteVertexArrays(1, &vao);
            }
            
            if let Some(vbo) = self.loading_vbo {
                gl::DeleteBuffers(1, &vbo);
            }
        }
        
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
        self.loading_shader = None;
        self.progress_shader = None;
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
                
            println!("Frame rendered - Loading: {}", app.is_loading);

            if let Err(e) = app.update() {
                error!("Error during update: {}", e);
                if e.to_string().contains("failed to swap buffers") || 
                   e.to_string().contains("context lost") {
                    error!("Critical rendering error, exiting");
                   app.cleanup();
                        elwt.exit(); // You probably want to exit here as well
                    }
                }
            }
            _ => {} // You should handle other events or add a default case
        }
    });
    
    Ok(()) // You need to return a Result
}
