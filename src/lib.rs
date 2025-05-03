//! Voxel engine core library

// Core systems
pub mod core;
pub mod rendering;
pub mod physics;
pub mod player;
pub mod terrain;
pub mod ui;

/// Engine prelude
pub mod prelude {
    pub use crate::core::*;
    pub use crate::rendering::*;
    pub use crate::physics::*;
    pub use crate::player::*;
    pub use crate::terrain::*;
    pub use crate::ui::*;
    
    // Commonly used external types
    pub use glam::{Vec2, Vec3, Vec4, Mat4};
    pub use parking_lot::{RwLock, Mutex};
}

/// Current engine version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
