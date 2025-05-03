use glam::{Vec3, Vec2};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Vec2, // pitch, yaw
    pub health: f32,
    pub hunger: f32,
    pub inventory: Inventory,
    pub selected_slot: usize,
    pub game_mode: GameMode,
    pub is_sprinting: bool,
    pub is_crouching: bool,
    pub is_flying: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: [Option<ItemStack>; 36],
    pub hotbar: [Option<ItemStack>; 9],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 70.0, 0.0),
            velocity: Vec3::ZERO,
            rotation: Vec2::ZERO,
            health: 20.0,
            hunger: 20.0,
            inventory: Inventory::default(),
            selected_slot: 0,
            game_mode: GameMode::Survival,
            is_sprinting: false,
            is_crouching: false,
            is_flying: false,
        }
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: [None; 36],
            hotbar: [None; 9],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_id: String,
    pub count: u32,
    pub durability: u32,
}
