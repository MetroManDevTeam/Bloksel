use crate::render::core::{Camera, Shader};
use crate::render::mesh::Mesh;
use crate::world::block::Block;
use crate::world::block_material::BlockMaterial;
use crate::world::blocks_data::BlockRegistry;
use crate::world::chunk::{CHUNK_SIZE, Chunk, ChunkMesh};
use anyhow::{Context, Result};
use ash::vk;
use glam::{Mat4, Vec2, Vec3, Vec4};
use image::RgbaImage;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;

const ATLAS_START_SIZE: u32 = 16;
const MAX_ATLAS_SIZE: u32 = 2048;
const TEXTURE_PADDING: u32 = 2;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("Texture atlas is full")]
    AtlasFull,
    #[error("Failed to load texture: {0}")]
    TextureLoadError(String),
    #[error("Vulkan error: {0}")]
    VulkanError(String),
    #[error("Memory allocation failed")]
    AllocationError,
    #[error("Buffer creation failed")]
    BufferError,
}

pub struct VulkanTexture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub extent: vk::Extent2D,
}

pub struct VulkanBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
}

pub struct ChunkRenderer {
    materials: HashMap<u16, BlockMaterial>,
    texture_atlas: Option<RgbaImage>,
    texture_coordinates: HashMap<u16, [Vec2; 4]>,
    current_atlas_pos: (u32, u32),
    max_row_height: u32,
    pub debug_mode: bool,
    pub lod_level: u8,
    pending_textures: HashSet<u16>,
    texture_atlas_size: u32,
    block_registry: Arc<BlockRegistry>,

    // Vulkan resources
    pub vulkan_texture: Option<VulkanTexture>,
    pub vertex_buffer: Option<VulkanBuffer>,
    pub index_buffer: Option<VulkanBuffer>,
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub command_pool: vk::CommandPool,



    // Statistics tracking
    pub draw_call_count: usize,
    pub vertex_count: usize,
    pub triangle_count: usize,

}

impl ChunkRenderer {
    pub fn new(
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
        block_registry: Arc<BlockRegistry>,
    ) -> Result<Self> {
        // Initialize Vulkan pipeline
        let (descriptor_set_layout, pipeline_layout, pipeline) =
            Self::create_pipeline(device)?;

        // Create command pool
        let command_pool = {
            let create_info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

            unsafe { device.create_command_pool(&create_info, None) }
                .map_err(|e| RenderError::VulkanError(format!("Failed to create command pool: {:?}", e)))?
        };

        // Create descriptor pool
        let descriptor_pool = {
            let pool_sizes = [
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1,
                },
            ];

            let create_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&pool_sizes);

