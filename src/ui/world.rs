use crate::config::core::EngineConfig;

#[derive(Debug, Clone)]
pub struct WorldMeta {
    pub name: String,
    pub world_type: WorldType,
    pub difficulty: Difficulty,
    pub seed: String,
    pub last_played: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateWorldState {
    pub name: String,
    pub world_type: WorldType,
    pub difficulty: Difficulty,
    pub seed: String,
    pub config: EngineConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldType {
    Normal,
    Flat,
    Custom,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}
