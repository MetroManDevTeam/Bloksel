use gl::types::*;
use std::ffi::{CString, NulError};
use std::fs;
use std::ptr;
use std::str;

#[derive(Debug)]
pub enum ShaderError {
    CompilationFailed(String),
    LinkingFailed(String),
    IoError(std::io::Error),
    NulError(NulError),
}

impl From<std::io::Error> for ShaderError {
    fn from(err: std::io::Error) -> Self {
        ShaderError::IoError(err)
    }
}

impl From<NulError> for ShaderError {
    fn from(err: NulError) -> Self {
        ShaderError::NulError(err)
    }
}

pub struct ShaderProgram {
    id: GLuint,
    uniforms: std::collections::HashMap<String, GLint>,
}

impl ShaderProgram {
    pub fn new(vertex_path: &str, fragment_path: &str) -> Result<Self, ShaderError> {
        let vertex_shader = ShaderProgram::compile_shader(
            vertex_path,
            gl::VERTEX_SHADER,
        )?;
        let fragment_shader = ShaderProgram::compile_shader(
            fragment_path,
            gl::FRAGMENT_SHADER,
        )?;

        let program = unsafe { gl::CreateProgram() };
        unsafe {
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
        }

        let mut success = 1;
        unsafe {
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
        }

        if success == 0 {
            let mut len = 0;
            unsafe {
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = ShaderProgram::create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl::GetProgramInfoLog(
                    program,
                    len,
                    ptr::null_mut(),
                    error.as_ptr() as *mut GLchar,
                );
            }

            return Err(ShaderError::LinkingFailed(error.to_string_lossy().into_owned()));
        }

        Ok(ShaderProgram {
            id: program,
            uniforms: std::collections::HashMap::new(),
        })
    }

    pub fn with_geometry(
        vertex_path: &str,
        geometry_path: &str,
        fragment_path: &str,
    ) -> Result<Self, ShaderError> {
        let vertex_shader = ShaderProgram::compile_shader(
            vertex_path,
            gl::VERTEX_SHADER,
        )?;
        let geometry_shader = ShaderProgram::compile_shader(
            geometry_path,
            gl::GEOMETRY_SHADER,
        )?;
        let fragment_shader = ShaderProgram::compile_shader(
            fragment_path,
            gl::FRAGMENT_SHADER,
        )?;

        let program = unsafe { gl::CreateProgram() };
        unsafe {
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, geometry_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(geometry_shader);
            gl::DeleteShader(fragment_shader);
        }

        let mut success = 1;
        unsafe {
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
        }

        if success == 0 {
            let mut len = 0;
            unsafe {
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = ShaderProgram::create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl::GetProgramInfoLog(
                    program,
                    len,
                    ptr::null_mut(),
                    error.as_ptr() as *mut GLchar,
                );
            }

            return Err(ShaderError::LinkingFailed(error.to_string_lossy().into_owned()));
        }

        Ok(ShaderProgram {
            id: program,
            uniforms: std::collections::HashMap::new(),
        })
    }

    fn compile_shader(path: &str, shader_type: GLenum) -> Result<GLuint, ShaderError> {
        let shader_source = fs::read_to_string(path)?;
        let shader_source_cstring = CString::new(shader_source.as_bytes())?;

        let shader = unsafe { gl::CreateShader(shader_type) };

        unsafe {
            gl::ShaderSource(
                shader,
                1,
                &shader_source_cstring.as_ptr(),
                ptr::null(),
            );
            gl::CompileShader(shader);
        }

        let mut success = 1;
        unsafe {
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        }

        if success == 0 {
            let mut len = 0;
            unsafe {
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = ShaderProgram::create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl::GetShaderInfoLog(
                    shader,
                    len,
                    ptr::null_mut(),
                    error.as_ptr() as *mut GLchar,
                );
            }

            return Err(ShaderError::CompilationFailed(error.to_string_lossy().into_owned()));
        }

        Ok(shader)
    }

    fn create_whitespace_cstring_with_len(len: usize) -> CString {
        // Allocate buffer of correct size
        let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
        // Fill it with spaces
        buffer.extend([b' '].iter().cycle().take(len));
        // Convert buffer to CString
        unsafe { CString::from_vec_unchecked(buffer) }
    }

    pub fn id(&self) -> GLuint {
        self.id
    }