            unsafe { device.create_descriptor_pool(&create_info, None) }
                .map_err(|e| RenderError::VulkanError(format!("Failed to create descriptor pool: {:?}", e)))?
        };

        // Allocate descriptor sets
        let descriptor_sets = {
            let layouts = [descriptor_set_layout];
            let allocate_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&layouts);

            unsafe { device.allocate_descriptor_sets(&allocate_info) }
                .map_err(|e| RenderError::VulkanError(format!("Failed to allocate descriptor sets: {:?}", e)))?
        };

        Ok(Self {
            materials: HashMap::new(),
            texture_atlas: Some(RgbaImage::new(ATLAS_START_SIZE, ATLAS_START_SIZE)),
            texture_coordinates: HashMap::new(),
            current_atlas_pos: (TEXTURE_PADDING, TEXTURE_PADDING),
            max_row_height: 0,
            debug_mode: false,
            lod_level: 0,
            pending_textures: HashSet::new(),
            texture_atlas_size: ATLAS_START_SIZE,
            block_registry,
            vulkan_texture: None,
            vertex_buffer: None,
            index_buffer: None,
            pipeline,
            pipeline_layout,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_sets,
            command_pool,
        })
    }

    fn create_pipeline(
        device: &ash::Device,
        render_pass: vk::RenderPass,
    ) -> Result<(vk::DescriptorSetLayout, vk::PipelineLayout, vk::Pipeline)> {
        // Create descriptor set layout
        let bindings = [
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build(),
        ];

        let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings);

        let descriptor_set_layout = unsafe {
            device.create_descriptor_set_layout(&create_info, None)
                .map_err(|e| RenderError::VulkanError(format!("Failed to create descriptor set layout: {:?}", e)))?
        };

        // Create pipeline layout
        let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&[descriptor_set_layout]);

        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&layout_create_info, None)
                .map_err(|e| RenderError::VulkanError(format!("Failed to create pipeline layout: {:?}", e)))?
        };

        // Create shader modules (assuming SPIR-V shaders)
        let vert_shader = Self::create_shader_module(device, include_bytes!("shaders/vert.glsl"))?;
        let frag_shader = Self::create_shader_module(device, include_bytes!("shaders/frag.glsl"))?;

        // Create graphics pipeline
        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_shader)
                .name(b"main\0".as_ptr() as *const i8)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_shader)
                .name(b"main\0".as_ptr() as *const i8)
                .build(),
        ];

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
    .vertex_binding_descriptions(&[
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32, // Define your Vertex type
            input_rate: vk::VertexInputRate::VERTEX,
        },
    ])
    .vertex_attribute_descriptions(&[
        // Positions
        vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32B32_SFLOAT,
            offset: 0,
        },
        // Normals
        vk::VertexInputAttributeDescription {
            location: 1,
            binding: 0,
            format: vk::Format::R32G32B32_SFLOAT,
            offset: 12,
        },

    ]);

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(1.0)
            .height(1.0)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(vk::Extent2D { width: 1, height: 1 });

        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&[viewport.build()])
            .scissors(&[scissor.build()]);

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(
                vk::ColorComponentFlags::R |
                vk::ColorComponentFlags::G |
                vk::ColorComponentFlags::B |
                vk::ColorComponentFlags::A,
            )
            .blend_enable(false)
            .build();

        let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&[color_blend_attachment])
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blending)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0);

        let pipeline = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info.build()],
                None,
            )
            .map_err(|(_, e)| RenderError::VulkanError(format!("Failed to create graphics pipeline: {:?}", e)))?
            .first()
            .copied()
            .ok_or(RenderError::VulkanError("No pipeline created".to_string()))?
        };

        // Cleanup shader modules
        unsafe {
            device.destroy_shader_module(vert_shader, None);
            device.destroy_shader_module(frag_shader, None);
        }

        Ok((descriptor_set_layout, pipeline_layout, pipeline))
    }

    fn create_shader_module(
        device: &ash::Device,
        code: &[u8],
    ) -> Result<vk::ShaderModule, RenderError> {
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(unsafe {
                std::slice::from_raw_parts(
                    code.as_ptr() as *const u32,
                    code.len() / std::mem::size_of::<u32>(),
                )
            });

        unsafe { device.create_shader_module(&create_info, None) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to create shader module: {:?}", e)))
    }

    pub fn load_material(
        &mut self,
        block_id: u16,
        material: BlockMaterial,
    ) -> Result<(), anyhow::Error> {
        if let Some(ref path) = material.texture_path {
            self.queue_texture_load(block_id, path)?;
        }
        self.materials.insert(block_id, material);
        Ok(())
    }

    fn queue_texture_load(&mut self, block_id: u16, path: &str) -> Result<(), anyhow::Error> {
        self.pending_textures.insert(block_id);
        Ok(())
    }

    pub fn process_texture_queue(&mut self) -> Result<(), anyhow::Error> {
        let mut new_atlas = RgbaImage::new(ATLAS_START_SIZE, ATLAS_START_SIZE);
        let mut current_pos = (TEXTURE_PADDING, TEXTURE_PADDING);
        let mut max_row_height = 0;

        for &block_id in &self.pending_textures.clone() {
            let material = self.materials.get(&block_id).unwrap();
            if let Some(path) = &material.texture_path {
                let img = image::open(path)
                    .with_context(|| format!("Failed to load texture: {}", path))?
                    .to_rgba8();

                let (width, height) = img.dimensions();

                // Check if we need to expand atlas
                if current_pos.0 + width + TEXTURE_PADDING > new_atlas.width()
                    || current_pos.1 + height + TEXTURE_PADDING > new_atlas.height()
                {
                    let new_size = (new_atlas.width() * 2).min(MAX_ATLAS_SIZE);
                    if new_size > new_atlas.width() {
                        new_atlas = RgbaImage::new(new_size, new_size);
                        current_pos = (TEXTURE_PADDING, TEXTURE_PADDING);
                        max_row_height = 0;
                    } else {
                        return Err(RenderError::AtlasFull.into());
                    }
                }

                // Copy texture to atlas
                for y in 0..height {
                    for x in 0..width {
                        let pixel = img.get_pixel(x, y);
                        new_atlas.put_pixel(current_pos.0 + x, current_pos.1 + y, *pixel);
                    }
                }

                // Store texture coordinates
                let u_min = current_pos.0 as f32 / new_atlas.width() as f32;
                let v_min = current_pos.1 as f32 / new_atlas.height() as f32;
                let u_max = (current_pos.0 + width) as f32 / new_atlas.width() as f32;
                let v_max = (current_pos.1 + height) as f32 / new_atlas.height() as f32;

                self.texture_coordinates.insert(
                    block_id,
                    [
                        Vec2::new(u_min, v_min),
                        Vec2::new(u_max, v_min),
                        Vec2::new(u_max, v_max),
                        Vec2::new(u_min, v_max),
                    ],
                );

                current_pos.0 += width + TEXTURE_PADDING;
                max_row_height = max_row_height.max(height);

                if current_pos.0 + TEXTURE_PADDING > new_atlas.width() {
                    current_pos.0 = TEXTURE_PADDING;
                    current_pos.1 += max_row_height + TEXTURE_PADDING;
                    max_row_height = 0;
                }
            }
        }

        self.texture_atlas = Some(new_atlas);
        self.pending_textures.clear();
        Ok(())
    }

    pub fn upload_textures(
        &mut self,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
    ) -> Result<(), RenderError> {
        let atlas = self.texture_atlas.as_ref().ok_or(RenderError::VulkanError("No texture atlas to upload".to_string()))?;
        let (width, height) = atlas.dimensions();

        // Create staging buffer
        let buffer_size = (width * height * 4) as vk::DeviceSize;
        let (staging_buffer, staging_buffer_memory) = Self::create_buffer(
            device,
            physical_device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // Copy texture data to staging buffer
        unsafe {
            let data_ptr = device.map_memory(
                staging_buffer_memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            ).map_err(|e| RenderError::VulkanError(format!("Failed to map memory: {:?}", e)))?;

            std::ptr::copy_nonoverlapping(
                atlas.as_ptr(),
                data_ptr as *mut u8,
                buffer_size as usize,
            );

            device.unmap_memory(staging_buffer_memory);
        }

        // Create Vulkan image
        let (texture_image, texture_image_memory) = Self::create_image(
            device,
            physical_device,
            width,
            height,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        // Transition image layout and copy buffer to image
        Self::transition_image_layout(
            device,
            command_pool,
            queue,
            texture_image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;

        Self::copy_buffer_to_image(
            device,
            command_pool,
            queue,
            staging_buffer,
            texture_image,
            width,
            height,
        )?;

        Self::transition_image_layout(
            device,
            command_pool,
            queue,
            texture_image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?;

        // Create image view
        let texture_image_view = Self::create_image_view(
            device,
            texture_image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageAspectFlags::COLOR,
        )?;

        // Create texture sampler
        let texture_sampler = Self::create_texture_sampler(device)?;

        // Cleanup staging buffer
        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_buffer_memory, None);
        }

        self.vulkan_texture = Some(VulkanTexture {
            image: texture_image,
            memory: texture_image_memory,
            view: texture_image_view,
            sampler: texture_sampler,
            extent: vk::Extent2D { width, height },
        });

        Ok(())
    }

    pub fn upload_chunk_data(
        &mut self,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        mesh: &mut ChunkMesh,
    ) -> Result<(), RenderError> {
        // Create vertex buffer
        let vertex_buffer_size = (mesh.vertices.len() * std::mem::size_of::<f32>()) as vk::DeviceSize;
        let (vertex_buffer, vertex_buffer_memory) = Self::create_buffer(
            device,
            physical_device,
            vertex_buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // Copy vertex data
        unsafe {
            let data_ptr = device.map_memory(
                vertex_buffer_memory,
                0,
                vertex_buffer_size,
                vk::MemoryMapFlags::empty(),
            ).map_err(|e| RenderError::VulkanError(format!("Failed to map memory: {:?}", e)))?;

            std::ptr::copy_nonoverlapping(
                mesh.vertices.as_ptr(),
                data_ptr as *mut f32,
                mesh.vertices.len(),
            );

            device.unmap_memory(vertex_buffer_memory);
        }

        // Create index buffer
        let index_buffer_size = (mesh.indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize;
        let (index_buffer, index_buffer_memory) = Self::create_buffer(
            device,
            physical_device,
            index_buffer_size,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // Copy index data
        unsafe {
            let data_ptr = device.map_memory(
                index_buffer_memory,
                0,
                index_buffer_size,
                vk::MemoryMapFlags::empty(),
            ).map_err(|e| RenderError::VulkanError(format!("Failed to map memory: {:?}", e)))?;

            std::ptr::copy_nonoverlapping(
                mesh.indices.as_ptr(),
                data_ptr as *mut u32,
                mesh.indices.len(),
            );

            device.unmap_memory(index_buffer_memory);
        }

        self.vertex_buffer = Some(VulkanBuffer {
            buffer: vertex_buffer,
            memory: vertex_buffer_memory,
            size: vertex_buffer_size,
        });

        self.index_buffer = Some(VulkanBuffer {
            buffer: index_buffer,
            memory: index_buffer_memory,
            size: index_buffer_size,
        });

        Ok(())
    }

    pub fn render_chunk(
        &mut self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        chunk: &Chunk,
        camera: &Camera,
    ) {
        if let Some(mesh) = &chunk.mesh {
            unsafe {
                // Bind pipeline
                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline,
                );

                // Bind vertex buffer
                if let Some(vertex_buffer) = &self.vertex_buffer {
                    device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[vertex_buffer.buffer],
                        &[0],
                    );
                }

                // Bind index buffer
                if let Some(index_buffer) = &self.index_buffer {
                    device.cmd_bind_index_buffer(
                        command_buffer,
                        index_buffer.buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                }

                // Update uniform buffer
                let model = chunk.transform();
                let view = camera.view_matrix();
                let projection = camera.projection_matrix();

                let ubo = UniformBufferObject {

                    model: chunk.transform(),

                    view: camera.view_matrix(),

                    projection: camera.projection_matrix(),

                };


                unsafe {

                    let data_ptr = device.map_memory(...);

                    std::ptr::copy_nonoverlapping(&ubo, data_ptr as *mut _, 1);

                    device.unmap_memory(...);

                }


                // Bind descriptor sets
                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    0,
                    &self.descriptor_sets,
                    &[],
                );

                // Draw
                device.cmd_draw_indexed(
                    command_buffer,
                    mesh.indices.len() as u32,
                    1,
                    0,
                    0,
                    0,
                );




            // Update statistics
            self.draw_call_count += 1;
            self.vertex_count += mesh.vertex_count;
            self.triangle_count += mesh.index_count / 3; // Assuming triangle list (3 indices per triangle)

            }
        }
    }

    // Helper functions for Vulkan resource creation
    fn create_buffer(
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), RenderError> {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.create_buffer(&buffer_info, None) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to create buffer: {:?}", e)))?;

        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(Self::find_memory_type(
                physical_device,
                mem_requirements.memory_type_bits,
                properties,
            )?);

        let buffer_memory = unsafe { device.allocate_memory(&alloc_info, None) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to allocate buffer memory: {:?}", e)))?;

        unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to bind buffer memory: {:?}", e)))?;

        Ok((buffer, buffer_memory))
    }

    fn create_image(
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        width: u32,
        height: u32,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Image, vk::DeviceMemory), RenderError> {
        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        let image = unsafe { device.create_image(&image_info, None) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to create image: {:?}", e)))?;

        let mem_requirements = unsafe { device.get_image_memory_requirements(image) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(Self::find_memory_type(
                physical_device,
                mem_requirements.memory_type_bits,
                properties,
            )?);

        let image_memory = unsafe { device.allocate_memory(&alloc_info, None) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to allocate image memory: {:?}", e)))?;

        unsafe { device.bind_image_memory(image, image_memory, 0) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to bind image memory: {:?}", e)))?;

        Ok((image, image_memory))
    }

    fn create_image_view(
        device: &ash::Device,
        image: vk::Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
    ) -> Result<vk::ImageView, RenderError> {
        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: aspect_flags,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        let create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(subresource_range);

        unsafe { device.create_image_view(&create_info, None) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to create image view: {:?}", e)))
    }

    fn create_texture_sampler(device: &ash::Device) -> Result<vk::Sampler, RenderError> {
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

        unsafe { device.create_sampler(&sampler_info, None) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to create texture sampler: {:?}", e)))
    }

    fn find_memory_type(
        physical_device: vk::PhysicalDevice,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32, RenderError> {
        let mem_properties = unsafe { ash::vk::get_physical_device_memory_properties(physical_device) };

        for (i, memory_type) in mem_properties.memory_types.iter().enumerate() {
            if (type_filter & (1 << i)) != 0 && (memory_type.property_flags & properties) == properties {
                return Ok(i as u32);
            }
        }

        Err(RenderError::AllocationError)
    }

    fn transition_image_layout(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> Result<(), RenderError> {
        let command_buffer = Self::begin_single_time_commands(device, command_pool)?;

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
            _ => return Err(RenderError::VulkanError("Unsupported layout transition".to_string())),
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
            device.cmd_pipeline_barrier(
                command_buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier.build()],
            );
        }

        Self::end_single_time_commands(device, command_pool, queue, command_buffer)
    }

    fn copy_buffer_to_image(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        buffer: vk::Buffer,
        image: vk::Image,
        width: u32,
        height: u32,
    ) -> Result<(), RenderError> {
        let command_buffer = Self::begin_single_time_commands(device, command_pool)?;

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
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });

        unsafe {
            device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region.build()],
            );
        }

        Self::end_single_time_commands(device, command_pool, queue, command_buffer)
    }

    fn begin_single_time_commands(
        device: &ash::Device,
        command_pool: vk::CommandPool,
    ) -> Result<vk::CommandBuffer, RenderError> {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(1);

        let command_buffer = unsafe { device.allocate_command_buffers(&alloc_info) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to allocate command buffer: {:?}", e)))?
            .first()
            .copied()
            .ok_or(RenderError::VulkanError("No command buffer allocated".to_string()))?;

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe { device.begin_command_buffer(command_buffer, &begin_info) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to begin command buffer: {:?}", e)))?;

        Ok(command_buffer)
    }

    fn end_single_time_commands(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), RenderError> {
        unsafe { device.end_command_buffer(command_buffer) }
            .map_err(|e| RenderError::VulkanError(format!("Failed to end command buffer: {:?}", e)))?;

        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[command_buffer]);

        unsafe {
            device.queue_submit(queue, &[submit_info.build()], vk::Fence::null())
                .map_err(|e| RenderError::VulkanError(format!("Failed to submit command buffer: {:?}", e)))?;
            device.queue_wait_idle(queue)
                .map_err(|e| RenderError::VulkanError(format!("Failed to wait for queue idle: {:?}", e)))?;
            device.free_command_buffers(command_pool, &[command_buffer]);
        }

        Ok(())
    }

    pub fn cleanup(&self, device: &ash::Device) {
        unsafe {
            // Cleanup buffers
            if let Some(vertex_buffer) = &self.vertex_buffer {
                device.destroy_buffer(vertex_buffer.buffer, None);
                device.free_memory(vertex_buffer.memory, None);
            }

            if let Some(index_buffer) = &self.index_buffer {
                device.destroy_buffer(index_buffer.buffer, None);
                device.free_memory(index_buffer.memory, None);
            }

            // Cleanup texture
            if let Some(texture) = &self.vulkan_texture {
                device.destroy_image(texture.image, None);
                device.free_memory(texture.memory, None);
                device.destroy_image_view(texture.view, None);
                device.destroy_sampler(texture.sampler, None);
            }

            // Cleanup pipeline resources
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_command_pool(self.command_pool, None);
        }


    /// Resets the render statistics at the start of each frame
    pub fn begin_frame(&mut self) {
        self.draw_call_count = 0;
        self.vertex_count = 0;
        self.triangle_count = 0;
    }

    /// Returns the number of draw calls made in the last frame
    pub fn get_draw_call_count(&self) -> usize {
        self.draw_call_count
    }

    /// Returns the number of vertices rendered in the last frame
    pub fn get_vertex_count(&self) -> usize {
        self.vertex_count
    }

    /// Returns the number of triangles rendered in the last frame
    pub fn get_triangle_count(&self) -> usize {
        self.triangle_count
    }
}

pub struct RenderPipeline {
    pub camera: Camera,
    pub meshes: Vec<Arc<Mesh>>,
}

impl RenderPipeline {
    pub fn new(camera: Camera) -> Self {
        Self {
            camera,
            meshes: Vec::new(),
        }
    }

    pub fn add_mesh(&mut self, mesh: Arc<Mesh>) {
        self.meshes.push(mesh);
    }
}
