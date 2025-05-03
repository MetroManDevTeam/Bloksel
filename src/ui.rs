// ui.rs - Menu and World Management UI

use eframe::egui;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use crate::engine::{EngineConfig, VoxelEngine};

#[derive(Default, Serialize, Deserialize)]
pub struct WorldMeta {
    pub name: String,
    pub seed: u64,
    pub last_played: chrono::DateTime<chrono::Local>,
    pub play_time: f32,
    pub version: String,
    pub preview_image: Option<Vec<u8>>,
}

pub struct MenuState {
    current_screen: MenuScreen,
    worlds: Vec<WorldMeta>,
    create_state: CreateWorldState,
    selected_world: Option<usize>,
}

#[derive(PartialEq)]
enum MenuScreen {
    Main,
    Create,
    Worlds,
    Loading,
}

struct CreateWorldState {
    name: String,
    seed: String,
    world_type: WorldType,
    difficulty: Difficulty,
    bonus_chest: bool,
    generate_structures: bool,
}

#[derive(PartialEq)]
enum WorldType {
    Default,
    Flat,
    Amplified,
    LargeBiomes,
}

#[derive(PartialEq)]
enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            current_screen: MenuScreen::Main,
            worlds: load_saved_worlds(),
            create_state: CreateWorldState {
                name: String::new(),
                seed: String::new(),
                world_type: WorldType::Default,
                difficulty: Difficulty::Normal,
                bonus_chest: false,
                generate_structures: true,
            },
            selected_world: None,
        }
    }
}

impl MenuState {
    pub fn show(&mut self, ctx: &egui::Context, engine: &mut VoxelEngine) {
        match self.current_screen {
            MenuScreen::Main => self.main_menu(ctx),
            MenuScreen::Create => self.create_world(ctx),
            MenuScreen::Worlds => self.worlds_list(ctx),
            MenuScreen::Loading => self.loading_screen(ctx),
        }
        
        self.handle_transitions(engine);
    }

    fn main_menu(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                logo(ui);
                ui.add_space(30.0);
                
                if button(ui, "Create New World").clicked() {
                    self.current_screen = MenuScreen::Create;
                }
                
                if button(ui, "Load World").clicked() {
                    self.current_screen = MenuScreen::Worlds;
                }
                
                if button(ui, "Settings").clicked() {
                    // Open settings window
                }
                
                if button(ui, "Quit Game").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }

    fn create_world(&mut self, ctx: &egui::Context) {
        egui::Window::new("Create New World")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("World Name:");
                    ui.text_edit_singleline(&mut self.create_state.name);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Seed:");
                    ui.text_edit_singleline(&mut self.create_state.seed);
                });
                
                egui::Grid::new("world_settings")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("World Type:");
                        egui::ComboBox::new("world_type", "")
                            .selected_text(format!("{:?}", self.create_state.world_type))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.create_state.world_type, WorldType::Default, "Default");
                                ui.selectable_value(&mut self.create_state.world_type, WorldType::Flat, "Flat");
                                ui.selectable_value(&mut self.create_state.world_type, WorldType::Amplified, "Amplified");
                                ui.selectable_value(&mut self.create_state.world_type, WorldType::LargeBiomes, "Large Biomes");
                            });
                        ui.end_row();

                        ui.label("Difficulty:");
                        egui::ComboBox::new("difficulty", "")
                            .selected_text(format!("{:?}", self.create_state.difficulty))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.create_state.difficulty, Difficulty::Peaceful, "Peaceful");
                                ui.selectable_value(&mut self.create_state.difficulty, Difficulty::Easy, "Easy");
                                ui.selectable_value(&mut self.create_state.difficulty, Difficulty::Normal, "Normal");
                                ui.selectable_value(&mut self.create_state.difficulty, Difficulty::Hard, "Hard");
                            });
                        ui.end_row();

                        ui.label("Options:");
                        ui.checkbox(&mut self.create_state.bonus_chest, "Bonus Chest");
                        ui.checkbox(&mut self.create_state.generate_structures, "Generate Structures");
                        ui.end_row();
                    });

                ui.horizontal(|ui| {
                    if button(ui, "Cancel").clicked() {
                        self.current_screen = MenuScreen::Main;
                    }
                    
                    if button(ui, "Create World").clicked() {
                        self.current_screen = MenuScreen::Loading;
                    }
                });
            });
    }

    fn worlds_list(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Select World");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, world) in self.worlds.iter().enumerate() {
                    let selected = self.selected_world == Some(idx);
                    ui.horizontal(|ui| {
                        // World preview image
                        if let Some(preview) = &world.preview_image {
                            // Display image texture
                        } else {
                            ui.label("📁");
                        }
                        
                        // World info
                        ui.vertical(|ui| {
                            ui.heading(&world.name);
                            ui.label(format!("Last played: {}", world.last_played));
                            ui.label(format!("Play time: {} hours", world.play_time));
                        });
                        
                        // Selection indicator
                        if selected {
                            ui.label("✔");
                        }
                    }).clicked().then(|| {
                        self.selected_world = Some(idx);
                    });
                }
            });
            
            ui.separator();
            
            ui.horizontal(|ui| {
                if button(ui, "Back").clicked() {
                    self.current_screen = MenuScreen::Main;
                }
                
                if button(ui, "Play").clicked() {
                    if let Some(idx) = self.selected_world {
                        self.current_screen = MenuScreen::Loading;
                    }
                }
                
                if button(ui, "Delete").clicked() {
                    if let Some(idx) = self.selected_world {
                        delete_world(&self.worlds[idx].name);
                        self.worlds.remove(idx);
                        self.selected_world = None;
                    }
                }
            });
        });
    }

    fn loading_screen(&self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.label("Generating world...");
                ui.add(egui::Spinner::new().size(50.0));
                ui.label("This may take a few minutes");
            });
        });
    }

    fn handle_transitions(&mut self, engine: &mut VoxelEngine) {
        if let MenuScreen::Loading = self.current_screen {
            if let Some(world_name) = self.get_pending_world() {
                let config = EngineConfig {
                    world_seed: self.create_state.seed.parse().unwrap_or(0),
                    // ... other config ...
                };
                
                engine.create_world(config);
                self.current_screen = MenuScreen::Main;
            }
        }
    }
    
    fn get_pending_world(&self) -> Option<String> {
        match self.current_screen {
            MenuScreen::Loading => Some(self.create_state.name.clone()),
            _ => None,
        }
    }
}

// Helper components
fn button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized(
        [200.0, 40.0], 
        egui::Button::new(text).fill(egui::Color32::from_rgb(40, 40, 40))
}

fn logo(ui: &mut egui::Ui) {
    ui.heading("VOXEL ENGINE");
    ui.add_space(10.0);
    ui.label("Version 1.0.0");
}

// World management
fn load_saved_worlds() -> Vec<WorldMeta> {
    let saves_dir = Path::new("saves");
    let mut worlds = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(saves_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = std::fs::read_to_string(entry.path().join("world.json")) {
                if let Ok(world) = serde_json::from_str(&meta) {
                    worlds.push(world);
                }
            }
        }
    }
    
    worlds
}

fn delete_world(name: &str) {
    let path = Path::new("saves").join(name);
    let _ = std::fs::remove_dir_all(path);
}
