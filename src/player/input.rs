use crate::player::physics::PlayerState;
use glam::Vec2;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::KeyCode;

#[derive(Debug, Default)]
pub struct PlayerInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub fly_up: bool,
    pub fly_down: bool,
    pub sprint: bool,
    pub crouch: bool,
    pub mouse_delta: Vec2,
    pub mouse_position: Vec2,
    pub mouse_scroll: Vec2,
    pub zoom_delta: Option<MouseScrollDelta>,
}

impl PlayerInput {
    pub fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        self.zoom_delta = Some(delta);
    }

    pub fn handle_mouse_move(&mut self, delta: Vec2) {
        self.mouse_delta = delta;
    }

    pub fn handle_key(&mut self, key: KeyCode, pressed: bool) {
        match key {
            KeyCode::KeyW => self.forward = pressed,
            KeyCode::KeyS => self.backward = pressed,
            KeyCode::KeyA => self.left = pressed,
            KeyCode::KeyD => self.right = pressed,
            KeyCode::Space => self.jump = pressed,
            KeyCode::ShiftLeft => self.sprint = pressed,
            KeyCode::KeyE => self.fly_up = pressed,
            KeyCode::KeyQ => self.fly_down = pressed,
            KeyCode::ControlLeft => self.crouch = pressed,
            _ => {}
        }
    }

    pub fn reset(&mut self) {
        self.mouse_delta = Vec2::ZERO;
        self.zoom_delta = None;
    }
}
