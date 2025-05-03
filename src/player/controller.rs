use winit::{
    event::{ElementState, VirtualKeyCode, MouseScrollDelta, MouseButton},
    keyboard::KeyCode,
};
use glam::{Vec2, Vec3};
use crate::{
    core::world::World,
    physics::handler::PlayerPhysicsHandler,
    rendering::camera::{Camera, Projection},
};

#[derive(Debug, Default)]
pub struct PlayerInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub crouch: bool,
    pub sprint: bool,
    pub fly: bool,
    pub pitch: f32,
    pub yaw: f32,
    pub mouse_sensitivity: f32,
}

#[derive(Debug)]
pub struct PlayerController {
    pub input: PlayerInput,
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Vec2, // pitch, yaw
    pub camera: Camera,
    pub fly_mode: bool,
    pub physics_handler: PlayerPhysicsHandler,
}

impl PlayerController {
    pub fn new(position: Vec3) -> Self {
        let mut camera = Camera::new(
            position,
            Vec2::new(0.0, 0.0),
            Projection::Perspective {
                fov: 70.0,
                aspect_ratio: 16.0 / 9.0,
                near: 0.1,
                far: 1000.0,
            },
        );

        camera.update_vectors();

        Self {
            input: PlayerInput {
                mouse_sensitivity: 0.002,
                ..Default::default()
            },
            position,
            velocity: Vec3::ZERO,
            rotation: Vec2::new(0.0, 0.0),
            camera,
            fly_mode: false,
            physics_handler: PlayerPhysicsHandler::default(),
        }
    }

    pub fn handle_keyboard_input(&mut self, key: KeyCode, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        match key {
            KeyCode::KeyW => self.input.forward = pressed,
            KeyCode::KeyS => self.input.backward = pressed,
            KeyCode::KeyA => self.input.left = pressed,
            KeyCode::KeyD => self.input.right = pressed,
            KeyCode::Space => self.input.jump = pressed,
            KeyCode::ShiftLeft => self.input.sprint = pressed,
            KeyCode::ControlLeft => self.input.crouch = pressed,
            KeyCode::KeyF => {
                if pressed {
                    self.fly_mode = !self.fly_mode;
                }
            },
            _ => {}
        }
    }

    pub fn handle_mouse_motion(&mut self, delta: (f64, f64)) {
        self.input.yaw += delta.0 as f32 * self.input.mouse_sensitivity;
        self.input.pitch += delta.1 as f32 * self.input.mouse_sensitivity;

        // Clamp pitch to prevent over-rotation
        self.input.pitch = self.input.pitch.clamp(
            -std::f32::consts::PI / 2.0 + 0.01,
            std::f32::consts::PI / 2.0 - 0.01,
        );

        self.rotation = Vec2::new(self.input.pitch, self.input.yaw);
        self.camera.set_rotation(self.rotation);
    }

    pub fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        let scroll = match delta {
            MouseScrollDelta::LineDelta(_, y) => y * 2.0,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
        };

        if let Projection::Perspective { fov, .. } = &mut self.camera.projection {
            *fov = (*fov - scroll).clamp(30.0, 110.0);
            self.camera.update_projection_matrix();
        }
    }

    pub fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        // Handle block breaking/placing
        // TODO: Implement block interaction
    }

    pub fn update(&mut self, world: &World, delta_time: f32) {
        // Update physics
        self.physics_handler.update(
            delta_time,
            self,
            world,
        );

        // Update camera position
        self.camera.position = self.position + Vec3::new(0.0, 1.62, 0.0); // Eye height
        self.camera.update_view_matrix();

        // Apply movement input
        let move_dir = self.get_movement_direction();
        self.physics_handler.apply_movement(self, move_dir, delta_time);

        // Handle jumping
        if self.input.jump {
            self.physics_handler.attempt_jump(self, world);
        }
    }

    fn get_movement_direction(&self) -> Vec3 {
        let mut direction = Vec3::ZERO;

        if self.input.forward {
            direction += self.camera.front;
        }
        if self.input.backward {
            direction -= self.camera.front;
        }
        if self.input.right {
            direction += self.camera.right;
        }
        if self.input.left {
            direction -= self.camera.right;
        }

        // Normalize and remove vertical component (unless flying)
        if self.fly_mode {
            direction.normalize_or_zero()
        } else {
            direction.y = 0.0;
            direction.normalize_or_zero()
        }
    }

    pub fn get_view_matrix(&self) -> Mat4 {
        self.camera.view_matrix()
    }

    pub fn get_projection_matrix(&self) -> Mat4 {
        self.camera.projection_matrix()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if let Projection::Perspective { aspect_ratio, .. } = &mut self.camera.projection {
            *aspect_ratio = width as f32 / height as f32;
            self.camera.update_projection_matrix();
        }
    }
}

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub front: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub world_up: Vec3,
    pub rotation: Vec2, // pitch, yaw
    pub projection: Projection,
    view_matrix: Mat4,
    projection_matrix: Mat4,
}

