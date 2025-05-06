use egui::Context;

pub struct HUD {
    pub show_fps: bool,
    pub show_debug: bool,
    pub show_inventory: bool,
}

impl HUD {
    pub fn new() -> Self {
        Self {
            show_fps: true,
            show_debug: false,
            show_inventory: false,
        }
    }

    pub fn draw(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("hud").show(ctx, |ui| {
            if self.show_fps {
                ui.label(format!("FPS: {:.1}", ctx.fps()));
            }

            if self.show_debug {
                ui.label("Debug Info");
            }

            if self.show_inventory {
                ui.label("Inventory");
            }
        });
    }
}
