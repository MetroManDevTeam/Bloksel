use anyhow::{Context, Result};
use ash::{version::DeviceV1_0, vk};
use bloksel::{
    config::{
        chunksys::ChunkSysConfig, core::EngineConfig, game::TerrainConfig,
        gameplay::GameplayConfig, rendering::RenderConfig, worldgen::WorldGenConfig,
    },
    engine::VoxelEngine,
    ui::{
        menu::MenuState,
         
        eguiRender::EguiRenderer,
       }, 
    render::vulkan::VulkanContext;
};
use egui::{ClippedPrimitive, Context as EguiContext, TexturesDelta};
use egui_winit::State as EguiWinitState;
use log::{debug, error, info, warn, LevelFilter};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use simple_logger::SimpleLogger;
use std::{
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};
 
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};




struct App {
    window: Window,
    vulkan_context: Arc<VulkanContext>,
    surface: vk::SurfaceKHR,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    swapchain_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,
    engine: Option<VoxelEngine>,
    loading_start: Instant,
    egui_ctx: EguiContext,
    egui_winit: EguiWinitState,
    egui_renderer: EguiRenderer,
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

        let window = window_builder
            .build(&event_loop)
            .context("Failed to create window")?;
        let window_size = (window.inner_size().width, window.inner_size().height);

        // Initialize Vulkan
        let vulkan_settings = vulkan::VulkanSettings {
            application_name: "Bloksel".to_string(),
            engine_name: "Bloksel Engine".to_string(),
            enable_validation: cfg!(debug_assertions),
            ..Default::default()
        };

        let vulkan_context = VulkanContext::new(vulkan_settings)?;
        let vulkan_context = Arc::new(vulkan_context);

        // Create surface
        let surface = vulkan_context.create_surface(&window)?;

        // Create swapchain
        let (swapchain, swapchain_images, swapchain_format, swapchain_extent) =
            vulkan_context.create_swapchain(surface, window_size.0, window_size.1, None)?;

        // Create image views
        let swapchain_image_views = swapchain_images
            .iter()
            .map(|&image| {
                vulkan_context.create_image_view(image, swapchain_format, vk::ImageAspectFlags::COLOR)
            })
            .collect::<Result<Vec<_>>>()?;

        // Create render pass
        let render_pass = vulkan_context.create_render_pass(swapchain_format, None)?;

        // Create framebuffers
        let framebuffers = swapchain_image_views
            .iter()
            .map(|&image_view| {
                vulkan_context.create_framebuffer(render_pass, &[image_view], swapchain_extent.width, swapchain_extent.height)
            })
            .collect::<Result<Vec<_>>>()?;

        // Create command pool
        let command_pool = vulkan_context.create_command_pool(vulkan_context.graphics_queue_family)?;

        // Allocate command buffers
        let command_buffers = vulkan_context
            .allocate_command_buffers(
                command_pool,
                vk::CommandBufferLevel::PRIMARY,
                framebuffers.len() as u32,
            )?;

        // Create sync objects
        let max_frames_in_flight = vulkan_context.settings.max_frames_in_flight;
        let image_available_semaphores = (0..max_frames_in_flight)
            .map(|_| vulkan_context.create_semaphore())
            .collect::<Result<Vec<_>>>()?;
        let render_finished_semaphores = (0..max_frames_in_flight)
            .map(|_| vulkan_context.create_semaphore())
            .collect::<Result<Vec<_>>>()?;
        let in_flight_fences = (0..max_frames_in_flight)
            .map(|_| vulkan_context.create_fence(true))
            .collect::<Result<Vec<_>>>()?;

        // Initialize egui
        let egui_ctx = EguiContext::default();
        let egui_winit = EguiWinitState::new(
            event_loop.create_proxy(),
            egui_ctx.clone(),
            egui::ViewportId::from_hash_of(window.id()),
            Some(window.scale_factor() as f32),
            None,
        );

        // Initialize egui renderer
        let egui_renderer = EguiRenderer::new(&vulkan_context, render_pass)?;

