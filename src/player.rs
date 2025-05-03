use glam::{Vec3, Vec2};
use std::f32::consts::PI;
use winit::event::{ElementState, VirtualKeyCode};
use crate::terrain_generator::{ChunkCoord, BlockData, Chunk, TerrainGenerator};
use crate::chunk_renderer::ChunkRenderer;

#[derive(Debug)]
pub struct Player {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Vec2, // x = yaw, y = pitch
    pub size: Vec3,     // Collision box size
    pub on_ground: bool,
    pub flying: bool,
    pub speed: f32,
    pub jump_force: f32,
    pub gravity: f32,
    pub sensitivity: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 70.0, 0.0), // Start above ground
            velocity: Vec3::ZERO,
            rotation: Vec2::new(0.0, 0.0), // Looking forward
            size: Vec3::new(0.6, 1.8, 0.6), // Minecraft-like dimensions
            on_ground: false,
            flying: false,
            speed: 5.0,
            jump_force: 8.0,
            gravity: 20.0,
            sensitivity: 0.002,
        }
    }
}

impl Player {
    pub fn update(
        &mut self,
        dt: f32,
        terrain: &TerrainGenerator,
        input: &PlayerInput,
    ) {
        // Handle rotation (mouse look)
        self.rotation.x += input.mouse_delta.x * self.sensitivity;
        self.rotation.y = (self.rotation.y + input.mouse_delta.y * self.sensitivity)
            .clamp(-PI * 0.49, PI * 0.49); // Prevent over-rotation up/down

        // Calculate movement direction based on rotation
        let forward = Vec3::new(
            self.rotation.x.sin(),
            0.0,
            self.rotation.x.cos(),
        ).normalize();

        let right = Vec3::new(
            (self.rotation.x + PI/2.0).sin(),
            0.0,
            (self.rotation.x + PI/2.0).cos(),
        ).normalize();

        // Apply gravity if not flying
        if !self.flying {
            self.velocity.y -= self.gravity * dt;
        } else {
            self.velocity.y = 0.0;
        }

        // Handle jumping
        if input.jump && (self.on_ground || self.flying) {
            self.velocity.y = if self.flying { 0.0 } else { self.jump_force };
            self.on_ground = false;
        }

        // Handle sprinting
        let current_speed = if input.sprint {
            self.speed * 1.5
        } else {
            self.speed
        };

        // Calculate movement vector
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

        // Normalize and apply speed
        if move_vec.length_squared() > 0.0 {
            move_vec = move_vec.normalize() * current_speed;
        }

        // Apply movement to velocity
        if self.flying {
            // Flying movement (ignore ground friction)
            self.velocity.x = move_vec.x;
            self.velocity.z = move_vec.z;
            
            // Handle vertical flying movement
            if input.fly_up {
                self.velocity.y = current_speed;
            }
            if input.fly_down {
                self.velocity.y = -current_speed;
            }
        } else {
            // Ground movement with air control
            let acceleration = if self.on_ground { 15.0 } else { 3.0 };
            self.velocity.x += move_vec.x * acceleration * dt;
            self.velocity.z += move_vec.z * acceleration * dt;
            
            // Apply friction
            let friction = if self.on_ground { 0.7 } else { 0.98 };
            self.velocity.x *= friction;
            self.velocity.z *= friction;
        }

        // Move the player with collision detection
        self.move_with_collision(dt, terrain);
    }

    fn move_with_collision(&mut self, dt: f32, terrain: &TerrainGenerator) {
        let mut new_position = self.position + self.velocity * dt;

        // Check for collisions in each axis separately
        let mut collision_info = CollisionInfo::default();

        // X-axis collision
        let x_position = Vec3::new(new_position.x, self.position.y, self.position.z);
        self.check_collision(x_position, terrain, &mut collision_info);
        if collision_info.collided {
            new_position.x = self.position.x;
            self.velocity.x = 0.0;
        }

        // Y-axis collision
        let y_position = Vec3::new(self.position.x, new_position.y, self.position.z);
        collision_info.reset();
        self.check_collision(y_position, terrain, &mut collision_info);
        if collision_info.collided {
            new_position.y = if collision_info.from_below {
                self.position.y + 0.1 // Small step up
            } else {
                self.position.y
            };
            self.velocity.y = 0.0;
            self.on_ground = collision_info.from_below;
        } else {
            self.on_ground = false;
        }

        // Z-axis collision
        let z_position = Vec3::new(self.position.x, self.position.y, new_position.z);
        collision_info.reset();
        self.check_collision(z_position, terrain, &mut collision_info);
        if collision_info.collided {
            new_position.z = self.position.z;
            self.velocity.z = 0.0;
        }

        self.position = new_position;
    }