    pub fn set_used(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn get_uniform_location(&mut self, name: &str) -> GLint {
        if let Some(location) = self.uniforms.get(name) {
            return *location;
        }

        let cname = CString::new(name).unwrap();
        let location = unsafe { gl::GetUniformLocation(self.id, cname.as_ptr()) };

        if location == -1 {
            log::warn!("Uniform '{}' not found in shader", name);
        }

        self.uniforms.insert(name.to_string(), location);
        location
    }

    // Uniform setters
    pub fn set_uniform_1i(&mut self, name: &str, value: i32) {
        self.set_used();
        let location = self.get_uniform_location(name);
        unsafe {
            gl::Uniform1i(location, value);
        }
    }

    pub fn set_uniform_1f(&mut self, name: &str, value: f32) {
        self.set_used();
        let location = self.get_uniform_location(name);
        unsafe {
            gl::Uniform1f(location, value);
        }
    }

    pub fn set_uniform_3f(&mut self, name: &str, x: f32, y: f32, z: f32) {
        self.set_used();
        let location = self.get_uniform_location(name);
        unsafe {
            gl::Uniform3f(location, x, y, z);
        }
    }

    pub fn set_uniform_mat4(&mut self, name: &str, mat: &[f32; 16]) {
        self.set_used();
        let location = self.get_uniform_location(name);
        unsafe {
            gl::UniformMatrix4fv(location, 1, gl::FALSE, mat.as_ptr());
        }
    }

    pub fn set_uniform_vec3(&mut self, name: &str, vec: &[f32; 3]) {
        self.set_used();
        let location = self.get_uniform_location(name);
        unsafe {
            gl::Uniform3fv(location, 1, vec.as_ptr());
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

// Predefined shaders for voxel rendering
pub mod voxel_shaders {
    use super::*;

    pub const VERTEX_SOURCE: &str = r#"
        #version 330 core
        layout (location = 0) in vec3 aPos;
        layout (location = 1) in vec3 aNormal;
        layout (location = 2) in vec3 aTangent;
        layout (location = 3) in vec3 aBitangent;
        layout (location = 4) in vec2 aTexCoord;
        layout (location = 5) in float aBlockId;

        out vec3 FragPos;
        out vec3 Normal;
        out vec2 TexCoord;
        out vec3 Tangent;
        out vec3 Bitangent;
        out float BlockId;

        uniform mat4 model;
        uniform mat4 view;
        uniform mat4 projection;

        void main() {
            FragPos = vec3(model * vec4(aPos, 1.0));
            Normal = mat3(transpose(inverse(model))) * aNormal;
            Tangent = mat3(model) * aTangent;
            Bitangent = mat3(model) * aBitangent;
            TexCoord = aTexCoord;
            BlockId = aBlockId;
            
            gl_Position = projection * view * vec4(FragPos, 1.0);
        }
    "#;

    pub const FRAGMENT_SOURCE: &str = r#"
        #version 330 core
        out vec4 FragColor;

        in vec3 FragPos;
        in vec3 Normal;
        in vec2 TexCoord;
        in vec3 Tangent;
        in vec3 Bitangent;
        in float BlockId;

        uniform sampler2D textureAtlas;
        uniform vec3 viewPos;
        uniform vec3 lightPos;
        uniform vec3 lightColor;
        uniform float lightIntensity;

        // Normal mapping
        vec3 getNormal() {
            vec3 normal = normalize(Normal);
            vec3 tangent = normalize(Tangent);
            vec3 bitangent = normalize(Bitangent);
            
            mat3 TBN = mat3(tangent, bitangent, normal);
            vec3 sampledNormal = texture(textureAtlas, TexCoord).rgb;
            sampledNormal = sampledNormal * 2.0 - 1.0;
            
            return normalize(TBN * sampledNormal);
        }

        void main() {
            // Sample texture
            vec4 texColor = texture(textureAtlas, TexCoord);
            if (texColor.a < 0.1) {
                discard;
            }

            // Lighting calculations
            vec3 norm = getNormal();
            vec3 lightDir = normalize(lightPos - FragPos);
            
            // Diffuse
            float diff = max(dot(norm, lightDir), 0.0);
            vec3 diffuse = diff * lightColor * lightIntensity;
            
            // Specular
            vec3 viewDir = normalize(viewPos - FragPos);
            vec3 reflectDir = reflect(-lightDir, norm);
            float spec = pow(max(dot(viewDir, reflectDir), 0.0), 32.0);
            vec3 specular = 0.5 * spec * lightColor;
            
            // Ambient
            vec3 ambient = 0.2 * lightColor;
            
            // Combine results
            vec3 result = (ambient + diffuse + specular) * texColor.rgb;
            FragColor = vec4(result, texColor.a);
        }
    "#;

    pub const GEOMETRY_SOURCE: &str = r#"
        #version 330 core
        layout (triangles) in;
        layout (triangle_strip, max_vertices = 3) out;

        in vec3 FragPos[];
        in vec3 Normal[];
        in vec2 TexCoord[];
        in vec3 Tangent[];
        in vec3 Bitangent[];
        in float BlockId[];

        out vec3 gFragPos;
        out vec3 gNormal;
        out vec2 gTexCoord;
        out vec3 gTangent;
        out vec3 gBitangent;
        out float gBlockId;

        uniform mat4 model;
        uniform mat4 view;
        uniform mat4 projection;

        void main() {
            for(int i = 0; i < 3; i++) {
                gFragPos = FragPos[i];
                gNormal = Normal[i];
                gTexCoord = TexCoord[i];
                gTangent = Tangent[i];
                gBitangent = Bitangent[i];
                gBlockId = BlockId[i];
                
                gl_Position = gl_in[i].gl_Position;
                EmitVertex();
            }
            EndPrimitive();
        }
    "#;

    pub const WIREFRAME_GEOMETRY_SOURCE: &str = r#"
        #version 330 core
        layout (triangles) in;
        layout (line_strip, max_vertices = 4) out;

        in vec3 FragPos[];

        uniform mat4 model;
        uniform mat4 view;
        uniform mat4 projection;
        uniform float thickness;

        void main() {
            vec4 pos0 = gl_in[0].gl_Position;
            vec4 pos1 = gl_in[1].gl_Position;
            vec4 pos2 = gl_in[2].gl_Position;
            
            // Edge 1
            gl_Position = pos0;
            EmitVertex();
            gl_Position = pos1;
            EmitVertex();
            EndPrimitive();
            
            // Edge 2
            gl_Position = pos1;
            EmitVertex();
            gl_Position = pos2;
            EmitVertex();
            EndPrimitive();
            
            // Edge 3
            gl_Position = pos2;
            EmitVertex();
            gl_Position = pos0;
            EmitVertex();
            EndPrimitive();
        }
    "#;

    pub fn create_default_voxel_shader() -> Result<ShaderProgram, ShaderError> {
        // In a real application, you would load these from files
        ShaderProgram::new_from_source(VERTEX_SOURCE, FRAGMENT_SOURCE)
    }

    pub fn create_voxel_shader_with_geometry() -> Result<ShaderProgram, ShaderError> {
        ShaderProgram::with_geometry_from_source(
            VERTEX_SOURCE,
            GEOMETRY_SOURCE,
            FRAGMENT_SOURCE,
        )
    }
}

// Extension for creating shaders from source strings
impl ShaderProgram {
    pub fn new_from_source(vertex_source: &str, fragment_source: &str) -> Result<Self, ShaderError> {
        let vertex_shader = ShaderProgram::compile_shader_from_source(
            vertex_source,
            gl::VERTEX_SHADER,
        )?;
        let fragment_shader = ShaderProgram::compile_shader_from_source(
            fragment_source,
            gl::FRAGMENT_SHADER,
        )?;

        let program = unsafe { gl::CreateProgram() };
        unsafe {
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
        }

        Self::check_link_status(program)
    }

    pub fn with_geometry_from_source(
        vertex_source: &str,
        geometry_source: &str,
        fragment_source: &str,
    ) -> Result<Self, ShaderError> {
        let vertex_shader = ShaderProgram::compile_shader_from_source(
            vertex_source,
            gl::VERTEX_SHADER,
        )?;
        let geometry_shader = ShaderProgram::compile_shader_from_source(
            geometry_source,
            gl::GEOMETRY_SHADER,
        )?;
        let fragment_shader = ShaderProgram::compile_shader_from_source(
            fragment_source,
            gl::FRAGMENT_SHADER,
        )?;

        let program = unsafe { gl::CreateProgram() };
        unsafe {
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, geometry_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(geometry_shader);
            gl::DeleteShader(fragment_shader);
        }

        Self::check_link_status(program)
    }

    fn compile_shader_from_source(source: &str, shader_type: GLenum) -> Result<GLuint, ShaderError> {
        let shader = unsafe { gl::CreateShader(shader_type) };

        let c_str = CString::new(source.as_bytes())?;
        unsafe {
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
            gl::CompileShader(shader);
        }

        let mut success = 1;
        unsafe {
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        }

        if success == 0 {
            let mut len = 0;
            unsafe {
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = ShaderProgram::create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl::GetShaderInfoLog(
                    shader,
                    len,
                    ptr::null_mut(),
                    error.as_ptr() as *mut GLchar,
                );
            }

            return Err(ShaderError::CompilationFailed(error.to_string_lossy().into_owned()));
        }

        Ok(shader)
    }

    fn check_link_status(program: GLuint) -> Result<Self, ShaderError> {
        let mut success = 1;
        unsafe {
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
        }

        if success == 0 {
            let mut len = 0;
            unsafe {
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = ShaderProgram::create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl::GetProgramInfoLog(
                    program,
                    len,
                    ptr::null_mut(),
                    error.as_ptr() as *mut GLchar,
                );
            }

            return Err(ShaderError::LinkingFailed(error.to_string_lossy().into_owned()));
        }

        Ok(ShaderProgram {
            id: program,
            uniforms: std::collections::HashMap::new(),
        })
    }
}
