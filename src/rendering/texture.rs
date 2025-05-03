pub struct Texture {
    pub width: u32,
    pub height: u32,
}

impl Texture {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}
