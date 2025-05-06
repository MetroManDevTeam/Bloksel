use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::KeyCode;
use glam::Vec2;
use crate::player::physics::PlayerState;

#[derive(Default)]
pub struct PlayerInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sprint: bool,
    pub mouse_delta: Vec2,
    pub zoom_delta: Option<MouseScrollDelta>,
}

impl PlayerInput {
   pub fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
       self.zoom_delta = Some(delta);
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
           _ => (),
       }
   }

   pub fn reset(&mut self) {
       self.mouse_delta = Vec2::ZERO;
       self.zoom_delta = None;
   }
}
