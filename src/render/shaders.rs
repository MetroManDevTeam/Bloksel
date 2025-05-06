// shader.rs - Complete Shader Management System

use crate::world::block_id::BlockId;
use crate::world::block_material::{BlockMaterial, MaterialModifiers};
use crate::world::block_tech::BlockFlags;
use gl::types::*;
use glam::{Mat4, Vec3};
use std::ffi::{CString, NulError};
use std::fs;
use std::ptr;
use std::str;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShaderError {
    #[error("Shader compilation failed: {0}")]
    Compilation(String),
    #[error("Program linking failed: {0}")]
    Linking(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Null byte error: {0}")]
    Nul(#[from] NulError),
    #[error("Uniform not found: {0}")]
    UniformNotFound(String),
}

/// Main shader program structure
pub struct ShaderProgram {
    id: GLuint,
    uniforms: std::collections::HashMap<String, GLint>,
    pub variant_support: bool,
    pub connection_support: bool,
}

/// Predefined voxel shader sources
pub mod voxel_shaders {
    /// Vertex shader source with variant support
    pub const VERTEX_SRC: &str = r#"
    #version 330 core
    layout (location = 0) in vec3 aPos;
    layout (location = 1) in vec3 aNormal;
    layout (location = 2) in vec2 aTexCoord;
    layout (location = 3) in uint aBlockId;
    layout (location = 4) in uint aVariantData;

    out vec3 FragPos;
    out vec3 Normal;
    out vec2 TexCoord;
    flat out uint BlockId;
    flat out uint VariantData;

    uniform mat4 model;
    uniform mat4 view;
    uniform mat4 projection;

    void main() {
        FragPos = vec3(model * vec4(aPos, 1.0));
        Normal = mat3(transpose(inverse(model))) * aNormal;
        TexCoord = aTexCoord;
        BlockId = aBlockId;
        VariantData = aVariantData;
        gl_Position = projection * view * vec4(FragPos, 1.0);
    }
    "#;

    /// Fragment shader with PBR and connected texture support
    pub const FRAGMENT_SRC: &str = r#"
    #version 330 core
    out vec4 FragColor;

    in vec3 FragPos;
    in vec3 Normal;
    in vec2 TexCoord;
    flat in uint BlockId;
    flat in uint VariantData;

    struct Material {
        vec3 albedo;
        float roughness;
        float metallic;
        int hasVariants;
        vec3 variantAlbedoMod;
        float roughnessMod;
        float metallicMod;
    };

    uniform sampler2DArray textureAtlas;
    uniform Material material;
    uniform int connectedDirections;
    uniform vec3 viewPos;
    uniform vec3 lightPos;
    uniform float time;

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
        uint variantId = (VariantData >> 16) & 0xFFFFu;
        uint facingBits = VariantData & 0xFFFFu;
        
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
        vec2 adjustedUV = get_connected_uv(uint(connectedDirections), TexCoord);
        
        // Sample texture array using combined ID
        uint textureIndex = BlockId * 16u + variantId;
        vec4 texColor = texture(textureAtlas, vec3(adjustedUV, float(textureIndex)));
        
        // PBR lighting calculations
        vec3 N = normalize(Normal);
        vec3 V = normalize(viewPos - FragPos);
        vec3 F0 = mix(vec3(0.04), finalAlbedo, finalMetallic);

        // Direct lighting
        vec3 L = normalize(lightPos - FragPos);
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

        FragColor = vec4(color * texColor.rgb, texColor.a);
    }
    "#;

    /// Geometry shader for advanced effects
    pub const GEOMETRY_SRC: &str = r#"
    #version 330 core
    layout (triangles) in;
    layout (triangle_strip, max_vertices = 3) out;

    in vec3 FragPos[];
    in vec3 Normal[];
    in vec2 TexCoord[];
    flat in uint BlockId[];
    flat in uint VariantData[];

    out vec3 gFragPos;
    out vec3 gNormal;
    out vec2 gTexCoord;
    flat out uint gBlockId;
    flat out uint gVariantData;

    void main() {
        for(int i = 0; i < 3; i++) {
            gFragPos = FragPos[i];
            gNormal = Normal[i];
            gTexCoord = TexCoord[i];
            gBlockId = BlockId[i];
            gVariantData = VariantData[i];
            gl_Position = gl_in[i].gl_Position;
            EmitVertex();
        }
        EndPrimitive();
    }
    "#;
}

/// Extension methods for geometry shader support
impl ShaderProgram {
    /// Creates program with geometry shader
    pub fn with_geometry(
        vertex_path: &str,
        geometry_path: &str,
        fragment_path: &str,
    ) -> Result<Self, ShaderError> {
        let vertex_shader = Self::compile_shader(vertex_path, gl::VERTEX_SHADER)?;
        let geometry_shader = Self::compile_shader(geometry_path, gl::GEOMETRY_SHADER)?;
        let fragment_shader = Self::compile_shader(fragment_path, gl::FRAGMENT_SHADER)?;

        let program = unsafe { gl::CreateProgram() };
        unsafe {
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, geometry_shader);
            gl::AttachShader(program, fragment_shader);
            Self::link_program(program)?;
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(geometry_shader);
            gl::DeleteShader(fragment_shader);
        }

        let mut program = ShaderProgram {
            id: program,
            uniforms: std::collections::HashMap::new(),
            variant_support: false,
            connection_support: false,
        };

        program.detect_features();
        Ok(program)
    }

    pub fn new(vertex_path: &str, fragment_path: &str) -> Result<Self, ShaderError> {
        let vertex_shader = Self::compile_shader(vertex_path, gl::VERTEX_SHADER)?;
        let fragment_shader = Self::compile_shader(fragment_path, gl::FRAGMENT_SHADER)?;

        let program = unsafe { gl::CreateProgram() };
        unsafe {
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            Self::link_program(program)?;
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
        }

        let mut program = ShaderProgram {
            id: program,
            uniforms: std::collections::HashMap::new(),
            variant_support: false,
            connection_support: false,
        };

        program.detect_features();
        Ok(program)
    }

    fn compile_shader(path: &str, shader_type: GLenum) -> Result<GLuint, ShaderError> {
        let source = fs::read_to_string(path)?;
        let source = CString::new(source)?;
        let shader = unsafe { gl::CreateShader(shader_type) };
        unsafe {
            gl::ShaderSource(shader, 1, &source.as_ptr(), std::ptr::null());
            gl::CompileShader(shader);
        }

        let mut success = 0;
        unsafe { gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success) };
        if success == 0 {
            let mut len = 0;
            unsafe { gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len) };
            let mut buffer = vec![0u8; len as usize];
            unsafe {
                gl::GetShaderInfoLog(
                    shader,
                    len,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut i8,
                )
            };
            let error = String::from_utf8_lossy(&buffer).to_string();
            unsafe { gl::DeleteShader(shader) };
            return Err(ShaderError::Compilation(error));
        }

        Ok(shader)
    }

    fn link_program(program: GLuint) -> Result<(), ShaderError> {
        unsafe { gl::LinkProgram(program) };
        let mut success = 0;
        unsafe { gl::GetProgramiv(program, gl::LINK_STATUS, &mut success) };
        if success == 0 {
            let mut len = 0;
            unsafe { gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len) };
            let mut buffer = vec![0u8; len as usize];
            unsafe {
                gl::GetProgramInfoLog(
                    program,
                    len,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut i8,
                )
            };
            let error = String::from_utf8_lossy(&buffer).to_string();
            return Err(ShaderError::Linking(error));
        }
        Ok(())
    }

    pub fn use_program(&self) {
        unsafe { gl::UseProgram(self.id) };
    }

    pub fn set_uniform(&self, name: &str, value: &impl UniformValue) {
        let location = self.get_uniform_location(name);
        value.set_uniform(location);
    }

    fn get_uniform_location(&self, name: &str) -> GLint {
        *self.uniforms.entry(name.to_string()).or_insert_with(|| {
            let name = CString::new(name).unwrap();
            unsafe { gl::GetUniformLocation(self.id, name.as_ptr()) }
        })
    }

    fn detect_features(&mut self) {
        self.variant_support = self.get_uniform_location("material.hasVariants") != -1;
        self.connection_support = self.get_uniform_location("connectedDirections") != -1;
    }
}

