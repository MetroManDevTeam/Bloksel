use egui::{Context, TopBottomPanel};
use crate::world::BlockRegistry;

pub struct HUD {
    block_registry: BlockRegistry,
}

impl HUD {
    pub fn new(block_registry: BlockRegistry) -> Self {
        Self {
            block_registry,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        TopBottomPanel::top("hud").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Selected Block: None");
                ui.separator();
                ui.label("FPS: 60");
            });
        });
    }
}
