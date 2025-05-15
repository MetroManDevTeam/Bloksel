use crate::render::vulkan::VulkanContext;
use anyhow::{Context, Result};
use ash::{vk, Device};
use egui::{ClippedPrimitive, Context as EguiContext, TexturesDelta};
use egui_winit::State as EguiWinitState;
use log::{debug, error, info, warn};
use std::sync::Arc;

pub struct EguiRenderer {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    font_texture: Option<FontTexture>,
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
}

struct FontTexture {
    image: vk::Image,
    image_view: vk::ImageView,
    memory: vk::DeviceMemory,
    sampler: vk::Sampler,
}

struct Buffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
}

impl EguiRenderer {
    pub fn new(vulkan_context: &Arc<VulkanContext>, render_pass: vk::RenderPass) -> Result<Self> {
        // Create descriptor set layout
        let descriptor_set_layout = unsafe {
            let binding = vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build();

            vulkan_context
                .device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&[binding]),
                    None,
                )
                .context("Failed to create descriptor set layout")?
        };

        // Create pipeline layout
        let pipeline_layout = unsafe {
            vulkan_context
                .device
                .create_pipeline_layout(
                    &vk::PipelineLayoutCreateInfo::builder().set_layouts(&[descriptor_set_layout]),
                    None,
                )
                .context("Failed to create pipeline layout")?
        };

        // Create descriptor pool
        let pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 100,
        };

        let descriptor_pool = unsafe {
            vulkan_context
                .device
                .create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::builder()
                        .max_sets(100)
                        .pool_sizes(&[pool_size]),
                    None,
                )
                .context("Failed to create descriptor pool")?
        };

        // Create graphics pipeline
        let vert_shader_code = include_bytes!("shaders/egui.vert.spv");
        let frag_shader_code = include_bytes!("shaders/egui.frag.spv");

        let vert_shader = vulkan_context
            .create_shader_module(vert_shader_code)
            .context("Failed to create vertex shader module")?;
        let frag_shader = vulkan_context
            .create_shader_module(frag_shader_code)
            .context("Failed to create fragment shader module")?;

        let pipeline = Self::create_pipeline(
            vulkan_context,
            render_pass,
            pipeline_layout,
            vert_shader,
            frag_shader,
        )
        .context("Failed to create egui graphics pipeline")?;

        // Clean up shader modules
        unsafe {
            vulkan_context.device.destroy_shader_module(vert_shader, None);
            vulkan_context.device.destroy_shader_module(frag_shader, None);
        }

        Ok(Self {
            pipeline,
            pipeline_layout,
            font_texture: None,
            vertex_buffer: None,
            index_buffer: None,
            descriptor_pool,
            descriptor_set_layout,
        })
    }

    fn create_pipeline(
        vulkan_context: &Arc<VulkanContext>,
        render_pass: vk::RenderPass,
        pipeline_layout: vk::PipelineLayout,
        vert_shader: vk::ShaderModule,
        frag_shader: vk::ShaderModule,
    ) -> Result<vk::Pipeline> {
        unsafe {
            // Shader stages
            let shader_stages = [
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(vert_shader)
                    .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap())
                    .build(),
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(frag_shader)
                    .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap())
                    .build(),
            ];

            // Vertex input state
            let vertex_binding = vk::VertexInputBindingDescription {
                binding: 0,
                stride: std::mem::size_of::<egui::epaint::Vertex>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            };

            let vertex_attributes = [
                vk::VertexInputAttributeDescription {
                    location: 0,
                    binding: 0,
                    format: vk::Format::R32G32_SFLOAT,
                    offset: 0,
                },
                vk::VertexInputAttributeDescription {
                    location: 1,
                    binding: 0,
                    format: vk::Format::R32G32_SFLOAT,
                    offset: 8,
                },
                vk::VertexInputAttributeDescription {
                    location: 2,
                    binding: 0,
                    format: vk::Format::R8G8B8A8_UNORM,
                    offset: 16,
                },
            ];

            let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_binding_descriptions(&[vertex_binding])
                .vertex_attribute_descriptions(&vertex_attributes);

            // Input assembly state
            let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

            // Viewport state
            let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
                .viewport_count(1)
                .scissor_count(1);

            // Rasterization state
            let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .line_width(1.0)
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .depth_bias_enable(false);

            // Multisample state
            let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                .sample_shading_enable(false)
                .alpha_to_coverage_enable(false)
                .alpha_to_one_enable(false);

            // Color blend attachment
            let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .build();

            // Color blend state
            let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op_enable(false)
                .attachments(&[color_blend_attachment]);

            // Dynamic state
            let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
            let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&dynamic_states);

            // Create the graphics pipeline
            let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
                .stages(&shader_stages)
                .vertex_input_state(&vertex_input_state)
                .input_assembly_state(&input_assembly_state)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterization_state)
                .multisample_state(&multisample_state)
                .color_blend_state(&color_blend_state)
                .dynamic_state(&dynamic_state)
                .layout(pipeline_layout)
                .render_pass(render_pass)
                .subpass(0)
                .build();

            let pipelines = vulkan_context
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .context("Failed to create graphics pipeline")?;

            Ok(pipelines[0])
        }
    }

    fn upload_font_texture(
        &mut self,
        vulkan_context: &Arc<VulkanContext>,
        font_texture: &egui::FontImage,
    ) -> Result<()> {
        // Cleanup old texture if exists
        if let Some(texture) = self.font_texture.take() {
            unsafe {
                vulkan_context.device.destroy_image(texture.image, None);
                vulkan_context.device.destroy_image_view(texture.image_view, None);
                vulkan_context.device.free_memory(texture.memory, None);
                vulkan_context.device.destroy_sampler(texture.sampler, None);
            }
        }

        // Create new texture
        let (width, height) = (font_texture.width() as u32, font_texture.height() as u32);
        let (image, memory) = vulkan_context
            .create_image(
                width,
                height,
                vk::Format::R8G8B8A8_UNORM,
                vk::ImageTiling::OPTIMAL,
                vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .context("Failed to create font texture image")?;

        // Create staging buffer and upload texture data
        let buffer_size = (width * height * 4) as vk::DeviceSize;
        let staging_buffer = vulkan_context
            .create_buffer(
                buffer_size,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
            .context("Failed to create staging buffer for font texture")?;

        // Map memory and copy data
        unsafe {
            let data_ptr = vulkan_context
                .device
                .map_memory(
                    staging_buffer.1,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .context("Failed to map staging buffer memory")?;

            // Copy pixels to mapped memory
            std::ptr::copy_nonoverlapping(
                font_texture.pixels.as_ptr() as *const u8,
                data_ptr as *mut u8,
                (width * height * 4) as usize,
            );

            vulkan_context.device.unmap_memory(staging_buffer.1);
        }

        // Transition image layout and copy buffer to image
        vulkan_context
            .transition_image_layout(
                image,
                vk::Format::R8G8B8A8_UNORM,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            )
            .context("Failed to transition image layout to TRANSFER_DST_OPTIMAL")?;

        vulkan_context
            .copy_buffer_to_image(staging_buffer.0, image, width, height)
            .context("Failed to copy buffer to image")?;

        vulkan_context
            .transition_image_layout(
                image,
                vk::Format::R8G8B8A8_UNORM,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            )
            .context("Failed to transition image layout to SHADER_READ_ONLY_OPTIMAL")?;

        // Clean up staging buffer
        unsafe {
            vulkan_context.device.destroy_buffer(staging_buffer.0, None);
            vulkan_context.device.free_memory(staging_buffer.1, None);
        }

        // Create image view
        let image_view = vulkan_context
            .create_image_view(
                image,
                vk::Format::R8G8B8A8_UNORM,
                vk::ImageAspectFlags::COLOR,
            )
            .context("Failed to create image view for font texture")?;

        // Create sampler
        let sampler = unsafe {
            let sampler_info = vk::SamplerCreateInfo::builder()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT)
                .anisotropy_enable(false)
                .max_anisotropy(1.0)
                .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
                .unnormalized_coordinates(false)
                .compare_enable(false)
                .compare_op(vk::CompareOp::ALWAYS)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .mip_lod_bias(0.0)
                .min_lod(0.0)
                .max_lod(0.0);

            vulkan_context
                .device
                .create_sampler(&sampler_info, None)
                .context("Failed to create sampler for font texture")?
        };

        self.font_texture = Some(FontTexture {
            image,
            image_view,
            memory,
            sampler,
        });

        Ok(())
    }

    fn update_buffers(
        &mut self,
        vulkan_context: &Arc<VulkanContext>,
        vertices: &[egui::epaint::Vertex],
        indices: &[u32],
    ) -> Result<()> {
        // Update vertex buffer
        self.update_vertex_buffer(vulkan_context, vertices)?;
        
        // Update index buffer
        self.update_index_buffer(vulkan_context, indices)?;

        Ok(())
    }

    fn update_vertex_buffer(
        &mut self,
        vulkan_context: &Arc<VulkanContext>,
        vertices: &[egui::epaint::Vertex],
    ) -> Result<()> {
        let vertex_buffer_size = (vertices.len() * std::mem::size_of::<egui::epaint::Vertex>()) as vk::DeviceSize;
        
        // Check if existing buffer is large enough
        if let Some(buffer) = &self.vertex_buffer {
            let current_size = unsafe {
                vulkan_context.device.get_buffer_memory_requirements(buffer.buffer).size
            };
            
            if vertex_buffer_size > current_size {
                // Need to create a larger buffer
                unsafe {
                    vulkan_context.device.destroy_buffer(buffer.buffer, None);
                    vulkan_context.device.free_memory(buffer.memory, None);
                }
                self.vertex_buffer = None;
            }
        }

        // Create new buffer if needed
        if self.vertex_buffer.is_none() {
            let (buffer, memory) = vulkan_context
                .create_buffer(
                    vertex_buffer_size,
                    vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                )
                .context("Failed to create vertex buffer")?;
                
            self.vertex_buffer = Some(Buffer { buffer, memory });
        }

        // Upload vertex data
        if let Some(buffer) = &self.vertex_buffer {
            vulkan_context
                .copy_to_device_local_buffer(
                    buffer.buffer,
                    buffer.memory,
                    unsafe {
                        std::slice::from_raw_parts(
                            vertices.as_ptr() as *const u8,
                            vertex_buffer_size as usize,
                        )
                    },
                )
                .context("Failed to copy vertex data to buffer")?;
        }

        Ok(())
    }

    fn update_index_buffer(
        &mut self,
        vulkan_context: &Arc<VulkanContext>,
        indices: &[u32],
    ) -> Result<()> {
        let index_buffer_size = (indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize;
        
        // Check if existing buffer is large enough
        if let Some(buffer) = &self.index_buffer {
            let current_size = unsafe {
                vulkan_context.device.get_buffer_memory_requirements(buffer.buffer).size
            };
            
            if index_buffer_size > current_size {
                // Need to create a larger buffer
                unsafe {
                    vulkan_context.device.destroy_buffer(buffer.buffer, None);
                    vulkan_context.device.free_memory(buffer.memory, None);
                }
                self.index_buffer = None;
            }
        }

        // Create new buffer if needed
        if self.index_buffer.is_none() {
            let (buffer, memory) = vulkan_context
                .create_buffer(
                    index_buffer_size,
                    vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                )
                .context("Failed to create index buffer")?;
                
            self.index_buffer = Some(Buffer { buffer, memory });
        }

        // Upload index data
        if let Some(buffer) = &self.index_buffer {
            vulkan_context
                .copy_to_device_local_buffer(
                    buffer.buffer,
                    buffer.memory,
                    unsafe {
                        std::slice::from_raw_parts(
                            indices.as_ptr() as *const u8,
                            index_buffer_size as usize,
                        )
                    },
                )
                .context("Failed to copy index data to buffer")?;
        }

        Ok(())
    }

    pub fn render(
        &self,
        vulkan_context: &Arc<VulkanContext>,
        command_buffer: vk::CommandBuffer,
        primitives: &[ClippedPrimitive],
        textures: &TexturesDelta,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<()> {
        // Update font texture if needed
        for (texture_id, image_delta) in &textures.set {
            if texture_id == &egui::TextureId::default() {
                if let Some(font_texture) = &image_delta.image {
                    self.upload_font_texture(vulkan_context, font_texture)?;
                }
            }
        }

        // Process vertices and indices
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        for clipped_primitive in primitives {
            let vertex_offset = vertices.len();
            vertices.extend_from_slice(&clipped_primitive.primitive.vertices);
            
            // Adjust indices for the current vertex offset
            indices.extend(
                clipped_primitive.primitive.indices
                    .iter()
                    .map(|&idx| idx + vertex_offset as u32)
            );
        }
        
        self.update_buffers(vulkan_context, &vertices, &indices)?;

        // Set viewport
        unsafe {
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: viewport_width as f32,
                height: viewport_height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            
            vulkan_context.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        }

        // Bind pipeline
        unsafe {
            vulkan_context.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
        }

        // Bind vertex and index buffers
        if let Some(buffer) = &self.vertex_buffer {
            unsafe {
                vulkan_context.device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[buffer.buffer],
                    &[0],
                );
            }
        }

        if let Some(buffer) = &self.index_buffer {
            unsafe {
                vulkan_context.device.cmd_bind_index_buffer(
                    command_buffer,
                    buffer.buffer,
                    0,
                    vk::IndexType::UINT32,
                );
            }
        }

        // Draw primitives
        let mut index_offset = 0;
        for clipped_primitive in primitives {
            let clip_rect = clipped_primitive.clip_rect;
            
            // Ensure scissor coordinates are within framebuffer bounds
            let min_x = clip_rect.min.x.max(0.0) as i32;
            let min_y = clip_rect.min.y.max(0.0) as i32;
            let max_x = clip_rect.max.x.min(viewport_width as f32) as i32;
            let max_y = clip_rect.max.y.min(viewport_height as f32) as i32;
            
            // Skip drawing if scissor rectangle is invalid
            if min_x >= max_x || min_y >= max_y {
                index_offset += clipped_primitive.primitive.indices.len() as u32;
                continue;
            }
            
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: min_x, y: min_y },
                extent: vk::Extent2D {
                    width: (max_x - min_x) as u32,
                    height: (max_y - min_y) as u32,
                },
            };

            unsafe {
                vulkan_context.device.cmd_set_scissor(command_buffer, 0, &[scissor]);
            }

            // Bind texture
            if let Some(texture) = &self.font_texture {
                let descriptor_set = self.create_descriptor_set(
                    vulkan_context, 
                    texture.image_view, 
                    texture.sampler
                )?;

                unsafe {
                    vulkan_context.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout,
                        0,
                        &[descriptor_set],
                        &[],
                    );
                }
            }

            // Draw
            unsafe {
                vulkan_context.device.cmd_draw_indexed(
                    command_buffer,
                    clipped_primitive.primitive.indices.len() as u32,
                    1,
                    index_offset,
                    0,
                    0,
                );
            }

            index_offset += clipped_primitive.primitive.indices.len() as u32;
        }

        Ok(())
    }

    fn create_descriptor_set(
        &self,
        vulkan_context: &Arc<VulkanContext>,
        image_view: vk::ImageView,
        sampler: vk::Sampler,
    ) -> Result<vk::DescriptorSet> {
        unsafe {
            // Allocate descriptor set
            let descriptor_set_layouts = [self.descriptor_set_layout];
            let allocate_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(self.descriptor_pool)
                .set_layouts(&descriptor_set_layouts);

            let descriptor_sets = vulkan_context
                .device
                .allocate_descriptor_sets(&allocate_info)
                .context("Failed to allocate descriptor set")?;
            
            let descriptor_set = descriptor_sets[0];

            // Update descriptor set
            let image_info = [vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(image_view)
                .sampler(sampler)
                .build()];

            let write_descriptor_sets = [vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&image_info)
                .build()];

            vulkan_context.device.update_descriptor_sets(&write_descriptor_sets, &[]);

            Ok(descriptor_set)
        }
    }

    pub fn cleanup(&mut self, vulkan_context: &Arc<VulkanContext>) {
        unsafe {
            // Clean up font texture
            if let Some(texture) = self.font_texture.take() {
                vulkan_context.device.destroy_image(texture.image, None);
                vulkan_context.device.destroy_image_view(texture.image_view, None);
                vulkan_context.device.free_memory(texture.memory, None);
                vulkan_context.device.destroy_sampler(texture.sampler, None);
            }

            // Clean up vertex buffer
            if let Some(buffer) = self.vertex_buffer.take() {
                vulkan_context.device.destroy_buffer(buffer.buffer, None);
                vulkan_context.device.free_memory(buffer.memory, None);
            }

            // Clean up index buffer
            if let Some(buffer) = self.index_buffer.take() {
                vulkan_context.device.destroy_buffer(buffer.buffer, None);
                vulkan_context.device.free_memory(buffer.memory, None);
            }

            // Clean up Vulkan resources
            vulkan_context.device.destroy_descriptor_pool(self.descriptor_pool, None);
            vulkan_context.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            vulkan_context.device.destroy_pipeline(self.pipeline, None);
            vulkan_context.device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

impl Drop for EguiRenderer {
    fn drop(&mut self) {
        // Important: This is a safety measure but should not be relied upon
        // The owner should explicitly call cleanup() before dropping
        warn!("EguiRenderer dropped without explicit cleanup. This may indicate a resource leak.");
    }
                              }
