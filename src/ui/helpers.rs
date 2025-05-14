use std::path::{Path, PathBuf};
use crate::{
    ui::world::WorldMeta,
    render::vulkan::{VulkanContext, vk}
};
use egui::{Color32, Ui, Button, Response, Layout, Align, Spinner, ProgressBar, Window, Frame};
use ash::vk;

// Standardized UI components

/// Creates a standardized button with consistent styling
pub fn button(ui: &mut Ui, text: &str) -> Response {
    ui.add_sized(
        [200.0, 40.0],
        Button::new(text).fill(Color32::from_rgb(40, 40, 40))
}

/// Creates a small secondary button
pub fn small_button(ui: &mut Ui, text: &str) -> Response {
    ui.add_sized(
        [120.0, 30.0],
        Button::new(text).fill(Color32::from_rgb(60, 60, 60))
    )
}

/// Shows the engine logo with version
pub fn logo(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.heading("BLOKSEL");
        ui.add_space(10.0);
        ui.label("Version 1.0.0");
    });
}

/// Creates a loading spinner with progress bar
pub fn loading_spinner(ui: &mut Ui, current_task: &str, progress: f32) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.heading("Loading...");
        ui.add_space(20.0);
        ui.add(Spinner::new().size(50.0));
        
        ui.add(ProgressBar::new(progress)
            .show_percentage()
            .animate(true));
        
        ui.add_space(10.0);
        ui.label(current_task);
    });
}

/// Creates a standardized window with consistent styling
pub fn standard_window(ui: &Ui, title: &str) -> Window {
    Window::new(title)
        .collapsible(false)
        .resizable(false)
        .title_bar(true)
        .frame(Frame {
            fill: Color32::from_rgb(25, 25, 25),
            rounding: 5.0.into(),
            ..Default::default()
        })
}

// World management functions
pub fn save_world(world: &WorldMeta) -> std::io::Result<()> {
    let world_dir = get_worlds_dir().join(&world.name);
    std::fs::create_dir_all(&world_dir)?;
    
    let meta_path = world_dir.join("world.meta");
    let meta_json = serde_json::to_string_pretty(world)?;
    std::fs::write(meta_path, meta_json)?;
    
    Ok(())
}

pub fn load_saved_worlds() -> Vec<WorldMeta> {
    let mut worlds = Vec::new();
    if let Ok(worlds_dir) = std::fs::read_dir(get_worlds_dir()) {
        for entry in worlds_dir.flatten() {
            if let Ok(meta_path) = entry.path().join("world.meta").canonicalize() {
                if let Ok(meta_json) = std::fs::read_to_string(meta_path) {
                    if let Ok(world) = serde_json::from_str::<WorldMeta>(&meta_json) {
                        worlds.push(world);
                    }
                }
            }
        }
    }
    worlds
}

fn get_worlds_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("Bloksel");
    dir.push("worlds");
    dir
}

pub fn delete_world(name: &str) {
    let path = Path::new("saves").join(name);
    let _ = std::fs::remove_dir_all(path);
}

// Vulkan helper extensions
pub mod vulkan {
    use super::*;
    use anyhow::Result;

    /// Simplified buffer creation helper
    pub fn create_buffer(
        context: &VulkanContext,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe {
            context.device.create_buffer(&buffer_info, None)?
        };

        let mem_requirements = unsafe { context.device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(
                context.find_memory_type(mem_requirements.memory_type_bits, properties)?
            );

        let buffer_memory = unsafe {
            context.device.allocate_memory(&alloc_info, None)?
        };

        unsafe {
            context.device.bind_buffer_memory(buffer, buffer_memory, 0)?;
        }

        Ok((buffer, buffer_memory))
    }

    /// Simplified image creation helper
    pub fn create_image(
        context: &VulkanContext,
        width: u32,
        height: u32,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Image, vk::DeviceMemory)> {
        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        let image = unsafe {
            context.device.create_image(&image_info, None)?
        };

        let mem_requirements = unsafe { context.device.get_image_memory_requirements(image) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(
                context.find_memory_type(mem_requirements.memory_type_bits, properties)?
            );

        let image_memory = unsafe {
            context.device.allocate_memory(&alloc_info, None)?
        };

        unsafe {
            context.device.bind_image_memory(image, image_memory, 0)?;
        }

        Ok((image, image_memory))
    }

    /// Helper for single-time command buffer operations
    pub fn execute_one_time_commands<F>(
        context: &VulkanContext,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        operation: F,
    ) -> Result<()>
    where
        F: FnOnce(vk::CommandBuffer) -> Result<()>,
    {
        let command_buffer = context.begin_single_time_commands(command_pool)?;
        operation(command_buffer)?;
        context.end_single_time_commands(command_pool, queue, command_buffer)
    }

    /// Helper for common image layout transitions
    pub fn transition_image_layout(
        context: &VulkanContext,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> Result<()> {
        execute_one_time_commands(context, command_pool, queue, |command_buffer| {
            let (src_access_mask, dst_access_mask, src_stage, dst_stage) = match (old_layout, new_layout) {
                (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                ),
                (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::AccessFlags::SHADER_READ,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                ),
                _ => return Err(anyhow::anyhow!("Unsupported layout transition")),
            };

            let barrier = vk::ImageMemoryBarrier::builder()
                .old_layout(old_layout)
                .new_layout(new_layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(src_access_mask)
                .dst_access_mask(dst_access_mask);

            unsafe {
                context.device.cmd_pipeline_barrier(
                    command_buffer,
                    src_stage,
                    dst_stage,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier.build()],
                );
            }

            Ok(())
        })
    }

    /// Helper for copying buffer to image
    pub fn copy_buffer_to_image(
        context: &VulkanContext,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        buffer: vk::Buffer,
        image: vk::Image,
        width: u32,
        height: u32,
    ) -> Result<()> {
        execute_one_time_commands(context, command_pool, queue, |command_buffer| {
            let region = vk::BufferImageCopy::builder()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D { width, height, depth: 1 });

            unsafe {
                context.device.cmd_copy_buffer_to_image(
                    command_buffer,
                    buffer,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[region.build()],
                );
            }

            Ok(())
        })
    }
}

