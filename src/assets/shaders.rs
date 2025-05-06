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