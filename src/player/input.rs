use winit::event::{KeyboardInput, MouseButton};
use crate::physics::PlayerState;

#[derive(Default)]
pub struct PlayerInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sprint: bool,
    pub fly_up: bool,
    pub fly_down: bool,
    pub mouse_delta: Vec2,
    pub zoom_delta: Option<MouseScrollDelta>,
}

impl PlayerInput {
   pub fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        self.zoom_delta = Some(delta);
    }
}
