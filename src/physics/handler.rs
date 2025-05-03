use std::time::Duration;
use glam::{Vec3, Vec2, Quat};
use crate::core::{Block, BlockProperties};
use crate::physics::collision::{Collision, AABB};
use crate::core::world::World;
use crate::player::PlayerState;

const PLAYER_WIDTH: f32 = 0.6;
const PLAYER_HEIGHT: f32 = 1.8;
const EYE_HEIGHT: f32 = 1.62;
const STEP_HEIGHT: f32 = 0.6;
const SLIDE_THRESHOLD: f32 = 0.5;

pub struct PlayerPhysicsHandler {
    gravity: f32,
    terminal_velocity: f32,
    jump_force: f32,
    air_resistance: f32,
    ground_friction: f32,
    fly_speed: f32,
    last_position: Vec3,
    accumulated_time: f32,
}

impl Default for PlayerPhysicsHandler {
    fn default() -> Self {
        Self {
            gravity: 32.0, // blocks/sec² (matches Minecraft)
            terminal_velocity: 54.0, // blocks/sec
            jump_force: 8.0, // blocks/sec initial jump velocity
            air_resistance: 0.1, // per second
            ground_friction: 0.6, // per second
            fly_speed: 10.0, // blocks/sec
            last_position: Vec3::ZERO,
            accumulated_time: 0.0,
        }
    }
}

impl PlayerPhysicsHandler {
    pub fn update(
        &mut self,
        delta_time: Duration,
        player: &mut PlayerState,
        world: &World,
    ) {
        let dt = delta_time.as_secs_f32().min(0.1); // Cap delta time
        self.accumulated_time += dt;

        // Fixed timestep physics (60 updates/sec)
        while self.accumulated_time >= 1.0/60.0 {
            self.fixed_update(player, world, 1.0/60.0);
            self.accumulated_time -= 1.0/60.0;
        }

        // Handle remaining time
        if self.accumulated_time > 0.0 {
            self.fixed_update(player, world, self.accumulated_time);
            self.accumulated_time = 0.0;
        }
    }

    fn fixed_update(
        &mut self,
        player: &mut PlayerState,
        world: &World,
        delta_time: f32,
    ) {
        self.last_position = player.position;

        // Apply gravity if not flying
        if !player.is_flying {
            player.velocity.y -= self.gravity * delta_time;
            player.velocity.y = player.velocity.y.max(-self.terminal_velocity);
        }

        // Apply movement forces
        self.apply_movement(player, delta_time);

        // Check and resolve collisions
        self.resolve_collisions(player, world, delta_time);

        // Apply environmental effects
        self.apply_environment(player, world, delta_time);

        // Update player state
        player.on_ground = self.check_ground(player, world);
    }

    fn apply_movement(&self, player: &mut PlayerState, delta_time: f32) {
        let move_dir = self.get_move_direction(player);

        if player.is_flying {
            player.velocity = move_dir * self.fly_speed;
        } else {
            let speed = if player.is_sprinting { 5.6 } else { 4.3 };
            let acceleration = if player.on_ground { 20.0 } else { 5.0 };
            
            player.velocity.x += move_dir.x * acceleration * delta_time;
            player.velocity.z += move_dir.z * acceleration * delta_time;
            
            // Cap horizontal speed
            let horiz_speed = (player.velocity.x * player.velocity.x 
                + player.velocity.z * player.velocity.z).sqrt();
            if horiz_speed > speed {
                let ratio = speed / horiz_speed;
                player.velocity.x *= ratio;
                player.velocity.z *= ratio;
            }
        }
    }

    fn get_move_direction(&self, player: &PlayerState) -> Vec3 {
        let pitch = Quat::from_rotation_x(player.rotation.x);
        let yaw = Quat::from_rotation_y(player.rotation.y);
        let orientation = yaw * pitch;

        let mut direction = Vec3::ZERO;

        if player.input.forward {
            direction += orientation * -Vec3::Z;
        }
        if player.input.backward {
            direction += orientation * Vec3::Z;
        }
        if player.input.right {
            direction += orientation * Vec3::X;
        }
        if player.input.left {
            direction += orientation * -Vec3::X;
        }

        // Normalize and remove vertical component (unless flying)
        if player.is_flying {
            if player.input.up {
                direction.y += 1.0;
            }
            if player.input.down {
                direction.y -= 1.0;
            }
            direction.normalize_or_zero()
        } else {
            direction.y = 0.0;
            direction.normalize_or_zero()
        }
    }

