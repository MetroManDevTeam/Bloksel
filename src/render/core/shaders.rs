// shaders.rs - Vulkan Shader Management System

use ash::vk;
use glam::{Mat4, Vec3, Vec4};
use std::collections::HashMap;
use std::ffi::{CString, NulError};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use anyhow::Result;

#[derive(Debug, Error)]
pub enum ShaderError {
    #[error("Shader compilation failed: {0}")]
    Compilation(String),
    #[error("Shader module creation failed: {0}")]
    ModuleCreation(String),
    #[error("Pipeline creation failed: {0}")]
    PipelineCreation(String),
    #[error("Descriptor set layout creation failed: {0}")]
    DescriptorLayoutCreation(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Null byte error: {0}")]
    Nul(#[from] NulError),
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

pub struct ShaderProgram {
    device: Arc<ash::Device>,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    uniform_buffers: Vec<vk::Buffer>,
    uniform_memories: Vec<vk::DeviceMemory>,
    uniform_mapped: Vec<*mut std::ffi::c_void>,
    pub variant_support: bool,
    pub connection_support: bool,
    uniforms: Mutex<HashMap<String, UniformInfo>>,
}

#[derive(Clone)]
struct UniformInfo {
    binding: u32,
    size: usize,
    offset: usize,
    uniform_type: UniformType,
}

#[derive(Clone)]
enum UniformType {
    Matrix4,
    Vector3,
    Float,
    Int,
    Texture,
}

impl ShaderProgram {
    pub fn new(
        device: Arc<ash::Device>,
        vertex_path: &str,
        fragment_path: &str,
    ) -> Result<Self, ShaderError> {
        let vertex_code = Self::load_shader_file(vertex_path)?;
        let fragment_code = Self::load_shader_file(fragment_path)?;

        let vertex_module = Self::create_shader_module(&device, &vertex_code)?;
        let fragment_module = Self::create_shader_module(&device, &fragment_code)?;

        let descriptor_set_layout = Self::create_descriptor_set_layout(&device)?;
        let pipeline_layout = Self::create_pipeline_layout(&device, &descriptor_set_layout)?;
        let descriptor_pool = Self::create_descriptor_pool(&device)?;
        let descriptor_sets = Self::allocate_descriptor_sets(&device, &descriptor_pool, &descriptor_set_layout)?;

        let (uniform_buffers, uniform_memories, uniform_mapped) = 
            Self::create_uniform_buffers(&device, 256)?; // Example size

        let mut uniforms = HashMap::new();
        uniforms.insert("model".to_string(), UniformInfo {
            binding: 0,
            size: std::mem::size_of::<Mat4>(),
            offset: 0,
            uniform_type: UniformType::Matrix4,
        });
        // Add other uniforms...

        Ok(Self {
            device,
            pipeline: vk::Pipeline::null(), // Will be set in create_pipeline
            pipeline_layout,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_sets,
            uniform_buffers,
            uniform_memories,
            uniform_mapped,
            variant_support: true,
            connection_support: true,
            uniforms: Mutex::new(uniforms),
        })
    }

    pub fn with_geometry(
        device: Arc<ash::Device>,
        vertex_path: &str,
        geometry_path: &str,
        fragment_path: &str,
    ) -> Result<Self, ShaderError> {
        // Similar to new() but with geometry shader
        unimplemented!()
    }

    fn load_shader_file(path: &str) -> Result<Vec<u8>, ShaderError> {
        let path = Path::new(path);
        if path.extension().and_then(|s| s.to_str()) == Some("spv") {
            fs::read(path).map_err(Into::into)
        } else {
            // Compile GLSL to SPIR-V here or expect pre-compiled
            Err(ShaderError::Compilation("Shader must be pre-compiled to SPIR-V".into()))
        }
    }

    fn create_shader_module(
        device: &ash::Device,
        code: &[u8],
    ) -> Result<vk::ShaderModule, ShaderError> {
        let code = unsafe {
            std::slice::from_raw_parts(
                code.as_ptr() as *const u32,
                code.len() / std::mem::size_of::<u32>(),
            )
        };

        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(code);

        unsafe {
            device.create_shader_module(&create_info, None)
                .map_err(|e| ShaderError::ModuleCreation(e.to_string()))
        }
    }

    fn create_descriptor_set_layout(
        device: &ash::Device,
    ) -> Result<vk::DescriptorSetLayout, ShaderError> {
        let bindings = [
            // UBO for matrices
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build(),
            // UBO for material
            vk::DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build(),
            // Texture sampler
            vk::DescriptorSetLayoutBinding::builder()
                .binding(2)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build(),
        ];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings);

        unsafe {
            device.create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| ShaderError::DescriptorLayoutCreation(e.to_string()))
        }
    }

    fn create_pipeline_layout(
        device: &ash::Device,
        descriptor_set_layout: &vk::DescriptorSetLayout,
    ) -> Result<vk::PipelineLayout, ShaderError> {
        let layouts = [*descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&layouts);

        unsafe {
            device.create_pipeline_layout(&layout_info, None)
                .map_err(|e| ShaderError::PipelineCreation(e.to_string()))
        }
    }

    fn create_descriptor_pool(
        device: &ash::Device,
    ) -> Result<vk::DescriptorPool, ShaderError> {
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 2,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
            },
        ];