impl Camera {
    pub fn new(position: Vec3, rotation: Vec2, projection: Projection) -> Self {
        let mut camera = Self {
            position,
            front: Vec3::new(0.0, 0.0, -1.0),
            right: Vec3::ZERO,
            up: Vec3::ZERO,
            world_up: Vec3::Y,
            rotation,
            projection,
            view_matrix: Mat4::IDENTITY,
            projection_matrix: Mat4::IDENTITY,
        };

        camera.update_vectors();
        camera.update_view_matrix();
        camera.update_projection_matrix();

        camera
    }

    pub fn update_vectors(&mut self) {
        // Calculate front vector from rotation
        self.front = Vec3::new(
            self.rotation.y.cos() * self.rotation.x.cos(),
            self.rotation.x.sin(),
            self.rotation.y.sin() * self.rotation.x.cos(),
        ).normalize();

        // Re-calculate right and up vectors
        self.right = self.front.cross(self.world_up).normalize();
        self.up = self.right.cross(self.front).normalize();
    }

    pub fn set_rotation(&mut self, rotation: Vec2) {
        self.rotation = rotation;
        self.update_vectors();
    }

    pub fn update_view_matrix(&mut self) {
        self.view_matrix = Mat4::look_to_rh(
            self.position,
            self.front,
            self.up,
        );
    }

    pub fn update_projection_matrix(&mut self) {
        self.projection_matrix = match self.projection {
            Projection::Perspective { fov, aspect_ratio, near, far } => {
                Mat4::perspective_rh(fov.to_radians(), aspect_ratio, near, far)
            }
        };
    }

    pub fn view_matrix(&self) -> Mat4 {
        self.view_matrix
    }

    pub fn projection_matrix(&self) -> Mat4 {
        self.projection_matrix
    }
}

#[derive(Debug, Clone)]
pub enum Projection {
    Perspective {
        fov: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_camera_vectors() {
        let mut camera = Camera::new(
            Vec3::ZERO,
            Vec2::new(0.0, 0.0),
            Projection::Perspective {
                fov: 70.0,
                aspect_ratio: 16.0 / 9.0,
                near: 0.1,
                far: 1000.0,
            },
        );

        // Test initial orientation
        assert_relative_eq!(camera.front, Vec3::new(0.0, 0.0, -1.0));
        assert_relative_eq!(camera.right, Vec3::new(1.0, 0.0, 0.0));
        assert_relative_eq!(camera.up, Vec3::new(0.0, 1.0, 0.0));

        // Test rotation
        camera.set_rotation(Vec2::new(0.0, std::f32::consts::PI / 2.0));
        assert_relative_eq!(camera.front, Vec3::new(0.0, 0.0, 1.0), epsilon = 0.0001);
    }

    #[test]
    fn test_player_movement() {
        let mut player = PlayerController::new(Vec3::ZERO);
        let world = World::new(12345, 10, 100);

        // Test forward movement
        player.input.forward = true;
        player.update(&world, 1.0);
        assert!(player.velocity.z < 0.0, "Player should move forward");
    }
}
