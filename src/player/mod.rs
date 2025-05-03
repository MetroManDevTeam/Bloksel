//! Player systems
pub mod controller;
pub mod state;

// Public interface
pub use controller::PlayerController;
pub use state::{PlayerState, Inventory, GameMode};

/// Default player eye height
pub const EYE_HEIGHT: f32 = 1.62;