        let pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(1);

        unsafe {
            device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| ShaderError::Vulkan(e))
        }
    }

    fn allocate_descriptor_sets(
        device: &ash::Device,
        pool: &vk::DescriptorPool,
        layout: &vk::DescriptorSetLayout,
    ) -> Result<Vec<vk::DescriptorSet>, ShaderError> {
        let layouts = [*layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(*pool)
            .set_layouts(&layouts);

        unsafe {
            device.allocate_descriptor_sets(&alloc_info)
                .map_err(|e| ShaderError::Vulkan(e))
        }
    }

    fn create_uniform_buffers(
        device: &ash::Device,
        size: usize,
    ) -> Result<(Vec<vk::Buffer>, Vec<vk::DeviceMemory>, Vec<*mut std::ffi::c_void>), ShaderError> {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe {
            device.create_buffer(&buffer_info, None)?
        };

        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(0); // Should find proper type

        let memory = unsafe {
            device.allocate_memory(&alloc_info, None)?
        };

        unsafe {
            device.bind_buffer_memory(buffer, memory, 0)?;
        }

        let mapped = unsafe {
            device.map_memory(
                memory,
                0,
                size as vk::DeviceSize,
                vk::MemoryMapFlags::empty(),
            )?
        };

        Ok((vec![buffer], vec![memory], vec![mapped]))
    }

    pub fn create_pipeline(
        &mut self,
        render_pass: vk::RenderPass,
        vertex_bindings: &[vk::VertexInputBindingDescription],
        vertex_attributes: &[vk::VertexInputAttributeDescription],
    ) -> Result<(), ShaderError> {
        unimplemented!() // Similar to previous example but with proper error handling
    }

    pub fn use_program(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &self.descriptor_sets,
                &[],
            );
        }
    }

    pub fn set_uniform<T: UniformValue>(&self, name: &str, value: &T) {
        let uniforms = self.uniforms.lock().unwrap();
        if let Some(info) = uniforms.get(name) {
            unsafe {
                let dest = self.uniform_mapped[0] as *mut u8;
                value.write_to_memory(dest.add(info.offset));
            }
            
            // Update descriptor set if needed
            let buffer_info = vk::DescriptorBufferInfo {
                buffer: self.uniform_buffers[0],
                offset: info.offset as vk::DeviceSize,
                range: info.size as vk::DeviceSize,
            };

            let write = vk::WriteDescriptorSet::builder()
                .dst_set(self.descriptor_sets[0])
                .dst_binding(info.binding)
                .dst_array_element(0)
                .descriptor_type(match info.uniform_type {
                    UniformType::Matrix4 | UniformType::Vector3 | UniformType::Float | UniformType::Int => 
                        vk::DescriptorType::UNIFORM_BUFFER,
                    UniformType::Texture => 
                        vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                })
                .buffer_info(&[buffer_info])
                .build();

            unsafe {
                self.device.update_descriptor_sets(&[write], &[]);
            }
        }
    }

    fn detect_features(&mut self) {
        // In Vulkan we know these features exist because they're in the shader
        self.variant_support = true;
        self.connection_support = true;
    }
}