        Ok((
            Self {
                window,
                vulkan_context,
                surface,
                swapchain,
                swapchain_images,
                swapchain_image_views,
                swapchain_format,
                swapchain_extent,
                render_pass,
                framebuffers,
                command_pool,
                command_buffers,
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
                current_frame: 0,
                engine: None,
                loading_start: Instant::now(),
                egui_ctx,
                egui_winit,
                egui_renderer,
                menu_state: MenuState::new(),
                is_loading: true,
                window_size,
            },
            event_loop,
        ))
    }

    fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        // First let egui handle the event
        let response = self.egui_winit.on_window_event(&self.window, event);
        if response.consumed {
            return true;
        }

        match event {
            WindowEvent::CloseRequested => true,
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.window_size = (size.width, size.height);
                    
                    // Recreate swapchain with new size
                    self.recreate_swapchain().unwrap_or_else(|e| {
                        error!("Failed to recreate swapchain: {}", e);
                    });
                }
                false
            }
            _ => false,
        }
    }

    fn recreate_swapchain(&mut self) -> Result<()> {
        unsafe {
            self.vulkan_context.device.device_wait_idle()?;
        }

        // Cleanup old swapchain resources
        self.cleanup_swapchain();

        // Create new swapchain
        let (swapchain, swapchain_images, swapchain_format, swapchain_extent) = self
            .vulkan_context
            .create_swapchain(
                self.surface,
                self.window_size.0,
                self.window_size.1,
                Some(self.swapchain),
            )?;

        self.swapchain = swapchain;
        self.swapchain_images = swapchain_images;
        self.swapchain_format = swapchain_format;
        self.swapchain_extent = swapchain_extent;

        // Recreate image views
        self.swapchain_image_views = self
            .swapchain_images
            .iter()
            .map(|&image| {
                self.vulkan_context
                    .create_image_view(image, self.swapchain_format, vk::ImageAspectFlags::COLOR)
            })
            .collect::<Result<Vec<_>>>()?;

        // Recreate framebuffers
        self.framebuffers = self
            .swapchain_image_views
            .iter()
            .map(|&image_view| {
                self.vulkan_context.create_framebuffer(
                    self.render_pass,
                    &[image_view],
                    self.swapchain_extent.width,
                    self.swapchain_extent.height,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        // Reallocate command buffers
        self.command_buffers = self
            .vulkan_context
            .allocate_command_buffers(
                self.command_pool,
                vk::CommandBufferLevel::PRIMARY,
                self.framebuffers.len() as u32,
            )?;

        Ok(())
    }

    fn cleanup_swapchain(&mut self) {
        unsafe {
            for &framebuffer in &self.framebuffers {
                self.vulkan_context
                    .device
                    .destroy_framebuffer(framebuffer, None);
            }

            for &image_view in &self.swapchain_image_views {
                self.vulkan_context
                    .device
                    .destroy_image_view(image_view, None);
            }

            if self.swapchain != vk::SwapchainKHR::null() {
                self.vulkan_context
                    .swapchain_loader
                    .as_ref()
                    .unwrap()
                    .destroy_swapchain_khr(self.swapchain, None);
            }
        }
    }

    fn update(&mut self) -> Result<()> {
        // Wait for previous frame to finish
        unsafe {
            self.vulkan_context.device.wait_for_fences(
                &[self.in_flight_fences[self.current_frame]],
                true,
                std::u64::MAX,
            )?;
            self.vulkan_context.device.reset_fences(&[self.in_flight_fences[self.current_frame]])?;
        }

        // Acquire next image
        let (image_index, _) = self.vulkan_context.acquire_next_image(
            self.swapchain,
            self.image_available_semaphores[self.current_frame],
            vk::Fence::null(),
        )?;

        // Begin command buffer
        let command_buffer = self.command_buffers[image_index as usize];
        self.vulkan_context.begin_command_buffer(command_buffer)?;

        // Begin render pass
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.2, 0.3, 0.3, 1.0],
            },
        }];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffers[image_index as usize])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_extent,
            })
            .clear_values(&clear_values);

        unsafe {
            self.vulkan_context.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
        }

        // Handle loading state
        if self.engine.is_none() {
            self.render_loading_screen(command_buffer)?;
            
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
                        self.is_loading = false;
                    }
                    Err(e) => {
                        error!("Engine initialization failed: {}", e);
                    }
                }
            }
        } else {
            // Handle menu state after loading completes
            let raw_input = self.egui_winit.take_egui_input(&self.window);
            self.egui_ctx.begin_frame(raw_input);

            // Show menu
            if let Some(engine) = &mut self.engine {
                self.menu_state.show(&self.egui_ctx, engine);
            }

            let full_output = self.egui_ctx.end_frame();
            let clipped_primitives = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

            // Render egui
            self.egui_renderer.render(
                &self.vulkan_context,
                command_buffer,
                &clipped_primitives,
                &full_output.textures_delta,
                self.swapchain_extent.width,
                self.swapchain_extent.height,
            )?;

            self.egui_winit.handle_platform_output(&self.window, full_output.platform_output);
        }

        // End render pass
        unsafe {
            self.vulkan_context.device.cmd_end_render_pass(command_buffer);
            self.vulkan_context.end_command_buffer(command_buffer)?;
        }

        // Submit command buffer
        let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];
        let command_buffers = [command_buffer];

        self.vulkan_context.submit_command_buffers(
            self.vulkan_context.graphics_queue,
            &command_buffers,
            &wait_semaphores,
            &wait_stages,
            &signal_semaphores,
            self.in_flight_fences[self.current_frame],
        )?;

        // Present
        let swapchains = [self.swapchain];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        let result = unsafe {
            self.vulkan_context
                .swapchain_loader
                .as_ref()
                .unwrap()
                .queue_present_khr(self.vulkan_context.present_queue, &present_info)
        };

        if result == Ok(vk::Result::SUBOPTIMAL_KHR) || result == Err(vk::Result::ERROR_OUT_OF_DATE_KHR) {
            self.recreate_swapchain()?;
        } else if let Err(e) = result {
            return Err(anyhow::anyhow!("Failed to present swapchain image: {:?}", e));
        }

        self.current_frame = (self.current_frame + 1) % self.vulkan_context.settings.max_frames_in_flight;

        Ok(())
    }

    fn render_loading_screen(&mut self, command_buffer: vk::CommandBuffer) -> Result<()> {
        // Prepare egui input
        let raw_input = self.egui_winit.take_egui_input(&self.window);
        self.egui_ctx.begin_frame(raw_input);

        // Create loading screen UI
        egui::CentralPanel::default().show(&self.egui_ctx, |ui| {
            ui.heading("Loading Bloksel...");
            ui.add(egui::ProgressBar::new(
                self.loading_start.elapsed().as_secs_f32().min(3.0) / 3.0
            ));
            ui.label("Initializing engine...");
        });

        let full_output = self.egui_ctx.end_frame();
        let clipped_primitives = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        // Render egui
        self.egui_renderer.render(
            &self.vulkan_context,
            command_buffer,
            &clipped_primitives,
            &full_output.textures_delta,
            self.swapchain_extent.width,
            self.swapchain_extent.height,
        )?;

        Ok(())
    }

    fn cleanup(&mut self) {
        info!("Cleaning up resources...");

        unsafe {
            self.vulkan_context.device.device_wait_idle().unwrap();

            // Cleanup swapchain resources
            self.cleanup_swapchain();

            // Cleanup sync objects
            for &semaphore in &self.image_available_semaphores {
                self.vulkan_context.device.destroy_semaphore(semaphore, None);
            }
            for &semaphore in &self.render_finished_semaphores {
                self.vulkan_context.device.destroy_semaphore(semaphore, None);
            }
            for &fence in &self.in_flight_fences {
                self.vulkan_context.device.destroy_fence(fence, None);
            }

            // Cleanup command pool
            self.vulkan_context
                .device
                .destroy_command_pool(self.command_pool, None);

            // Cleanup render pass
            self.vulkan_context
                .device
                .destroy_render_pass(self.render_pass, None);

            // Cleanup surface
            self.vulkan_context
                .surface_loader
                .as_ref()
                .unwrap()
                .destroy_surface_khr(self.surface, None);

            // Cleanup egui renderer
            self.egui_renderer.cleanup(&self.vulkan_context);

            // Cleanup engine
            if let Some(engine) = self.engine.take() {
                drop(engine);
            }
        }

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
            _ => {}
        }
    });
    
    Ok(())
}
