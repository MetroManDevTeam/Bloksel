use crate::world::block::BlockPhysics;
use crate::world::{BlockData, Chunk, ChunkCoord, TerrainGenerator};
use glam::{Mat4, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::f32::consts::{FRAC_PI_2, PI};
use winit::event::{ElementState, MouseScrollDelta};
use winit::keyboard::KeyCode;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum PlayerState {
    Normal,
    Flying,
    Spectator,
}

#[derive(Debug)]
pub struct Player {
    // Core properties
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Vec2,
    pub size: Vec3,

    // State management
    pub state: PlayerState,
    pub on_ground: bool,

    // Movement parameters
    pub base_speed: f32,
    pub speed_multiplier: f32,
    pub jump_force: f32,
    pub gravity: f32,

    // Camera controls
    pub sensitivity: f32,
    pub zoom_level: f32,
    pub max_zoom: f32,
    pub min_zoom: f32,

    // World interaction
    pub chunk_size: i32,
    pub collision_enabled: bool,
    pub last_safe_position: Vec3,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 70.0, 0.0),
            velocity: Vec3::ZERO,
            rotation: Vec2::new(0.0, 0.0),
            size: Vec3::new(0.6, 1.8, 0.6),
            state: PlayerState::Normal,
            on_ground: false,
            base_speed: 5.0,
            speed_multiplier: 1.0,
            jump_force: 8.0,
            gravity: 20.0,
            sensitivity: 0.002,
            zoom_level: 1.0,
            max_zoom: 2.5,
            min_zoom: 0.4,
            chunk_size: 30,
            collision_enabled: true,
            last_safe_position: Vec3::new(0.0, 70.0, 0.0),
        }
    }
}

impl Player {
    pub fn update(&mut self, dt: f32, terrain: &TerrainGenerator, input: &PlayerInput) {
        self.handle_rotation(input);
        self.handle_movement(dt, input);
        self.handle_zoom(input);
        self.apply_physics(dt);
        self.update_position(dt, terrain);
        self.clamp_rotation();
        self.update_safe_position();
    }

    fn handle_rotation(&mut self, input: &PlayerInput) {
        // Smooth rotation with edge case protection
        let mouse_delta = input.mouse_delta * self.sensitivity * self.zoom_level;
        self.rotation.x = (self.rotation.x + mouse_delta.x).rem_euclid(2.0 * PI);
        self.rotation.y =
            (self.rotation.y + mouse_delta.y).clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
    }

    fn handle_movement(&mut self, dt: f32, input: &PlayerInput) {
        let movement = self.calculate_movement_vector(input);
        let speed = self.calculate_current_speed(input);

        match self.state {
            PlayerState::Spectator => {
                // Instant velocity response with speed multiplier
                self.velocity = movement * speed * self.speed_multiplier;
                if input.fly_up {
                    self.velocity.y = speed * self.speed_multiplier;
                }
                if input.fly_down {
                    self.velocity.y = -speed * self.speed_multiplier;
                }
            }
            PlayerState::Flying => {
                // Smoother flight controls
                self.velocity = self.velocity.lerp(movement * speed, dt * 10.0);
                if input.fly_up {
                    self.velocity.y = speed;
                }
                if input.fly_down {
                    self.velocity.y = -speed;
                }
            }
            PlayerState::Normal => {
                // Ground-based movement with air control
                let acceleration = if self.on_ground { 15.0 } else { 3.0 };
                self.velocity.x += movement.x * acceleration * dt;
                self.velocity.z += movement.z * acceleration * dt;

                let friction = if self.on_ground { 0.7 } else { 0.98 };
                self.velocity.x *= friction;
                self.velocity.z *= friction;
            }
        }
    }

    fn calculate_movement_vector(&self, input: &PlayerInput) -> Vec3 {
        let forward = Vec3::new(self.rotation.x.sin(), 0.0, self.rotation.x.cos()).normalize();

        let right = Vec3::new(
            (self.rotation.x + FRAC_PI_2).sin(),
            0.0,
            (self.rotation.x + FRAC_PI_2).cos(),
        )
        .normalize();

        let mut move_vec = Vec3::ZERO;
        if input.forward {
            move_vec += forward;
        }
        if input.backward {
            move_vec -= forward;
        }
        if input.left {
            move_vec -= right;
        }
        if input.right {
            move_vec += right;
        }

        if move_vec.length_squared() > 0.0 {
            move_vec.normalize()
        } else {
            move_vec
        }
    }

    fn calculate_current_speed(&self, input: &PlayerInput) -> f32 {
        let base = match self.state {
            PlayerState::Spectator => self.base_speed * 3.0,
            _ => self.base_speed,
        };

        if input.sprint { base * 2.0 } else { base }
    }