pub trait UniformValue {
    fn write_to_memory(&self, dest: *mut u8);
}

impl UniformValue for Mat4 {
    fn write_to_memory(&self, dest: *mut u8) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.as_ref().as_ptr() as *const u8,
                dest,
                std::mem::size_of::<Mat4>(),
            );
        }
    }
}

impl UniformValue for Vec3 {
    fn write_to_memory(&self, dest: *mut u8) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.as_ref().as_ptr() as *const u8,
                dest,
                std::mem::size_of::<Vec3>(),
            );
        }
    }
}

impl UniformValue for f32 {
    fn write_to_memory(&self, dest: *mut u8) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                self as *const f32 as *const u8,
                dest,
                std::mem::size_of::<f32>(),
            );
        }
    }
}

impl UniformValue for i32 {
    fn write_to_memory(&self, dest: *mut u8) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                self as *const i32 as *const u8,
                dest,
                std::mem::size_of::<i32>(),
            );
        }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe {
            for memory in &self.uniform_memories {
                if !memory.is_null() {
                    self.device.unmap_memory(*memory);
                }
            }

            for (buffer, memory) in self.uniform_buffers.iter().zip(&self.uniform_memories) {
                self.device.destroy_buffer(*buffer, None);
                self.device.free_memory(*memory, None);
            }

            if !self.pipeline.is_null() {
                self.device.destroy_pipeline(self.pipeline, None);
            }

            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}

pub mod voxel_shaders {
    // Updated vertex shader using Vulkan GLSL
    pub const VERTEX_SRC: &str = r#"
    #version 450
    layout(location = 0) in vec3 aPos;
    layout(location = 1) in vec3 aNormal;
    layout(location = 2) in vec2 aTexCoord;
    layout(location = 3) in uint aBlockId;
    layout(location = 4) in uint aVariantData;

    layout(set = 0, binding = 0) uniform UniformBufferObject {
        mat4 model;
        mat4 view;
        mat4 projection;
    } ubo;

    layout(location = 0) out vec3 fragPos;
    layout(location = 1) out vec3 normal;
    layout(location = 2) out vec2 texCoord;
    layout(location = 3) flat out uint blockId;
    layout(location = 4) flat out uint variantData;

    void main() {
        fragPos = vec3(ubo.model * vec4(aPos, 1.0));
        normal = mat3(transpose(inverse(ubo.model))) * aNormal;
        texCoord = aTexCoord;
        blockId = aBlockId;
        variantData = aVariantData;
        gl_Position = ubo.projection * ubo.view * vec4(fragPos, 1.0);
    }
    "#;

    // Updated fragment shader using Vulkan GLSL
    pub const FRAGMENT_SRC: &str = r#"
    #version 450
    layout(location = 0) out vec4 fragColor;

    layout(set = 0, binding = 1) uniform Material {
        vec3 albedo;
        float roughness;
        float metallic;
        int hasVariants;
        vec3 variantAlbedoMod;
        float roughnessMod;
        float metallicMod;
    } material;

    layout(set = 0, binding = 2) uniform sampler2DArray textureAtlas;

    layout(set = 0, binding = 3) uniform LightData {
        vec3 viewPos;
        vec3 lightPos;
        float time;
        int connectedDirections;
    } light;

    layout(location = 0) in vec3 fragPos;
    layout(location = 1) in vec3 normal;
    layout(location = 2) in vec2 texCoord;
    layout(location = 3) flat in uint blockId;
    layout(location = 4) flat in uint variantData;

    const float PI = 3.14159265359;

    vec3 fresnelSchlick(float cosTheta, vec3 F0) {
        return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
    }

    float DistributionGGX(vec3 N, vec3 H, float roughness) {
        float a = roughness * roughness;
        float a2 = a * a;
        float NdotH = max(dot(N, H), 0.0);
        float NdotH2 = NdotH * NdotH;
        return a2 / (PI * pow(NdotH2 * (a2 - 1.0) + 1.0, 2.0));
    }

