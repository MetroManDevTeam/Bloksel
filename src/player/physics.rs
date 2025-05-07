use crate::player::input::InputState;
use crate::utils::math::{AABB, Plane, ViewFrustum};
use crate::world::block_id::BlockData;
use crate::world::block_tech::BlockPhysics;
use crate::world::{Chunk, ChunkCoord, TerrainGenerator};
use glam::{Mat4, Quat, Vec2, Vec3};
use log::info;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;
use winit::event::{ElementState, MouseScrollDelta};
use winit::keyboard::KeyCode;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum PlayerState {
    Normal,
    Flying,
    Spectator,
    Walking,
    Sprinting,
    Crouching,
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

    pub physics: BlockPhysics,
    pub input: InputState,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 70.0, 0.0),
            velocity: Vec3::ZERO,
            rotation: Vec2::new(0.0, 0.0),
            size: Vec3::new(0.6, 1.8, 0.6),
            state: PlayerState::Walking,
            on_ground: false,
            base_speed: 5.0,
            speed_multiplier: 1.0,
            jump_force: 8.0,
            gravity: 20.0,
            sensitivity: 0.002,
            zoom_level: 1.0,
            max_zoom: 2.5,
            min_zoom: 0.4,
            chunk_size: 16, // Standard chunk size
            collision_enabled: true,
            last_safe_position: Vec3::new(0.0, 70.0, 0.0),
            physics: BlockPhysics::default(),
            input: InputState::default(),
        }
    }
}

impl Player {
    pub fn update(&mut self, dt: f32, terrain: &TerrainGenerator, input: &InputState) {
        self.handle_rotation(input);
        self.handle_movement(dt, input);
        self.handle_zoom(input);
        self.apply_physics(dt);
        self.update_position(dt, terrain);
        self.clamp_rotation();
        self.update_safe_position();
    }

    fn handle_rotation(&mut self, input: &InputState) {
        let mouse_delta = Vec2::new(input.mouse_delta.0, input.mouse_delta.1)
            * self.sensitivity
            * self.zoom_level;
        self.rotation.x = (self.rotation.x + mouse_delta.x).rem_euclid(2.0 * PI);
        self.rotation.y =
            (self.rotation.y + mouse_delta.y).clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
    }

    fn handle_movement(&mut self, dt: f32, input: &InputState) {
        match self.state {
            PlayerState::Walking => {
                let speed = self.base_speed * self.speed_multiplier;
                let mut velocity = Vec3::ZERO;

                if input.forward {
                    velocity.z -= speed;
                }
                if input.backward {
                    velocity.z += speed;
                }
                if input.left {
                    velocity.x -= speed;
                }
                if input.right {
                    velocity.x += speed;
                }

                self.velocity = velocity;
            }
            PlayerState::Crouching => {
                let speed = self.base_speed * self.speed_multiplier * 0.5;
                let mut velocity = Vec3::ZERO;

                if input.forward {
                    velocity.z -= speed;
                }
                if input.backward {
                    velocity.z += speed;
                }
                if input.left {
                    velocity.x -= speed;
                }
                if input.right {
                    velocity.x += speed;
                }

                self.velocity = velocity;
            }
            PlayerState::Flying => {
                let speed = self.base_speed * self.speed_multiplier * 2.0;
                let mut velocity = Vec3::ZERO;

                if input.forward {
                    velocity.z -= speed;
                }
                if input.backward {
                    velocity.z += speed;
                }
                if input.left {
                    velocity.x -= speed;
                }
                if input.right {
                    velocity.x += speed;
                }
                if input.jump {
                    velocity.y += speed;
                }
                if input.crouch {
                    velocity.y -= speed;
                }

                self.velocity = velocity;
            }
            PlayerState::Sprinting => {
                let speed = self.base_speed * self.speed_multiplier * 1.5;
                let mut velocity = Vec3::ZERO;

                if input.forward {
                    velocity.z -= speed;
                }
                if input.backward {
                    velocity.z += speed;
                }
                if input.left {
                    velocity.x -= speed;
                }
                if input.right {
                    velocity.x += speed;
                }

                self.velocity = velocity;
            }
            PlayerState::Normal => {
                // Ground-based movement with air control
                let acceleration = if self.on_ground { 15.0 } else { 3.0 };
                self.velocity.x += self.velocity.x * acceleration * dt;
                self.velocity.z += self.velocity.z * acceleration * dt;

                let friction = if self.on_ground { 0.7 } else { 0.98 };
                self.velocity.x *= friction;
                self.velocity.z *= friction;
            }
            PlayerState::Spectator => {
                // Instant velocity response with speed multiplier
                self.velocity = self.velocity.lerp(self.velocity, dt * 10.0);
            }
        }
    }

