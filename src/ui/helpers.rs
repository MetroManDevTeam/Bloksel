
// Helper components


pub fn button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized(
        [200.0, 40.0], 
        egui::Button::new(text).fill(egui::Color32::from_rgb(40, 40, 40))
}


impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id) };
    }
}

pub fn logo(ui: &mut egui::Ui) {
    ui.heading("VOXEL ENGINE");
    ui.add_space(10.0);
    ui.label("Version 1.0.0");
}

// World management
pub fn load_saved_worlds() -> Vec<WorldMeta> {
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

pub fn delete_world(name: &str) {
    let path = Path::new("saves").join(name);
    let _ = std::fs::remove_dir_all(path);
}