    vec2 get_connected_uv(uint connections, vec2 uv) {
        ivec2 texSize = textureSize(textureAtlas, 0).xy;
        vec2 pixelUV = uv * texSize;
        
        // Horizontal connections
        if ((connections & 0x3u) != 0u) {
            if (pixelUV.x < 2.0) pixelUV.x += 2.0;
            if (pixelUV.x > texSize.x - 2.0) pixelUV.x -= 2.0;
        }
        
        // Vertical connections
        if ((connections & 0xCu) != 0u) {
            if (pixelUV.y < 2.0) pixelUV.y += 2.0;
            if (pixelUV.y > texSize.y - 2.0) pixelUV.y -= 2.0;
        }
        
        return pixelUV / texSize;
    }

    void main() {
        // Extract variant data
        uint variantId = (variantData >> 16) & 0xFFFFu;
        uint facingBits = variantData & 0xFFFFu;
        
        // Calculate final material properties
        vec3 finalAlbedo = material.albedo;
        float finalRoughness = material.roughness;
        float finalMetallic = material.metallic;
        
        if (material.hasVariants == 1) {
            finalAlbedo *= material.variantAlbedoMod;
            finalRoughness = clamp(finalRoughness + material.roughnessMod, 0.0, 1.0);
            finalMetallic = clamp(finalMetallic + material.metallicMod, 0.0, 1.0);
        }

        // Calculate connected texture coordinates
        vec2 adjustedUV = get_connected_uv(uint(light.connectedDirections), texCoord);
        
        // Sample texture array using combined ID
        uint textureIndex = blockId * 16u + variantId;
        vec4 texColor = texture(textureAtlas, vec3(adjustedUV, float(textureIndex)));
        
        // PBR lighting calculations
        vec3 N = normalize(normal);
        vec3 V = normalize(light.viewPos - fragPos);
        vec3 F0 = mix(vec3(0.04), finalAlbedo, finalMetallic);

        // Direct lighting
        vec3 L = normalize(light.lightPos - fragPos);
        vec3 H = normalize(V + L);
        float NDF = DistributionGGX(N, H, finalRoughness);
        vec3 F = fresnelSchlick(max(dot(H, V), 0.0), F0);
        vec3 kS = F;
        vec3 kD = (vec3(1.0) - kS) * (1.0 - finalMetallic);

        float NdotL = max(dot(N, L), 0.0);
        vec3 radiance = vec3(1.0) * NdotL;

        vec3 Lo = (kD * finalAlbedo / PI + NDF * F) * radiance;
        vec3 ambient = vec3(0.03) * finalAlbedo;
        vec3 color = ambient + Lo;

        fragColor = vec4(color * texColor.rgb, texColor.a);
    }
    "#;

    // Updated geometry shader using Vulkan GLSL
    pub const GEOMETRY_SRC: &str = r#"
    #version 450
    layout(triangles) in;
    layout(triangle_strip, max_vertices = 3) out;

    layout(location = 0) in vec3 fragPos[];
    layout(location = 1) in vec3 normal[];
    layout(location = 2) in vec2 texCoord[];
    layout(location = 3) flat in uint blockId[];
    layout(location = 4) flat in uint variantData[];

    layout(location = 0) out vec3 gFragPos;
    layout(location = 1) out vec3 gNormal;
    layout(location = 2) out vec2 gTexCoord;
    layout(location = 3) flat out uint gBlockId;
    layout(location = 4) flat out uint gVariantData;

    void main() {
        for(int i = 0; i < 3; i++) {
            gFragPos = fragPos[i];
            gNormal = normal[i];
            gTexCoord = texCoord[i];
            gBlockId = blockId[i];
            gVariantData = variantData[i];
            gl_Position = gl_in[i].gl_Position;
            EmitVertex();
        }
        EndPrimitive();
    }
    "#;
}

pub struct Shader {
    pub vertex_source: String,
    pub fragment_source: String,
    pub geometry_source: Option<String>,
}

impl Shader {
    pub fn new(vertex: &str, fragment: &str) -> Self {
        Self {
            vertex_source: vertex.to_string(),
            fragment_source: fragment.to_string(),
            geometry_source: None,
        }
    }
} 