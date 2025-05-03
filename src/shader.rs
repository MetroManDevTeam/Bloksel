// shader.rs - Complete Shader Management System

use gl::types::*;
use std::ffi::{CString, NulError};
use std::fs;
use std::ptr;
use std::str;
use thiserror::Error;
use crate::block::{BlockId, BlockFlags, MaterialModifiers};
use crate::chunk_renderer::BlockMaterial;

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

impl ShaderProgram {
    /// Creates a new shader program from vertex and fragment shader files
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

    /// Compiles a shader from source file
    fn compile_shader(path: &str, shader_type: GLenum) -> Result<GLuint, ShaderError> {
        let source = fs::read_to_string(path)?;
        let c_source = CString::new(source.as_bytes())?;
        
        let shader = unsafe { gl::CreateShader(shader_type) };
        unsafe {
            gl::ShaderSource(shader, 1, &c_source.as_ptr(), ptr::null());
            gl::CompileShader(shader);
        }

        let mut success = 1;
        unsafe { gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success) };
        if success == 0 {
            let mut len = 0;
            unsafe { gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len) };
            let error = Self::get_shader_log(shader, len as usize);
            return Err(ShaderError::Compilation(error));
        }

        Ok(shader)
    }

    /// Links shader program and validates result
    fn link_program(program: GLuint) -> Result<(), ShaderError> {
        unsafe {
            gl::LinkProgram(program);
            gl::ValidateProgram(program);
        }

        let mut success = 1;
        unsafe { gl::GetProgramiv(program, gl::LINK_STATUS, &mut success) };
        if success == 0 {
            let mut len = 0;
            unsafe { gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len) };
            let error = Self::get_program_log(program, len as usize);
            return Err(ShaderError::Linking(error));
        }

        Ok(())
    }

    /// Detects supported features by checking uniform locations
    fn detect_features(&mut self) {
        self.variant_support = self.get_uniform_location("material.variantData").is_ok();
        self.connection_support = self.get_uniform_location("connectedDirections").is_ok();
    }

    /// Sets up block material properties including variants
    
    pub fn set_block_material(&mut self, material: &BlockMaterial) -> Result<(), ShaderError> {
        self.set_uniform_vec4("material.albedo", &material.albedo)?;
        self.set_uniform_1f("material.roughness", material.roughness)?;
        self.set_uniform_1f("material.metallic", material.metallic)?;
        self.set_uniform_vec3("material.emissive", &material.emissive)?;
        Ok(())
    }


    /// Sets connected texture directions using bitflags
    pub fn set_connected_textures(&mut self, connections: u8) -> Result<(), ShaderError> {
        self.set_uniform_1i("connectedDirections", connections as i32)
    }

    /// Generic uniform setting methods
    pub fn get_uniform_location(&mut self, name: &str) -> Result<GLint, ShaderError> {
        if let Some(loc) = self.uniforms.get(name) {
            return Ok(*loc);
        }

        let cname = CString::new(name).map_err(|e| ShaderError::Nul(e))?;
        let location = unsafe { gl::GetUniformLocation(self.id, cname.as_ptr()) };
        
        if location == -1 {
            return Err(ShaderError::UniformNotFound(name.to_string()));
        }

        self.uniforms.insert(name.to_string(), location);
        Ok(location)
    }

    pub fn set_uniform_1i(&mut self, name: &str, value: i32) -> Result<(), ShaderError> {
        let loc = self.get_uniform_location(name)?;
        unsafe { gl::Uniform1i(loc, value) };
        Ok(())
    }

    pub fn set_uniform_1f(&mut self, name: &str, value: f32) -> Result<(), ShaderError> {
        let loc = self.get_uniform_location(name)?;
        unsafe { gl::Uniform1f(loc, value) };
        Ok(())
    }

    pub fn set_uniform_vec3(&mut self, name: &str, value: &[f32; 3]) -> Result<(), ShaderError> {
        let loc = self.get_uniform_location(name)?;
        unsafe { gl::Uniform3f(loc, value[0], value[1], value[2]) };
        Ok(())
    }

    pub fn set_uniform_mat4(&mut self, name: &str, value: &[f32; 16]) -> Result<(), ShaderError> {
        let loc = self.get_uniform_location(name)?;
        unsafe { gl::UniformMatrix4fv(loc, 1, gl::FALSE, value.as_ptr()) };
        Ok(())
    }

    /// Internal logging utilities
    fn get_shader_log(shader: GLuint, len: usize) -> String {
        let mut buffer = Vec::with_capacity(len);
        unsafe {
            gl::GetShaderInfoLog(shader, len as i32, ptr::null_mut(), buffer.as_mut_ptr() as *mut GLchar);
            buffer.set_len(len);
        }
        String::from_utf8_lossy(&buffer).into_owned()
    }

    fn get_program_log(program: GLuint, len: usize) -> String {
        let mut buffer = Vec::with_capacity(len);
        unsafe {
            gl::GetProgramInfoLog(program, len as i32, ptr::null_mut(), buffer.as_mut_ptr() as *mut GLchar);
            buffer.set_len(len);
        }
        String::from_utf8_lossy(&buffer).into_owned()
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id) };
    }
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
        fragment_path: &str
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
}
