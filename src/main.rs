
use eframe::egui;

struct MenuApp {
    selected_option: usize,
    options: Vec<String>,
}

impl Default for MenuApp {
    fn default() -> Self {
        Self {
            selected_option: 0,
            options: vec![
                "Start Game".to_string(),
                "Settings".to_string(),
                "Exit".to_string(),
            ],
        }
    }
}

impl eframe::App for MenuApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Voxel Engine Menu");
            
            for (idx, option) in self.options.iter().enumerate() {
                if ui.button(option).clicked() {
                    self.selected_option = idx;
                    match idx {
                        0 => println!("Starting game..."),
                        1 => println!("Opening settings..."),
                        2 => frame.close(),
                        _ => unreachable!(),
                    }
                }
            }
        });
    }
}

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(400.0, 300.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Voxel Engine",
        options,
        Box::new(|_cc| Box::new(MenuApp::default())),
    );
}
