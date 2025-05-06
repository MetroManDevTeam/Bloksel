use crate::world::ChunkCoord;
use crate::world::chunk::Chunk;
use anyhow::Result;
use log;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
    path::Path,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
pub struct WorldSave {
    pub config: EngineConfig,
    pub chunks: Vec<SerializedChunk>,
    pub player_state: PlayerState,
}

impl WorldSave {
    pub fn auto_save_if_needed(&self, last_save: Instant, interval: f32, path: &Path) -> bool {
        if last_save.elapsed().as_secs_f32() > interval {
            self.save(path)
                .unwrap_or_else(|e| log::error!("Save failed: {}", e));
            true
        } else {
            false
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        // Implementation of save method
        Ok(())
    }

    pub fn save_chunk(coord: ChunkCoord, chunk: &Chunk) -> Result<()> {
        // Implementation of save_chunk
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)] // Added Clone
struct CompressedSubBlock {
    local_pos: (u8, u8, u8),
    id: BlockId,
    metadata: u8, // Added missing field
    orientation: BlockOrientation,
}

#[derive(Serialize, Deserialize, Debug, Clone)] // Added Clone
struct CompressedBlock {
    position: (usize, usize, usize),
    id: u16,
    sub_blocks: Vec<CompressedSubBlock>,
}