    fn check_collision(
        &self,
        position: Vec3,
        terrain: &TerrainGenerator,
        info: &mut CollisionInfo,
    ) {
        // Check blocks in the player's bounding box
        let min = position - self.size * 0.5;
        let max = position + self.size * 0.5;

        // Check blocks in the collision area
        for x in (min.x.floor() as i32)..=(max.x.ceil() as i32) {
            for y in (min.y.floor() as i32)..=(max.y.ceil() as i32) {
                for z in (min.z.floor() as i32)..=(max.z.ceil() as i32) {
                    // Get the chunk coordinates
                    let chunk_x = x.div_euclid(30);
                    let chunk_y = y.div_euclid(30);
                    let chunk_z = z.div_euclid(30);
                    
                    // Get local block coordinates
                    let local_x = x.rem_euclid(30) as usize;
                    let local_y = y.rem_euclid(30) as usize;
                    let local_z = z.rem_euclid(30) as usize;

                    // Check if the block is solid
                    if let Some(chunk) = terrain.get_chunk(ChunkCoord { x: chunk_x, y: chunk_y, z: chunk_z }) {
                        let chunk = chunk.read();
                        if let Some(block) = chunk.blocks[local_x][local_y][local_z] {
                            if block.physics == Physics::Steady || block.physics == Physics::Gravity {
                                info.collided = true;
                                
                                // Check if we're colliding from below
                                if position.y - self.size.y * 0.5 < y as f32 + 1.0 &&
                                   self.position.y - self.size.y * 0.5 >= y as f32 + 1.0 {
                                    info.from_below = true;
                                }
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn get_view_matrix(&self) -> [[f32; 4]; 4] {
        let rotation_x = glam::Mat4::from_rotation_x(-self.rotation.y);
        let rotation_y = glam::Mat4::from_rotation_y(-self.rotation.x);
        let translation = glam::Mat4::from_translation(-self.position);
        
        let view_matrix = rotation_x * rotation_y * translation;
        view_matrix.to_cols_array_2d()
    }

    pub fn toggle_flying(&mut self) {
        self.flying = !self.flying;
        if self.flying {
            self.velocity.y = 0.0;
        }
    }
}

#[derive(Default)]
struct CollisionInfo {
    collided: bool,
    from_below: bool,
}

impl CollisionInfo {
    fn reset(&mut self) {
        self.collided = false;
        self.from_below = false;
    }
}

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
}

impl Default for PlayerInput {
    fn default() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            jump: false,
            sprint: false,
            fly_up: false,
            fly_down: false,
            mouse_delta: Vec2::ZERO,
        }
    }
}

pub fn handle_key_input(input: &mut PlayerInput, key: VirtualKeyCode, state: ElementState) {
    let pressed = state == ElementState::Pressed;
    match key {
        VirtualKeyCode::W | VirtualKeyCode::Up => input.forward = pressed,
        VirtualKeyCode::S | VirtualKeyCode::Down => input.backward = pressed,
        VirtualKeyCode::A | VirtualKeyCode::Left => input.left = pressed,
        VirtualKeyCode::D | VirtualKeyCode::Right => input.right = pressed,
        VirtualKeyCode::Space => input.jump = pressed,
        VirtualKeyCode::LShift => input.sprint = pressed,
        VirtualKeyCode::Q => input.fly_down = pressed,
        VirtualKeyCode::E => input.fly_up = pressed,
        _ => (),
    }
}