    fn calculate_movement_vector(&self, input: &InputState) -> Vec3 {
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

    fn calculate_current_speed(&self, input: &InputState) -> f32 {
        let base = match self.state {
            PlayerState::Spectator => self.base_speed * 3.0,
            PlayerState::Walking => self.base_speed,
            PlayerState::Sprinting => self.base_speed * 2.0,
            PlayerState::Crouching => self.base_speed,
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
            PlayerState::Walking => {
                self.velocity.y -= self.gravity * dt;
                self.velocity.y = self.velocity.y.clamp(-self.gravity, self.gravity);
            }
            PlayerState::Crouching => {
                self.velocity.y -= self.gravity * dt;
                self.velocity.y = self.velocity.y.clamp(-self.gravity, self.gravity);
            }
            PlayerState::Sprinting => {
                self.velocity.y -= self.gravity * dt;
                self.velocity.y = self.velocity.y.clamp(-self.gravity, self.gravity);
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
        let chunk_coord = ChunkCoord::from_world_pos(position, self.chunk_size);
        if let Some(chunk) = terrain.get_chunk(chunk_coord) {
            let local_pos = position - chunk_coord.to_world_pos(self.chunk_size);
            return chunk.is_solid_at(local_pos.x as i32, local_pos.y as i32, local_pos.z as i32);
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

    fn handle_zoom(&mut self, input: &InputState) {
        if self.state != PlayerState::Spectator {
            return;
        }

        let zoom_speed = 0.1;
        self.zoom_level =
            (self.zoom_level - input.mouse_scroll * zoom_speed).clamp(self.min_zoom, self.max_zoom);
    }

    pub fn get_view_matrix(&self) -> glam::Mat4 {
        let rotation_x = glam::Mat4::from_rotation_x(-self.rotation.y);
        let rotation_y = glam::Mat4::from_rotation_y(-self.rotation.x);
        let translation = glam::Mat4::from_translation(-self.position);

        // Apply zoom for spectator mode
        let zoom = glam::Mat4::from_scale(Vec3::splat(self.zoom_level));

        rotation_x * rotation_y * translation * zoom
    }

    pub fn toggle_state(&mut self) {
        self.state = match self.state {
            PlayerState::Normal => PlayerState::Flying,
            PlayerState::Flying => PlayerState::Spectator,
            PlayerState::Spectator => PlayerState::Normal,
            PlayerState::Walking => PlayerState::Walking,
            PlayerState::Sprinting => PlayerState::Walking,
            PlayerState::Crouching => PlayerState::Walking,
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
            PlayerState::Walking => {
                self.collision_enabled = true;
                self.position = self.last_safe_position;
            }
            PlayerState::Sprinting => {
                self.collision_enabled = true;
                self.position = self.last_safe_position;
            }
            PlayerState::Crouching => {
                self.collision_enabled = true;
                self.position = self.last_safe_position;
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyCode, pressed: bool) {
        match key {
            KeyCode::Space => {
                if pressed {
                    self.jump();
                }
            }
            KeyCode::ShiftLeft => {
                if pressed {
                    self.sprint();
                } else {
                    self.walk();
                }
            }
            KeyCode::ControlLeft => {
                if pressed {
                    self.crouch();
                } else {
                    self.stand();
                }
            }
            _ => {}
        }
    }

    pub fn save_state(&self) -> PlayerState {
        self.state.clone()
    }

    pub fn load_state(&mut self, state: PlayerState) {
        self.state = state;
    }

    pub fn jump(&mut self) {
        if self.state == PlayerState::Walking {
            self.velocity.y = 5.0;
        }
    }

    pub fn crouch(&mut self) {
        if self.state == PlayerState::Walking {
            self.state = PlayerState::Crouching;
        }
    }

    pub fn sprint(&mut self) {
        if self.state == PlayerState::Walking {
            self.state = PlayerState::Sprinting;
        }
    }

    pub fn stand(&mut self) {
        if self.state == PlayerState::Crouching {
            self.state = PlayerState::Walking;
        }
    }

    pub fn walk(&mut self) {
        if self.state == PlayerState::Sprinting {
            self.state = PlayerState::Walking;
        }
    }
}
