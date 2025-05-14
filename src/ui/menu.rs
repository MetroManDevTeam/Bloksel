use crate::{
    render::vulkan::VulkanContext,
    ui::{egui_render::EguiRenderer, helpers},
    VoxelEngine, 
    world::WorldMeta,
    config::{core::EngineConfig, worldgen::WorldGenConfig}
};
use ash::vk;
use egui::{
    CentralPanel, ComboBox, Context, Grid, Spinner, Window, 
    Align, Layout, Rect, Vec2, Ui, ClippedPrimitive, TexturesDelta,
    Color32, ProgressBar, Label, ScrollArea, SelectableLabel
};
use egui_winit::State as EguiWinitState;
use std::{path::PathBuf, sync::Arc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuScreen {
    Main,
    LoadWorld,
    CreateWorld,
    Settings,
    Credits,
    Loading,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Default)]
pub struct CreateWorldState {
    pub name: String,
    pub world_type: WorldType,
    pub difficulty: Difficulty,
    pub game_mode: GameMode,
    pub seed: String,
}

#[derive(Debug)]
pub struct MenuState {
    current_screen: MenuScreen,
    create_world_state: CreateWorldState,
    selected_world: Option<WorldMeta>,
    worlds_list: Vec<WorldMeta>,
    egui_renderer: Option<Arc<EguiRenderer>>,
    egui_context: Context,
    egui_winit_state: EguiWinitState,
}

impl MenuState {
    pub fn new(vulkan_context: Arc<VulkanContext>, render_pass: vk::RenderPass) -> Self {
        let egui_context = Context::default();
        let egui_winit_state = EguiWinitState::new(1280, 720, 1.0);
        
        let egui_renderer = EguiRenderer::new(&vulkan_context, render_pass)
            .expect("Failed to create egui renderer");

        Self {
            current_screen: MenuScreen::Main,
            create_world_state: CreateWorldState::default(),
            selected_world: None,
            worlds_list: Vec::new(),
            egui_renderer: Some(Arc::new(egui_renderer)),
            egui_context,
            egui_winit_state,
        }
    }

    pub fn dummy() -> Self {
        Self {
            current_screen: MenuScreen::Main,
            create_world_state: CreateWorldState::default(),
            selected_world: None,
            worlds_list: Vec::new(),
            egui_renderer: None,
            egui_context: Context::default(),
            egui_winit_state: EguiWinitState::new(0, 0, 1.0),
        }
    }

    pub fn handle_event(&mut self, event: &winit::event::WindowEvent<'_>) -> bool {
        self.egui_winit_state.on_event(&self.egui_context, event).consumed
    }

    pub fn update(&mut self, window: &winit::window::Window) {
        let raw_input = self.egui_winit_state.take_egui_input(window);
        self.egui_context.begin_frame(raw_input);
    }