    fn apply_physics(&mut self, dt: f32) {
        match self.state {
            PlayerState::Normal => {
                self.velocity.y -= self.gravity * dt;
                self.velocity.y = self.velocity.y.clamp(-self.gravity, self.gravity);
            }
            PlayerState::Flying => {
                self.velocity.y *= 0.95;
            }
            PlayerState::Spectator => {
                self.velocity *= 0.85;
            }
        }

        // Prevent extreme velocities
        self.velocity = self.velocity.clamp_length_max(100.0);
    }

    fn update_position(&mut self, dt: f32, terrain: &TerrainGenerator) {
        if self.collision_enabled && self.state != PlayerState::Spectator {
            self.move_with_collision(dt, terrain);
        } else {
            // Spectator mode free movement
            self.position += self.velocity * dt;
        }
    }

    fn move_with_collision(&mut self, dt: f32, terrain: &TerrainGenerator) {
        let original_position = self.position;
        let mut new_position = self.position + self.velocity * dt;

        // Multi-axis collision check
        for axis in 0..3 {
            let mut test_position = new_position;
            test_position[axis] = self.position[axis];

            if self.check_collision(test_position, terrain) {
                self.velocity[axis] = 0.0;
                new_position[axis] = self.position[axis];
            }
        }

        // Vertical collision handling
        if self.velocity.y != 0.0 {
            let test_position = Vec3::new(new_position.x, new_position.y, self.position.z);
            if self.check_collision(test_position, terrain) {
                self.velocity.y = 0.0;
                self.on_ground = self.velocity.y < 0.0;
                new_position.y = self.position.y;
            }
        }

        self.position = new_position;

        // Fallback to last safe position if stuck
        if self.check_collision(self.position, terrain) {
            self.position = self.last_safe_position;
            self.velocity = Vec3::ZERO;
        }
    }

    fn check_collision(&self, position: Vec3, terrain: &TerrainGenerator) -> bool {
        let chunk_coord = ChunkCoord::from_world(position);
        if let Some(chunk) = terrain.get_chunk(chunk_coord) {
            let local_pos = position - chunk_coord.to_world();
            // Perform the actual collision check using the chunk data
            return chunk.read().is_solid_at(local_pos);
        }
        false
    }

    fn update_safe_position(&mut self) {
        if self.on_ground && self.velocity.length() < 0.1 {
            self.last_safe_position = self.position;
        }
    }

    fn clamp_rotation(&mut self) {
        self.rotation.x = self.rotation.x.rem_euclid(2.0 * PI);
        self.rotation.y = self.rotation.y.clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
    }

    fn handle_zoom(&mut self, input: &PlayerInput) {
        if self.state != PlayerState::Spectator {
            return;
        }

        let zoom_speed = 0.1;
        match input.zoom_delta {
            Some(MouseScrollDelta::LineDelta(_, y)) => {
                self.zoom_level =
                    (self.zoom_level - y * zoom_speed).clamp(self.min_zoom, self.max_zoom);
            }
            Some(MouseScrollDelta::PixelDelta(pos)) => {
                self.zoom_level =
                    (self.zoom_level - pos.y as f32 * 0.01).clamp(self.min_zoom, self.max_zoom);
            }
            None => {} // Handle None case
        }
    }

    pub fn get_view_matrix(&self) -> Mat4 {
        let rotation_x = Mat4::from_rotation_x(-self.rotation.y);
        let rotation_y = Mat4::from_rotation_y(-self.rotation.x);
        let translation = Mat4::from_translation(-self.position);

        // Apply zoom for spectator mode
        let zoom = Mat4::from_scale(Vec3::splat(self.zoom_level));

        rotation_x * rotation_y * translation * zoom
    }

    pub fn toggle_state(&mut self) {
        self.state = match self.state {
            PlayerState::Normal => PlayerState::Flying,
            PlayerState::Flying => PlayerState::Spectator,
            PlayerState::Spectator => PlayerState::Normal,
        };

        // State transition logic
        match self.state {
            PlayerState::Spectator => {
                self.collision_enabled = false;
                self.velocity = Vec3::ZERO;
                self.zoom_level = 1.0;
            }
            PlayerState::Flying => {
                self.collision_enabled = true;
                self.velocity.y = 0.0;
            }
            PlayerState::Normal => {
                self.collision_enabled = true;
                self.position = self.last_safe_position;
            }
        }
    }

    pub fn handle_key_input(&mut self, key: VirtualKeyCode, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        match key {
            KeyCode::Tab if pressed => self.toggle_state(),
            _ => {}
        }
    }

    pub fn save_state(&self) -> PlayerState {
        self.state.clone()
    }

    pub fn load_state(&mut self, state: PlayerState) {
        self.state = state;
    }
}
