
#[derive(Default, Serialize, Deserialize)]
pub struct WorldMeta {
    pub name: String,
    pub seed: u64,
    pub last_played: chrono::DateTime<chrono::Local>,
    pub play_time: f32,
    pub version: String,
    pub preview_image: Option<Vec<u8>>,
}


pub struct CreateWorldState {
    name: String,
    seed: String,
    world_type: WorldType,
    difficulty: Difficulty,
    bonus_chest: bool,
    generate_structures: bool,
}

#[derive(PartialEq)]
pub enum WorldType {
    Default,
    Flat,
    Amplified,
    LargeBiomes,
}

#[derive(PartialEq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}