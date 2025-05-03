use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct LoadingScreen {
    progress: Arc<Mutex<f32>>,
    loading_thread: Option<thread::JoinHandle<()>>,
    chunks_loaded: usize,
    total_chunks: usize,
}

impl LoadingScreen {
    pub fn new() -> Self {
        Self {
            progress: Arc::new(Mutex::new(0.0)),
            loading_thread: None,
            chunks_loaded: 0,
            total_chunks: 9, // 3x3 chunks around player
        }
    }

    pub fn start_loading(&mut self) {
        let progress = self.progress.clone();
        let total_chunks = self.total_chunks;
        
        self.loading_thread = Some(thread::spawn(move || {
            // Simulate chunk loading
            for i in 0..total_chunks {
                thread::sleep(std::time::Duration::from_millis(500));
                *progress.lock().unwrap() = (i + 1) as f32 / total_chunks as f32;
            }
        }));
    }

    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) -> bool {
        let mut finished = false;
        
        // Check if loading is complete
        if let Some(thread) = &self.loading_thread {
            if thread.is_finished() {
                finished = true;
                self.loading_thread = None;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Loading World...");
            
            let progress = *self.progress.lock().unwrap();
            ui.add(egui::ProgressBar::new(progress).text(format!(
                "Loading chunks: {}/{}",
                (progress * self.total_chunks as f32).ceil() as usize,
                self.total_chunks
            )));
            
            if finished {
                ui.label("Loading complete! Starting game...");
                if ui.button("Continue").clicked() {
                    // Here you would transition to the actual game
                    println!("Game starting!");
                }
            }
        });

        finished
    }
}