    fn resolve_collisions(
        &mut self,
        player: &mut PlayerState,
        world: &World,
        delta_time: f32,
    ) {
        // Broad phase: Get nearby collision boxes
        let player_aabb = self.get_player_aabb(player.position);
        let nearby_blocks = world.get_collision_boxes(
            player.position,
            2, // chunk radius
        );

        // Narrow phase: Check against each block
        let mut resolved_pos = player.position;
        let mut remaining_velocity = player.velocity * delta_time;

        // Multiple iterations for corner cases
        for _ in 0..3 {
            if remaining_velocity.length_squared() < 0.0001 {
                break;
            }

            let new_aabb = AABB {
                min: player_aabb.min + remaining_velocity,
                max: player_aabb.max + remaining_velocity,
            };

            let mut closest_collision: Option<Collision> = None;

            for block_aabb in &nearby_blocks {
                if let Some(collision) = new_aabb.collide(block_aabb) {
                    if closest_collision.as_ref().map_or(true, |c| 
                        collision.depth > c.depth
                    ) {
                        closest_collision = Some(collision);
                    }
                }
            }

            if let Some(collision) = closest_collision {
                // Apply material effects
                self.apply_material_effects(player, &collision);

                // Slide along collision plane
                let slide_plane_normal = collision.normal;
                let velocity_along_normal = remaining_velocity.dot(slide_plane_normal);
                
                if velocity_along_normal < 0.0 {
                    remaining_velocity -= slide_plane_normal * velocity_along_normal;
                }

                // Small offset to prevent getting stuck
                resolved_pos += collision.normal * collision.depth * 1.01;
            } else {
                resolved_pos += remaining_velocity;
                remaining_velocity = Vec3::ZERO;
            }
        }

        player.position = resolved_pos;
        player.velocity = remaining_velocity / delta_time;
    }

    fn apply_material_effects(&self, player: &mut PlayerState, collision: &Collision) {
        // Handle bouncy blocks
        if let Some(bounce_factor) = collision.properties.bounce_factor {
            if collision.normal.y > 0.7 && player.velocity.y < 0.0 {
                player.velocity.y = -player.velocity.y * bounce_factor;
            }
        }

        // Handle slippery blocks
        if collision.properties.friction < 0.3 {
            player.velocity.x *= 1.1;
            player.velocity.z *= 1.1;
        }
    }

    fn check_ground(&self, player: &PlayerState, world: &World) -> bool {
        let feet_pos = player.position - Vec3::new(0.0, 0.05, 0.0);
        let feet_aabb = AABB {
            min: Vec3::new(
                feet_pos.x - PLAYER_WIDTH/2.0,
                feet_pos.y,
                feet_pos.z - PLAYER_WIDTH/2.0,
            ),
            max: Vec3::new(
                feet_pos.x + PLAYER_WIDTH/2.0,
                feet_pos.y + 0.1,
                feet_pos.z + PLAYER_WIDTH/2.0,
            ),
        };

        world.get_collision_boxes(player.position, 1)
            .iter()
            .any(|block| feet_aabb.collide(block).is_some())
    }

    fn apply_environment(&self, player: &mut PlayerState, world: &World, delta_time: f32) {
        // Check if in water
        let head_pos = player.position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
        if let Some(block) = world.get_block(
            head_pos.x.floor() as i32,
            head_pos.y.floor() as i32,
            head_pos.z.floor() as i32,
        ) {
            if block.is_liquid() {
                player.velocity *= 0.8; // Water resistance
                player.velocity.y = player.velocity.y.max(-2.0); // Terminal velocity in water
            }
        }

        // Apply friction
        if player.on_ground {
            player.velocity.x *= (1.0 - self.ground_friction * delta_time).max(0.0);
            player.velocity.z *= (1.0 - self.ground_friction * delta_time).max(0.0);
        } else {
            player.velocity.x *= (1.0 - self.air_resistance * delta_time).max(0.0);
            player.velocity.z *= (1.0 - self.air_resistance * delta_time).max(0.0);
        }
    }

    fn get_player_aabb(&self, position: Vec3) -> AABB {
        AABB {
            min: Vec3::new(
                position.x - PLAYER_WIDTH/2.0,
                position.y,
                position.z - PLAYER_WIDTH/2.0,
            ),
            max: Vec3::new(
                position.x + PLAYER_WIDTH/2.0,
                position.y + PLAYER_HEIGHT,
                position.z + PLAYER_WIDTH/2.0,
            ),
        }
    }

    pub fn attempt_jump(&self, player: &mut PlayerState, world: &World) {
        if player.is_flying {
            player.velocity.y = self.jump_force;
        } else if self.check_ground(player, world) {
            player.velocity.y = self.jump_force;
            player.on_ground = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::world::World;
    use crate::player::PlayerInput;

    fn test_world() -> World {
        World::new(12345, 10, 100)
    }

    fn test_player() -> PlayerState {
        PlayerState {
            position: Vec3::new(0.0, 70.0, 0.0),
            velocity: Vec3::ZERO,
            rotation: Vec2::ZERO,
            input: PlayerInput::default(),
            on_ground: false,
            is_flying: false,
            is_sprinting: false,
        }
    }

    #[test]
    fn test_gravity_application() {
        let mut physics = PlayerPhysicsHandler::default();
        let mut player = test_player();
        let world = test_world();

        physics.update(Duration::from_secs_f32(0.5), &mut player, &world);
        assert!(player.velocity.y < 0.0, "Player should be falling");
    }

    #[test]
    fn test_ground_collision() {
        let mut physics = PlayerPhysicsHandler::default();
        let mut player = test_player();
        let mut world = test_world();

        // Place a block below player
        world.set_block(0, 69, 0, Block::test_block()).unwrap();

        physics.update(Duration::from_secs_f32(1.0), &mut player, &world);
        assert!(player.on_ground, "Player should be on ground");
        assert!(player.position.y >= 70.0, "Player should not sink into block");
    }
}
