//! Rendering pipeline
pub mod chunk_renderer;
pub mod mesh;
pub mod texture;
pub mod camera;

// Public interface
pub use chunk_renderer::{ChunkRenderer, ChunkMesh};
pub use mesh::{Mesh, Vertex};
pub use texture::TextureAtlas;
pub use camera::{Camera, Projection};

/// Maximum render distance (in chunks)
pub const RENDER_DISTANCE: u32 = 16;
