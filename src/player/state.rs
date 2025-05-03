use glam::{Vec3, Vec2};

#[derive(Debug)]
pub struct PlayerState {
    pub position: Vec3,
    pub rotation: Vec2,
    pub velocity: Vec3,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec2::ZERO,
            velocity: Vec3::ZERO,
        }
    }
} 
