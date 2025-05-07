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
    window: Option<Window>,
    gl_context: Option<PossiblyCurrentContext>,
    gl_surface: Option<Surface<WindowSurface>>,
    engine: VoxelEngine,
}

impl App {
    fn new(engine: VoxelEngine) -> Self {
        Self {
            window: None,
            gl_context: None,
            gl_surface: None,
            engine,
        }
    }

    fn init(&mut self, event_loop: &EventLoop<()>) {
        let window_builder = WindowBuilder::new()
            .with_title("Voxel Engine")
            .with_inner_size(LogicalSize::new(1280.0, 720.0));

        // Create window and OpenGL context
        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
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

        // Create GL context
        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(None))
            .build(Some(window.raw_window_handle()));

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
        gl::load_with(|symbol| {
            let symbol = std::ffi::CString::new(symbol).unwrap();
            gl_display.get_proc_address(symbol.as_c_str()) as *const _
        });

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
    }

    fn handle_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                std::process::exit(0);
            }
            WindowEvent::Resized(size) => {
                info!("Window resized to: {:?}", size);
                if let Some(surface) = &self.gl_surface {
                    surface.resize(
                        &self.gl_context.as_ref().unwrap(),
                        NonZeroU32::new(size.width as u32).unwrap(),
                        NonZeroU32::new(size.height as u32).unwrap(),
                    );
                }
            }
            _ => (),
        }
    }

    fn update(&mut self) {
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

    event_loop.run(move |event, window_target| match event {
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
