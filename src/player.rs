use glam::{Vec3, Vec2, Quat};
use winit::event::{ElementState, VirtualKeyCode, MouseScrollDelta};
use std::f32::consts::PI;

pub struct Player {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Vec2, // pitch and yaw
    pub speed: f32,
    pub is_flying: bool,
    pub is_crouching: bool,
    pub is_sprinting: bool,
    pub on_ground: bool,
    pub height: f32,
    pub eye_height: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 70.0, 0.0), // Start above ground
            velocity: Vec3::ZERO,
            rotation: Vec2::ZERO,
            speed: 4.317, // Default Minecraft walking speed (m/s)
            is_flying: false,
            is_crouching: false,
            is_sprinting: false,
            on_ground: true,
            height: 1.8, // Standard player height (meters)
            eye_height: 1.62, // Eye level (meters)
        }
    }
}

impl Player {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        // Apply gravity if not flying
        if !self.is_flying {
            self.velocity.y -= 32.0 * delta_time; // Minecraft gravity (blocks/tick²)
        }

        // Apply velocity to position
        self.position += self.velocity * delta_time;

        // Cap falling speed
        if self.velocity.y < -39.2 { // Terminal velocity
            self.velocity.y = -39.2;
        }

        // Apply friction
        if self.on_ground {
            self.velocity.x *= 0.6;
            self.velocity.z *= 0.6;
        } else {
            self.velocity.x *= 0.91; // Air resistance
            self.velocity.z *= 0.91;
        }
    }

    pub fn handle_keyboard_input(&mut self, key: VirtualKeyCode, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        
        match key {
            VirtualKeyCode::LShift | VirtualKeyCode::RShift => {
                self.is_crouching = pressed;
                self.eye_height = if pressed { 1.27 } else { 1.62 }; // Adjust eye height
            },
            VirtualKeyCode::Space => {
                if pressed && (self.on_ground || self.is_flying) {
                    self.jump();
                }
            },
            VirtualKeyCode::W => self.is_sprinting = pressed,
            VirtualKeyCode::F => {
                if pressed {
                    self.is_flying = !self.is_flying;
                    if self.is_flying {
                        self.velocity.y = 0.0;
                    }
                }
            },
            _ => {}
        }
    }

    pub fn handle_mouse_movement(&mut self, delta: (f64, f64)) {
        let sensitivity = 0.0025;
        self.rotation.x -= (delta.1 as f32) * sensitivity;
        self.rotation.y -= (delta.0 as f32) * sensitivity;

        // Clamp pitch to prevent over-rotation
        self.rotation.x = self.rotation.x.clamp(-PI/2.0 + 0.01, PI/2.0 - 0.01);
    }

    pub fn jump(&mut self) {
        if self.is_flying {
            self.velocity.y = 5.0; // Fly up
        } else if self.on_ground {
            self.velocity.y = 8.0; // Jump velocity
            self.on_ground = false;
        }
    }

    pub fn get_movement_direction(&self, input: &PlayerInput) -> Vec3 {
        let (mut forward, right) = self.get_orientation_vectors();

        // If flying, allow vertical movement
        if self.is_flying {
            forward = forward.normalize_or_zero();
        } else {
            forward.y = 0.0;
            forward = forward.normalize_or_zero();
        }

        let mut direction = Vec3::ZERO;

        if input.forward {
            direction += forward;
        }
        if input.backward {
            direction -= forward;
        }
        if input.right {
            direction += right;
        }
        if input.left {
            direction -= right;
        }
        if self.is_flying {
            if input.up {
                direction.y += 1.0;
            }
            if input.down {
                direction.y -= 1.0;
            }
        }

        // Normalize and apply sprinting
        let mut speed = self.speed;
        if self.is_sprinting {
            speed *= 1.3; // Sprinting speed boost
        }
        if self.is_crouching {
            speed *= 0.3; // Crouching speed reduction
        }

        direction.normalize_or_zero() * speed
    }

    pub fn get_orientation_vectors(&self) -> (Vec3, Vec3) {
        let pitch = Quat::from_rotation_x(self.rotation.x);
        let yaw = Quat::from_rotation_y(self.rotation.y);
        let orientation = yaw * pitch;

        let forward = orientation * -Vec3::Z;
        let right = orientation * Vec3::X;

        (forward, right)
    }

    pub fn get_view_matrix(&self) -> glam::Mat4 {
        let eye_pos = self.position + Vec3::new(0.0, self.eye_height, 0.0);
        let pitch = Quat::from_rotation_x(self.rotation.x);
        let yaw = Quat::from_rotation_y(self.rotation.y);
        
        glam::Mat4::look_to_rh(
            eye_pos,
            (yaw * pitch) * -Vec3::Z,
            Vec3::Y
        )
    }
}

pub struct PlayerInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

impl Default for PlayerInput {
    fn default() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
        }
    }
}