    pub fn render(
        &mut self,
        vulkan_context: &Arc<VulkanContext>,
        command_buffer: vk::CommandBuffer,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.egui_winit_state.set_pixels_per_point(1.0);

        if self.egui_renderer.is_none() {
            self.show(&self.egui_context, &mut VoxelEngine::dummy());
        }

        let full_output = self.egui_context.end_frame();
        let clipped_primitives = self.egui_context.tessellate(full_output.shapes);
        let textures_delta = full_output.textures_delta;

        if let Some(renderer) = &self.egui_renderer {
            renderer.render(
                vulkan_context,
                command_buffer,
                &clipped_primitives,
                &textures_delta,
                viewport_width,
                viewport_height,
            )?;
        }

        Ok(())
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
            let available_size = ui.available_size();
            
            helpers::logo(ui);
            ui.add_space(available_size.y * 0.15);
            
            ui.vertical_centered(|ui| {
                ui.set_width(available_size.x * 0.3);
                
                if helpers::button(ui, "Load World").clicked() {
                    self.current_screen = MenuScreen::LoadWorld;
                    self.scan_for_worlds();
                }
                
                ui.add_space(20.0);
                
                if helpers::button(ui, "Create World").clicked() {
                    self.current_screen = MenuScreen::CreateWorld;
                }
            });
            
            ui.add_space(available_size.y * 0.3);
            
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                ui.add_space(20.0);
                
                ui.horizontal(|ui| {
                    if helpers::small_button(ui, "Settings").clicked() {
                        self.current_screen = MenuScreen::Settings;
                    }
                    
                    ui.add_space(available_size.x * 0.6);
                    
                    if helpers::small_button(ui, "Credits").clicked() {
                        self.current_screen = MenuScreen::Credits;
                    }
                });
                
                ui.add_space(10.0);
                ui.label("MetroManDevTeam 2025");
            });
        });
    }

    fn show_load_world(&mut self, ctx: &Context) {
        helpers::standard_window(ctx, "Load World")
            .default_size([400.0, 500.0])
            .show(ctx, |ui| {
                ui.heading("Select a World");
                ui.add_space(10.0);
                
                ScrollArea::vertical().show(ui, |ui| {
                    if self.worlds_list.is_empty() {
                        ui.label("No worlds found. Create a new world!");
                    } else {
                        for world in &self.worlds_list {
                            let is_selected = self.selected_world.as_ref()
                                .map_or(false, |w| w.name == world.name);
                            
                            if SelectableLabel::new(is_selected, &world.name).ui(ui).clicked() {
                                self.selected_world = Some(world.clone());
                            }
                        }
                    }
                });
                
                ui.add_space(20.0);
                
                ui.horizontal(|ui| {
                    let btn_width = ui.available_width() / 3.0 - 10.0;
                    
                    if ui.add_sized([btn_width, 30.0], helpers::small_button(ui, "Play Selected")).clicked() 
                        && self.selected_world.is_some() {
                        self.current_screen = MenuScreen::Loading;
                    }
                    
                    ui.add_space(10.0);
                    
                    if ui.add_sized([btn_width, 30.0], helpers::small_button(ui, "Delete")).clicked() {
                        if let Some(world) = &self.selected_world {
                            helpers::delete_world(&world.name);
                            self.scan_for_worlds();
                        }
                    }
                    
                    ui.add_space(10.0);
                    
                    if ui.add_sized([btn_width, 30.0], helpers::small_button(ui, "Back")).clicked() {
                        self.current_screen = MenuScreen::Main;
                    }
                });
            });
    }

    fn show_create_world(&mut self, ctx: &Context) {
        helpers::standard_window(ctx, "Create New World")
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

                if self.create_world_state.name.trim().is_empty() {
                    ui.colored_label(Color32::from_rgb(255, 100, 100), 
                        "⚠ World name cannot be empty");
                    ui.add_space(10.0);
                }

                ui.horizontal(|ui| {
                    let button_width = ui.available_width() / 2.0 - 5.0;
                    
                    if ui.add_sized([button_width, 30.0], helpers::small_button(ui, "Create"))
                        .clicked() && !self.create_world_state.name.trim().is_empty() {
                        self.current_screen = MenuScreen::Loading;
                    }
                    
                    ui.add_space(10.0);
                    
                    if ui.add_sized([button_width, 30.0], helpers::small_button(ui, "Cancel")).clicked() {
                        self.current_screen = MenuScreen::Main;
                    }
                });
            });
    }

    fn show_settings(&mut self, ctx: &Context) {
        helpers::standard_window(ctx, "Settings")
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.heading("Game Settings");
                ui.add_space(20.0);
                
                ui.horizontal(|ui| {
                    ui.selectable_label(true, "General");
                    ui.selectable_label(false, "Graphics");
                    ui.selectable_label(false, "Sound");
                    ui.selectable_label(false, "Controls");
                });
                
                ui.separator();
                ui.add_space(10.0);
                
                Grid::new("settings_grid")
                    .num_columns(2)
                    .spacing([40.0, 10.0])
                    .striped(true)
                    .show(ui, |ui| {
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
                
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        if helpers::small_button(ui, "Save").clicked() {
                            self.current_screen = MenuScreen::Main;
                        }
                        
                        if helpers::small_button(ui, "Cancel").clicked() {
                            self.current_screen = MenuScreen::Main;
                        }
                        
                        if helpers::small_button(ui, "Defaults").clicked() {
                            // Reset to defaults
                        }
                    });
                });
            });
    }

    fn show_credits(&mut self, ctx: &Context) {
        helpers::standard_window(ctx, "Credits")
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    helpers::logo(ui);
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
                    if helpers::small_button(ui, "Back").clicked() {
                        self.current_screen = MenuScreen::Main;
                    }
                });
            });
    }

    fn show_loading_screen(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            let progress = (ctx.frame_nr() as f32 % 100.0) / 100.0;
            let tasks = ["Generating terrain", "Loading chunks", "Spawning entities", "Preparing world"];
            let current_task = tasks[(ctx.frame_nr() as usize / 50) % tasks.len()];
            
            helpers::loading_spinner(ui, current_task, progress);
        });
    }

    fn handle_transitions(&mut self, engine: &mut VoxelEngine) {
        match self.current_screen {
            MenuScreen::Loading => {
                if let Some(selected_world) = &self.selected_world {
                    if let Err(e) = engine.load_world(&PathBuf::from(format!("worlds/{}", selected_world.name))) {
                        log::error!("Failed to load world: {}", e);
                        self.current_screen = MenuScreen::LoadWorld;
                    }
                } else {
                    let seed = match self.create_world_state.seed.parse::<u64>() {
                        Ok(s) => s,
                        Err(_) => {
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
                    
                    if let Err(e) = engine.load_world(&PathBuf::from(format!("worlds/{}", self.create_world_state.name))) {
                        log::error!("Failed to create world: {}", e);
                        self.current_screen = MenuScreen::CreateWorld;
                    }
                }
                
                // Reset create world state after loading
                self.create_world_state = CreateWorldState::default();
            }
            _ => {}
        }
    }
    
    pub fn scan_for_worlds(&mut self) {
        self.worlds_list = helpers::load_saved_worlds();
        
        // Add some default worlds if none found
        if self.worlds_list.is_empty() {
            self.worlds_list = vec![
                WorldMeta { 
                    name: "Test World".to_string(),
                    difficulty: Difficulty::Normal,
                    spawn_point: (0.0, 0.0, 0.0).into(),
                    world_type: WorldType::Normal,
                    seed: 12345,
                    last_played: 0,
                },
                WorldMeta {
                    name: "Creative Build".to_string(),
                    difficulty: Difficulty::Peaceful,
                    spawn_point: (0.0, 0.0, 0.0).into(),
                    world_type: WorldType::Superflat,
                    seed: 67890,
                    last_played: 0,
                }
            ];
        }
    }
}
