use crate::{engine::VoxelEngine, world::WorldMeta};
use egui::{CentralPanel, ComboBox, Context, Grid, Spinner, Window};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuScreen {
    Main,
    WorldSelect,
    CreateWorld,
    Loading,
}

#[derive(Debug)]
pub struct MenuState {
    current_screen: MenuScreen,
    create_world_state: CreateWorldState,
    selected_world: Option<WorldMeta>,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            current_screen: MenuScreen::Main,
            create_world_state: CreateWorldState::default(),
            selected_world: None,
        }
    }

    pub fn show(&mut self, ctx: &Context, engine: &mut VoxelEngine) {
        match self.current_screen {
            MenuScreen::Main => self.show_main_menu(ctx),
            MenuScreen::WorldSelect => self.show_world_select(ctx),
            MenuScreen::CreateWorld => self.show_create_world(ctx),
            MenuScreen::Loading => self.show_loading_screen(ctx),
        }

        self.handle_transitions(engine);
    }

    fn show_main_menu(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Bloksel");
                ui.add_space(20.0);

                if ui.button("Play").clicked() {
                    self.current_screen = MenuScreen::WorldSelect;
                }

                if ui.button("Settings").clicked() {
                    // TODO: Show settings
                }

                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }

    fn show_world_select(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Select World");
                ui.add_space(20.0);

                if ui.button("Create New World").clicked() {
                    self.current_screen = MenuScreen::CreateWorld;
                }

                if ui.button("Back").clicked() {
                    self.current_screen = MenuScreen::Main;
                }
            });
        });
    }

    fn show_create_world(&mut self, ctx: &Context) {
        Window::new("Create New World")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                Grid::new("world_settings")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("World Name:");
                        ui.text_edit_singleline(&mut self.create_world_state.name);
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
                    });

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        self.current_screen = MenuScreen::Loading;
                    }
                    if ui.button("Cancel").clicked() {
                        self.current_screen = MenuScreen::WorldSelect;
                    }
                });
            });
    }

    fn show_loading_screen(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Loading...");
                ui.add_space(20.0);
                ui.add(Spinner::new().size(50.0));
            });
        });
    }

    fn handle_transitions(&mut self, engine: &mut VoxelEngine) {
        match self.current_screen {
            MenuScreen::Loading => {
                let config = engine.create_world_config(&self.create_world_state);
                engine.load_world(config);
                self.current_screen = MenuScreen::Main;
            }
            _ => {}
        }
    }
}

#[derive(Debug, Default)]
pub struct CreateWorldState {
    pub name: String,
    pub world_type: WorldType,
    pub difficulty: Difficulty,
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
