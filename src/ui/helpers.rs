use std::path::PathBuf;
use crate::ui::world::WorldMeta;

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
pub fn save_world(world: &WorldMeta) -> std::io::Result<()> {
    let world_dir = get_worlds_dir().join(&world.name);
    std::fs::create_dir_all(&world_dir)?;
    
    let meta_path = world_dir.join("world.meta");
    let meta_json = serde_json::to_string_pretty(world)?;
    std::fs::write(meta_path, meta_json)?;
    
    Ok(())
}

pub fn load_saved_worlds() -> Vec<WorldMeta> {
    let mut worlds = Vec::new();
    if let Ok(worlds_dir) = std::fs::read_dir(get_worlds_dir()) {
        for entry in worlds_dir.flatten() {
            if let Ok(meta_path) = entry.path().join("world.meta").canonicalize() {
                if let Ok(meta_json) = std::fs::read_to_string(meta_path) {
                    if let Ok(world) = serde_json::from_str::<WorldMeta>(&meta_json) {
                        worlds.push(world);
                    }
                }
            }
        }
    }
    worlds
}

fn get_worlds_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("Bloksel");
    dir.push("worlds");
    dir
}

pub fn delete_world(name: &str) {
    let path = Path::new("saves").join(name);
    let _ = std::fs::remove_dir_all(path);
}
