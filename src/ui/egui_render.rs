use crate::render::vulkan::VulkanContext;
use ash::{version::DeviceV1_0, vk};
use egui::{ClippedPrimitive, Context as EguiContext, TexturesDelta};
use egui_winit::State as EguiWinitState;
use log::{debug, error, info, warn, LevelFilter};
 use std::sync::Arc;

pub struct EguiRenderer {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    font_texture: Option<(vk::Image, vk::ImageView, vk::DeviceMemory, vk::Sampler)>,
    vertex_buffer: Option<(vk::Buffer, vk::DeviceMemory)>,
    index_buffer: Option<(vk::Buffer, vk::DeviceMemory)>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
}

impl EguiRenderer {
    fn new(vulkan_context: &Arc<VulkanContext>, render_pass: vk::RenderPass) -> Result<Self> {
        // Create pipeline layout
        let descriptor_set_layout = unsafe {
            vulkan_context.device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder()
                    .bindings(&[
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(0)
                            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                            .descriptor_count(1)
                            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                            .build(),
                    ]),
                None,
            )?
        };

        let pipeline_layout = unsafe {
            vulkan_context.device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&[descriptor_set_layout]),
                None,
            )?
        };

        // Create descriptor pool
        let descriptor_pool = unsafe {
            vulkan_context.device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .max_sets(100)
                    .pool_sizes(&[vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                        descriptor_count: 100,
                    }]),
                None,
            )?
        };

        // Create graphics pipeline
        let vert_shader = vulkan_context.create_shader_module(include_bytes!("shaders/egui.vert.spv"))?;
        let frag_shader = vulkan_context.create_shader_module(include_bytes!("shaders/egui.frag.spv"))?;

        let pipeline = unsafe {
            let shader_stages = [
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(vert_shader)
                    .name(b"main\0")
                    .build(),
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(frag_shader)
                    .name(b"main\0")
                    .build(),
            ];

            let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_binding_descriptions(&[
                    vk::VertexInputBindingDescription {
                        binding: 0,
                        stride: std::mem::size_of::<egui::epaint::Vertex>() as u32,
                        input_rate: vk::VertexInputRate::VERTEX,
                    },
                ])
                .vertex_attribute_descriptions(&[
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
                ]);

            let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

            let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
                .viewport_count(1)
                .scissor_count(1);

            let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .line_width(1.0);

            let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);

            let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::all())
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD);

            let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
                .attachments(&[color_blend_attachment]);

            let dynamic_states = &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
            let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(dynamic_states);

            vulkan_context.device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[vk::GraphicsPipelineCreateInfo::builder()
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
                    .build()],
                None,
            )?[0]
        };

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

    fn upload_font_texture(
        &mut self,
        vulkan_context: &Arc<VulkanContext>,
        font_texture: &egui::FontImage,
    ) -> Result<()> {
        // Cleanup old texture if exists
        if let Some((image, image_view, memory, sampler)) = self.font_texture.take() {
            unsafe {
                vulkan_context.device.destroy_image(image, None);
                vulkan_context.device.destroy_image_view(image_view, None);
                vulkan_context.device.free_memory(memory, None);
                vulkan_context.device.destroy_sampler(sampler, None);
            }
        }

        // Create new texture
        let (width, height) = (font_texture.width() as u32, font_texture.height() as u32);
        let (image, memory) = vulkan_context.create_image(
            width,
            height,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        // Upload texture data
        let staging_buffer = vulkan_context.create_buffer(
            (width * height * 4) as vk::DeviceSize,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            let data_ptr = vulkan_context.device.map_memory(
                staging_buffer.1,
                0,
                (width * height * 4) as vk::DeviceSize,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(
                font_texture.pixels.as_ptr() as *const u8,
                data_ptr as *mut u8,
                (width * height * 4) as usize,
            );
            vulkan_context.device.unmap_memory(staging_buffer.1);
        }

        vulkan_context.transition_image_layout(
            image,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;
        vulkan_context.copy_buffer_to_image(
            staging_buffer.0,
            image,
            width,
            height,
        )?;
        vulkan_context.transition_image_layout(
            image,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?;

        unsafe {
            vulkan_context.device.destroy_buffer(staging_buffer.0, None);
            vulkan_context.device.free_memory(staging_buffer.1, None);
        }

        // Create image view
        let image_view = vulkan_context.create_image_view(
            image,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageAspectFlags::COLOR,
        )?;

        // Create sampler
        let sampler = unsafe {
            vulkan_context.device.create_sampler(
                &vk::SamplerCreateInfo::builder()
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
                    .max_lod(0.0),
                None,
            )?
        };

        self.font_texture = Some((image, image_view, memory, sampler));
        Ok(())
    }

    fn update_buffers(
        &mut self,
        vulkan_context: &Arc<VulkanContext>,
        vertices: &[egui::epaint::Vertex],
        indices: &[u32],
    ) -> Result<()> {
        // Update vertex buffer
        let vertex_buffer_size = (vertices.len() * std::mem::size_of::<egui::epaint::Vertex>()) as vk::DeviceSize;
        if let Some((buffer, memory)) = &self.vertex_buffer {
            if vertex_buffer_size > unsafe {
                vulkan_context.device.get_buffer_memory_requirements(*buffer).size
            } {
                unsafe {
                    vulkan_context.device.destroy_buffer(*buffer, None);
                    vulkan_context.device.free_memory(*memory, None);
                }
                self.vertex_buffer = None;
            }
        }

        if self.vertex_buffer.is_none() {
            let (buffer, memory) = vulkan_context.create_buffer(
                vertex_buffer_size,
                vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?;
            self.vertex_buffer = Some((buffer, memory));
        }

        // Update index buffer
        let index_buffer_size = (indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize;
        if let Some((buffer, memory)) = &self.index_buffer {
            if index_buffer_size > unsafe {
                vulkan_context.device.get_buffer_memory_requirements(*buffer).size
            } {
                unsafe {
                    vulkan_context.device.destroy_buffer(*buffer, None);
                    vulkan_context.device.free_memory(*memory, None);
                }
                self.index_buffer = None;
            }
        }

        if self.index_buffer.is_none() {
            let (buffer, memory) = vulkan_context.create_buffer(
                index_buffer_size,
                vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?;
            self.index_buffer = Some((buffer, memory));
        }

        // Upload data
        if let Some((buffer, memory)) = &self.vertex_buffer {
            vulkan_context.copy_to_device_local_buffer(
                *buffer,
                *memory,
                unsafe {
                    std::slice::from_raw_parts(
                        vertices.as_ptr() as *const u8,
                        vertex_buffer_size as usize,
                    )
                },
            )?;
        }

        if let Some((buffer, memory)) = &self.index_buffer {
            vulkan_context.copy_to_device_local_buffer(
                *buffer,
                *memory,
                unsafe {
                    std::slice::from_raw_parts(
                        indices.as_ptr() as *const u8,
                        index_buffer_size as usize,
                    )
                },
            )?;
        }

        Ok(())
    }

    fn render(
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
                if let Some(font_texture) = image_delta.image {
                    self.upload_font_texture(vulkan_context, font_texture)?;
                }
            }
        }

        // Update buffers
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for clipped_primitive in primitives {
            vertices.extend_from_slice(&clipped_primitive.primitive.vertices);
            indices.extend_from_slice(&clipped_primitive.primitive.indices);
        }
        self.update_buffers(vulkan_context, &vertices, &indices)?;

        // Set viewport
        unsafe {
            vulkan_context.device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: viewport_width as f32,
                    height: viewport_height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
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
        if let Some((vertex_buffer, _)) = self.vertex_buffer {
            unsafe {
                vulkan_context.device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[vertex_buffer],
                    &[0],
                );
            }
        }

        if let Some((index_buffer, _)) = self.index_buffer {
            unsafe {
                vulkan_context.device.cmd_bind_index_buffer(
                    command_buffer,
                    index_buffer,
                    0,
                    vk::IndexType::UINT32,
                );
            }
        }

        // Draw primitives
        let mut index_offset = 0;
        for clipped_primitive in primitives {
            let clip_rect = clipped_primitive.clip_rect;
            let scissor = vk::Rect2D {
                offset: vk::Offset2D {
                    x: clip_rect.min.x as i32,
                    y: clip_rect.min.y as i32,
                },
                extent: vk::Extent2D {
                    width: (clip_rect.max.x - clip_rect.min.x) as u32,
                    height: (clip_rect.max.y - clip_rect.min.y) as u32,
                },
            };

            unsafe {
                vulkan_context.device.cmd_set_scissor(command_buffer, 0, &[scissor]);
            }

            if let Some((_, image_view, _, sampler)) = &self.font_texture {
                let descriptor_set = unsafe {
                    let descriptor_set_layouts = [self.descriptor_set_layout];
                    let allocate_info = vk::DescriptorSetAllocateInfo::builder()
                        .descriptor_pool(self.descriptor_pool)
                        .set_layouts(&descriptor_set_layouts);

                    let descriptor_sets = vulkan_context.device.allocate_descriptor_sets(&allocate_info)?;
                    descriptor_sets[0]
                };

                unsafe {
                    let image_info = [vk::DescriptorImageInfo::builder()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(*image_view)
                        .sampler(*sampler)
                        .build()];

                    let write_descriptor_sets = [vk::WriteDescriptorSet::builder()
                        .dst_set(descriptor_set)
                        .dst_binding(0)
                        .dst_array_element(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&image_info)
                        .build()];

                    vulkan_context.device.update_descriptor_sets(&write_descriptor_sets, &[]);
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

    fn cleanup(&mut self, vulkan_context: &Arc<VulkanContext>) {
        unsafe {
            if let Some((image, image_view, memory, sampler)) = self.font_texture.take() {
                vulkan_context.device.destroy_image(image, None);
                vulkan_context.device.destroy_image_view(image_view, None);
                vulkan_context.device.free_memory(memory, None);
                vulkan_context.device.destroy_sampler(sampler, None);
            }

            if let Some((buffer, memory)) = self.vertex_buffer.take() {
                vulkan_context.device.destroy_buffer(buffer, None);
                vulkan_context.device.free_memory(memory, None);
            }

            if let Some((buffer, memory)) = self.index_buffer.take() {
                vulkan_context.device.destroy_buffer(buffer, None);
                vulkan_context.device.free_memory(memory, None);
            }

            vulkan_context.device.destroy_descriptor_pool(self.descriptor_pool, None);
            vulkan_context.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            vulkan_context.device.destroy_pipeline(self.pipeline, None);
            vulkan_context.device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
