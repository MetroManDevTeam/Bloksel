use crate::player::physics::PlayerState;
use glam::Vec2;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::KeyCode;

#[derive(Debug, Clone)]
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
    pub mouse_scroll: f32,
}

impl Default for PlayerInput {
    fn default() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            jump: false,
            fly_up: false,
            fly_down: false,
            sprint: false,
            crouch: false,
            mouse_delta: Vec2::ZERO,
            mouse_scroll: 0.0,
        }
    }
}

impl PlayerInput {
    pub fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        self.mouse_scroll = delta.line_delta;
    }

    pub fn handle_mouse_movement(&mut self, delta: Vec2) {
        self.mouse_delta = delta;
    }

    pub fn handle_key_input(&mut self, key: KeyCode, state: ElementState) {
        let pressed = state == ElementState::Pressed;
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
            _ => (),
        }
    }

    pub fn reset(&mut self) {
        self.mouse_delta = Vec2::ZERO;
        self.mouse_scroll = 0.0;
    }
}
