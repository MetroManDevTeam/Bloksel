//! Physics systems
pub mod collision;
pub mod handler;
pub mod materials;

// Core types
pub use collision::{AABB, Collision, CollisionProperties};
pub use handler::PlayerPhysicsHandler;
pub use materials::{MaterialProperties, PhysicsMaterials};

/// Physics timestep (60Hz)
pub const PHYSICS_DT: f32 = 1.0 / 60.0;
