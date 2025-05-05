use std::{path::Path, time::{Instant, Duration}};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct WorldSave {
    pub config: EngineConfig,
    pub chunks: Vec<SerializedChunk>,
    pub player_state: PlayerState,
}

impl WorldSave {
    pub fn auto_save_if_needed(&self, last_save: Instant, interval: f32, path: &Path) -> bool {
        if last_save.elapsed().as_secs_f32() > interval {
            self.save(path).unwrap_or_else(|e| log::error!("Save failed: {}", e));
            true
        } else {
            false
        }
    }

    pub fn save_chunk(coord: ChunkCoord, chunk: &Chunk) -> anyhow::Result<()> {
        // ... chunk serialization logic
    }
}
