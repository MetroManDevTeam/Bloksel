use crate::config::{
    chunksys::ChunkSysConfig, game::TerrainConfig, gameplay::GameplayConfig,
    rendering::RenderConfig, worldgen::WorldGenConfig,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub world_seed: u64,
    pub render_distance: u32,
    pub lod_levels: [u32; 3],
    pub chunk_size: u32,
    pub texture_atlas_size: u32,
    pub max_chunk_pool_size: usize,
    pub vsync: bool,
    pub async_loading: bool,
    pub fov: f32,
    pub view_distance: f32,
    pub save_interval: f32,
 
    pub terrain: TerrainConfig,
    pub gameplay: GameplayConfig,
    pub rendering: RenderConfig,
    pub chunksys: ChunkSysConfig,
    pub worldgen: WorldGenConfig,
}
