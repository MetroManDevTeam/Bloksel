use eframe::egui;
use crate::player::PlayerState;

pub struct Hud {
    crosshair: bool,
    debug_info: bool,
}

impl Hud {
    pub fn new() -> Self {
        Self {
            crosshair: true,
            debug_info: false,
        }
    }

    pub fn draw(&mut self, ctx: &egui::Context, player: &PlayerState) {
        egui::TopBottomPanel::bottom("hud_bottom").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Health bar
                ui.label(format!("❤ {}", player.health as i32));
                
                // Hunger bar
                ui.label(format!("🍗 {}", player.hunger as i32));
                
                // Hotbar
                ui.horizontal(|ui| {
                    for (i, slot) in player.inventory.hotbar.iter().enumerate() {
                        let frame = egui::Frame::none()
                            .fill(if i == player.selected_slot {
                                egui::Color32::from_rgba_unmultiplied(100, 100, 100, 100)
                            } else {
                                egui::Color32::TRANSPARENT
                            })
                            .inner_margin(4.0);
                        
                        frame.show(ui, |ui| {
                            if let Some(item) = slot {
                                ui.label(&item.item_id);
                            } else {
                                ui.label(" ");
                            }
                        });
                    }
                });
            });
        });

        if self.crosshair {
            egui::CentralPanel::default()
                .frame(egui::Frame::none())
                .show(ctx, |ui| {
                    let center = ui.available_rect().center();
                    ui.painter().line_segment(
                        [center - egui::vec2(10.0, 0.0), center + egui::vec2(10.0, 0.0)],
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                    );
                    ui.painter().line_segment(
                        [center - egui::vec2(0.0, 10.0), center + egui::vec2(0.0, 10.0)],
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                    );
                });
        }

        if self.debug_info {
            egui::Window::new("Debug Info").show(ctx, |ui| {
                ui.label(format!("Position: {:?}", player.position));
                ui.label(format!("Rotation: {:?}", player.rotation));
                ui.label(format!("Velocity: {:?}", player.velocity));
                ui.label(format!("Flying: {}", player.is_flying));
            });
        }
    }
}
