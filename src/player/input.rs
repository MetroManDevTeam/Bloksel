use crate::player::physics::PlayerState;
use glam::Vec2;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::KeyCode;

#[derive(Debug, Default)]
pub struct InputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub sprint: bool,
    pub fly_up: bool,
    pub fly_down: bool,
    pub crouch: bool,
    pub mouse_delta: (f32, f32),
    pub mouse_scroll: f32,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) {
        match key {
            KeyCode::KeyW => self.forward = pressed,
            KeyCode::KeyS => self.backward = pressed,
            KeyCode::KeyA => self.left = pressed,
            KeyCode::KeyD => self.right = pressed,
            KeyCode::ShiftLeft => self.sprint = pressed,
            KeyCode::KeyE => self.fly_up = pressed,
            KeyCode::KeyQ => self.fly_down = pressed,
            KeyCode::ControlLeft => self.crouch = pressed,
            _ => (),
        }
    }

    pub fn handle_mouse_motion(&mut self, delta: (f32, f32)) {
        self.mouse_delta = delta;
    }

    pub fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(_, y) => self.mouse_scroll = y,
            MouseScrollDelta::PixelDelta(pos) => self.mouse_scroll = pos.y as f32,
        }
    }

    pub fn reset_frame_input(&mut self) {
        self.mouse_delta = (0.0, 0.0);
        self.mouse_scroll = 0.0;
    }
}