pub trait UniformValue {
    fn set_uniform(&self, location: GLint);
}

impl UniformValue for Mat4 {
    fn set_uniform(&self, location: GLint) {
        unsafe { gl::UniformMatrix4fv(location, 1, gl::FALSE, self.as_ref().as_ptr()) };
    }
}

impl UniformValue for Vec3 {
    fn set_uniform(&self, location: GLint) {
        unsafe { gl::Uniform3f(location, self.x, self.y, self.z) };
    }
}

impl UniformValue for f32 {
    fn set_uniform(&self, location: GLint) {
        unsafe { gl::Uniform1f(location, *self) };
    }
}

impl UniformValue for i32 {
    fn set_uniform(&self, location: GLint) {
        unsafe { gl::Uniform1i(location, *self) };
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id) };
    }
}

pub struct Shader {
    pub vertex_source: String,
    pub fragment_source: String,
    pub geometry_source: Option<String>,
}

impl Shader {
    pub fn new(vertex_source: String, fragment_source: String) -> Self {
        Self {
            vertex_source,
            fragment_source,
            geometry_source: None,
        }
    }

    pub fn with_geometry(
        vertex_source: String,
        fragment_source: String,
        geometry_source: String,
    ) -> Self {
        Self {
            vertex_source,
            fragment_source,
            geometry_source: Some(geometry_source),
        }
    }
}
