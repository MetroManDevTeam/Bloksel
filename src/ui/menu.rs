use crate::{VoxelEngine, world::WorldMeta};
use egui::{CentralPanel, ComboBox, Context, Grid, Spinner, Window, Align, Layout, Rect, Vec2, Ui};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuScreen {
    Main,
    LoadWorld,
    CreateWorld,
    Settings,
    Credits,
    Loading,
}

#[derive(Debug)]
pub struct MenuState {
    current_screen: MenuScreen,
    create_world_state: CreateWorldState,
    selected_world: Option<WorldMeta>,
    worlds_list: Vec<WorldMeta>, // Store discovered worlds
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            current_screen: MenuScreen::Main,
            create_world_state: CreateWorldState::default(),
            selected_world: None,
            worlds_list: Vec::new(), // Will be populated when worlds are discovered
        }
    }

    pub fn show(&mut self, ctx: &Context, engine: &mut VoxelEngine) {
        match self.current_screen {
            MenuScreen::Main => self.show_main_menu(ctx),
            MenuScreen::LoadWorld => self.show_load_world(ctx),
            MenuScreen::CreateWorld => self.show_create_world(ctx),
            MenuScreen::Settings => self.show_settings(ctx),
            MenuScreen::Credits => self.show_credits(ctx),
            MenuScreen::Loading => self.show_loading_screen(ctx),
        }

        self.handle_transitions(engine);
    }

    fn show_main_menu(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            // Full screen layout with centered content
            let available_size = ui.available_size();
            
            // Title at the top (centered)
            ui.vertical_centered(|ui| {
                ui.add_space(available_size.y * 0.1); // Top spacing
                ui.heading("Bloksel");
                ui.add_space(available_size.y * 0.15); // Space after title
            });
            
            // Center buttons
            ui.vertical_centered(|ui| {
                let button_width = available_size.x * 0.3;
                
                ui.set_width(button_width);
                if ui.button("Load World").clicked() {
                    self.current_screen = MenuScreen::LoadWorld;
                }
                
                ui.add_space(20.0);
                
                if ui.button("Create World").clicked() {
                    self.current_screen = MenuScreen::CreateWorld;
                }
            });
            
            // Space to push bottom buttons
            ui.add_space(available_size.y * 0.3);
            
            // Bottom row with Settings and Credits buttons
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                ui.add_space(20.0); // Bottom margin
                
                // Create a horizontal layout for the bottom buttons
                ui.horizontal(|ui| {
                    // Forces left alignment for Settings button
                    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                        if ui.button("Settings").clicked() {
                            self.current_screen = MenuScreen::Settings;
                        }
                    });
                    
                    // Add expanding space between buttons
                    ui.add_space(available_size.x * 0.6);
                    
                    // Forces right alignment for Credits button
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if ui.button("Credits").clicked() {
                            self.current_screen = MenuScreen::Credits;
                        }
                    });
                });
                
                // Copyright text at the very bottom
                ui.add_space(10.0);
                ui.label("MetroManDevTeam 2025");
            });
        });
    }

    fn show_load_world(&mut self, ctx: &Context) {
        Window::new("Load World")
            .collapsible(false)
            .resizable(true)
            .default_size([400.0, 500.0])
            .show(ctx, |ui| {
                ui.heading("Select a World");
                ui.add_space(10.0);
                
                // World listing area with scrolling
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.worlds_list.is_empty() {
                        ui.label("No worlds found. Create a new world!");
                    } else {
                        for world in &self.worlds_list {
                            let world_name = &world.name;
                            if ui.selectable_label(
                                self.selected_world.as_ref().map_or(false, |w| &w.name == world_name),
                                format!("{} (Created: {})", world_name, "Unknown")
                            ).clicked() {
                                self.selected_world = Some(world.clone());
                            }
                        }
                    }
                });
                
                ui.add_space(20.0);
                
                ui.horizontal(|ui| {
                    let button_width = ui.available_width() / 3.0 - 10.0;
                    
                    if ui.add_sized([button_width, 30.0], egui::Button::new("Play Selected")).clicked() {
                        if self.selected_world.is_some() {
                            self.current_screen = MenuScreen::Loading;
                        }
                    }
                    
                    ui.add_space(10.0);
                    
                    if ui.add_sized([button_width, 30.0], egui::Button::new("Delete")).clicked() {
                        // TODO: Implement world deletion with confirmation dialog
                    }
                    
                    ui.add_space(10.0);
                    
                    if ui.add_sized([button_width, 30.0], egui::Button::new("Back")).clicked() {
                        self.current_screen = MenuScreen::Main;
                    }
                });
            });
    }

    fn show_create_world(&mut self, ctx: &Context) {
        Window::new("Create New World")
            .collapsible(false)
            .resizable(false)
            .default_size([450.0, 350.0])
            .show(ctx, |ui| {
                Grid::new("world_settings")
                    .num_columns(2)
                    .spacing([40.0, 15.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("World Name:");
                        ui.text_edit_singleline(&mut self.create_world_state.name)
                            .on_hover_text("Enter a unique name for your world");
                        ui.end_row();

                        ui.label("Seed (Optional):");
                        ui.text_edit_singleline(&mut self.create_world_state.seed)
                            .on_hover_text("Leave blank for random seed");
                        ui.end_row();

                        ui.label("World Type:");
                        ComboBox::new("world_type", "")
                            .selected_text(format!("{:?}", self.create_world_state.world_type))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.create_world_state.world_type,
                                    WorldType::Normal,
                                    "Normal",
                                );
                                ui.selectable_value(
                                    &mut self.create_world_state.world_type,
                                    WorldType::Superflat,
                                    "Superflat",
                                );
                                ui.selectable_value(
                                    &mut self.create_world_state.world_type,
                                    WorldType::Void,
                                    "Void",
                                );
                            });
                        ui.end_row();

                        ui.label("Difficulty:");
                        ComboBox::new("difficulty", "")
                            .selected_text(format!("{:?}", self.create_world_state.difficulty))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.create_world_state.difficulty,
                                    Difficulty::Peaceful,
                                    "Peaceful",
                                );
                                ui.selectable_value(
                                    &mut self.create_world_state.difficulty,
                                    Difficulty::Easy,
                                    "Easy",
                                );
                                ui.selectable_value(
                                    &mut self.create_world_state.difficulty,
                                    Difficulty::Normal,
                                    "Normal",
                                );
                                ui.selectable_value(
                                    &mut self.create_world_state.difficulty,
                                    Difficulty::Hard,
                                    "Hard",
                                );
                            });
                        ui.end_row();
                        
                        ui.label("Game Mode:");
                        ComboBox::new("game_mode", "")
                            .selected_text(format!("{:?}", self.create_world_state.game_mode))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.create_world_state.game_mode,
                                    GameMode::Survival,
                                    "Survival",
                                );
                                ui.selectable_value(
                                    &mut self.create_world_state.game_mode,
                                    GameMode::Creative,
                                    "Creative",
                                );
                                ui.selectable_value(
                                    &mut self.create_world_state.game_mode, 
                                    GameMode::Adventure,
                                    "Adventure",
                                );
                            });
                        ui.end_row();
                    });

                ui.add_space(30.0);

                // Validation warning if name is empty
                if self.create_world_state.name.trim().is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), 
                        "⚠ World name cannot be empty");
                    ui.add_space(10.0);
                }

                ui.horizontal(|ui| {
                    let button_width = ui.available_width() / 2.0 - 5.0;
                    
                    if ui.add_sized([button_width, 30.0], egui::Button::new("Create"))
                        .clicked() && !self.create_world_state.name.trim().is_empty() {
                        self.current_screen = MenuScreen::Loading;
                    }
                    
                    ui.add_space(10.0);
                    
                    if ui.add_sized([button_width, 30.0], egui::Button::new("Cancel")).clicked() {
                        self.current_screen = MenuScreen::Main;
                    }
                });
            });
    }

    fn show_settings(&mut self, ctx: &Context) {
        Window::new("Settings")
            .collapsible(false)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.heading("Game Settings");
                ui.add_space(20.0);
                
                // Settings tabs
                ui.horizontal(|ui| {
                    ui.selectable_label(true, "General");
                    ui.selectable_label(false, "Graphics");
                    ui.selectable_label(false, "Sound");
                    ui.selectable_label(false, "Controls");
                });
                
                ui.separator();
                ui.add_space(10.0);
                
                // Example settings for the selected tab
                Grid::new("settings_grid")
                    .num_columns(2)
                    .spacing([40.0, 10.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Graphics settings examples
                        ui.label("Render Distance:");
                        ui.add(egui::Slider::new(&mut 12, 2..=32).suffix(" chunks"));
                        ui.end_row();
                        
                        ui.label("FPS Limit:");
                        ui.add(egui::Slider::new(&mut 60, 30..=240).suffix(" fps"));
                        ui.end_row();
                        
                        ui.label("Fullscreen:");
                        ui.checkbox(&mut true, "");
                        ui.end_row();
                        
                        ui.label("VSync:");
                        ui.checkbox(&mut true, "");
                        ui.end_row();
                        
                        ui.label("FOV:");
                        ui.add(egui::Slider::new(&mut 70, 30..=110).suffix("°"));
                        ui.end_row();
                    });
                
                ui.add_space(20.0);
                
                // Bottom buttons
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            // Save settings
                            self.current_screen = MenuScreen::Main;
                        }
                        
                        if ui.button("Cancel").clicked() {
                            self.current_screen = MenuScreen::Main;
                        }
                        
                        if ui.button("Defaults").clicked() {
                            // Reset to defaults
                        }
                    });
                });
            });
    }

    fn show_credits(&mut self, ctx: &Context) {
        Window::new("Credits")
            .collapsible(false)
            .resizable(false)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Bloksel");
                    ui.add_space(10.0);
                    ui.label("Version 0.1.0");
                    ui.add_space(20.0);
                    
                    ui.strong("Development Team");
                    ui.label("MetroManDevTeam");
                    ui.add_space(20.0);
                    
                    ui.strong("Engine Programming");
                    ui.label("Lead Developer");
                    ui.label("Graphics Engineer");
                    ui.add_space(10.0);
                    
                    ui.strong("Game Design");
                    ui.label("Game Designer");
                    ui.label("Level Designer");
                    ui.add_space(10.0);
                    
                    ui.strong("Art & Assets");
                    ui.label("Art Director");
                    ui.label("3D Artist");
                    ui.add_space(10.0);
                    
                    ui.strong("Sound & Music");
                    ui.label("Sound Designer");
                    ui.label("Composer");
                    ui.add_space(20.0);
                    
                    ui.strong("Special Thanks");
                    ui.label("The Rust Community");
                    ui.label("egui Team");
                    ui.add_space(30.0);
                    
                    ui.label("Copyright © MetroManDevTeam 2025");
                    ui.label("All Rights Reserved");
                    
                    ui.add_space(20.0);
                    if ui.button("Back").clicked() {
                        self.current_screen = MenuScreen::Main;
                    }
                });
            });
    }

    fn show_loading_screen(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let available_size = ui.available_size();
                
                ui.add_space(available_size.y * 0.3);
                ui.heading("Loading World...");
                ui.add_space(20.0);
                ui.add(Spinner::new().size(50.0));
                
                // Loading progress bar
                ui.add_space(30.0);
                let progress = (ctx.frame_nr() as f32 % 100.0) / 100.0; // Simulate progress
                ui.add(egui::ProgressBar::new(progress)
                    .show_percentage()
                    .animate(true));
                    
                // Loading task description
                ui.add_space(10.0);
                let tasks = ["Generating terrain", "Loading chunks", "Spawning entities", "Preparing world"];
                let current_task = tasks[(ctx.frame_nr() / 50) % tasks.len()];
                ui.label(current_task);
            });
        });
    }

    fn handle_transitions(&mut self, engine: &mut VoxelEngine) {
        match self.current_screen {
            MenuScreen::Loading => {
                // Handle loading based on context (new world or loading existing)
                if let Some(selected_world) = &self.selected_world {
                    // Load existing world
                    engine.load_world(&PathBuf::from(format!("worlds/{}", selected_world.name)));
                } else {
                    // Create new world with proper error handling for the seed
                    let seed = match self.create_world_state.seed.parse::<u64>() {
                        Ok(s) => s, 
                        Err(_) => {
                            // Use a hash of the world name if seed is invalid
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};
                            
                            let mut hasher = DefaultHasher::new();
                            self.create_world_state.name.hash(&mut hasher);
                            hasher.finish()
                        }
                    };
                    
                    let config = engine.create_world_config(
                        self.create_world_state.name.clone(),
                        seed,
                    );
                    
                    // Additional world parameters would be set here
                    
                    engine.load_world(&PathBuf::from(format!("worlds/{}", self.create_world_state.name)));
                }
                
                // Simulating loading time - in a real implementation you'd check if loading is complete
                // and then transition to the game state
                self.current_screen = MenuScreen::Main; // Change this to actual game state
            }
            _ => {}
        }
    }
    
    // Method to scan for existing worlds
    pub fn scan_for_worlds(&mut self) {
        // In a real implementation, this would scan the worlds directory
        // and populate self.worlds_list with WorldMeta objects
        
        // Example placeholder implementation:
        self.worlds_list = vec![
            WorldMeta { 
                name: "Test World".to_string(),
                // other fields would be here
            },
            WorldMeta {
                name: "Creative Build".to_string(),
                // other fields would be here
            }
        ];
    }
}

#[derive(Debug, Default)]
pub struct CreateWorldState {
    pub name: String,
    pub world_type: WorldType,
    pub difficulty: Difficulty,
    pub game_mode: GameMode,
    pub seed: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorldType {
    Normal,
    Superflat,
    Void,
}

impl Default for WorldType {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Default for Difficulty {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
}

impl Default for GameMode {
    fn default() -> Self {
        Self::Survival
    }
}

// For demonstration purposes, let's assume WorldMeta has this structure
// In a real implementation, this would be imported from the world module
impl WorldMeta {
    // Mock implementation for the world metadata
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

// Mock implementation for display purposes
impl WorldMeta {
    pub fn name(&self) -> &str {
        &self.name
    }
}

// Adding this field to WorldMeta to match the code
#[derive(Debug, Clone)]
pub struct WorldMeta {
    pub name: String,
    // Other fields would be here in a real implementation
                                    }
