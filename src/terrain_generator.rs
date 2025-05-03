use noise::{NoiseFn, Seedable, Simplex};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockOrientation {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl BlockOrientation {
    pub fn to_char(&self) -> char {
        match self {
            Self::North => 'N',
            Self::South => 'S',
            Self::East => 'E',
            Self::West => 'W',
            Self::Up => 'U',
            Self::Down => 'D',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'N' => Some(Self::North),
            'S' => Some(Self::South),
            'E' => Some(Self::East),
            'W' => Some(Self::West),
            'U' => Some(Self::Up),
            'D' => Some(Self::Down),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockState {
    Full,
    Half(BlockOrientation),
    Quarter(BlockOrientation),
    Slab,
    Custom(u8),
}

impl BlockState {
    pub fn to_suffix(&self) -> String {
        match self {
            Self::Full => "F".to_string(),
            Self::Half(orient) => format!("H{}", orient.to_char()),
            Self::Quarter(orient) => format!("Q{}", orient.to_char()),
            Self::Slab => "S".to_string(),
            Self::Custom(n) => format!("C{}", n),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockType {
    Air,
    Stone,
    Dirt,
    Grass,
    Sand,
    Water,
    Bedrock,
}

impl BlockType {
    pub fn to_id(&self) -> u8 {
        match self {
            Self::Air => 0,
            Self::Stone => 1,
            Self::Dirt => 2,
            Self::Grass => 3,
            Self::Sand => 4,
            Self::Water => 5,
            Self::Bedrock => 6,
        }
    }

    pub fn default_texture_path(&self) -> PathBuf {
        PathBuf::from(format!("textures/default/{}.png", self.to_id()))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Block {
    pub block_type: BlockType,
    pub state: BlockState,
}

impl Block {
    pub fn new(block_type: BlockType, state: BlockState) -> Self {
        Self { block_type, state }
    }

    pub fn to_id(&self) -> String {
        format!("{}{}", self.block_type.to_id(), self.state.to_suffix())
    }

    pub fn is_solid(&self) -> bool {
        !matches!(self.block_type, BlockType::Air | BlockType::Water)
    }

    pub fn can_merge(&self, other: &Block) -> bool {
        self.block_type == other.block_type && self.state == other.state
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChunkData {
    pub blocks: [[[Block; 30]; 30]; 30],
    pub position: (i32, i32),
}

#[derive(Serialize, Deserialize)]
pub struct TerrainGenerator {
    noise: Simplex,
    seed: u32,
    chunk_cache: HashMap<(i32, i32), ChunkData>,
    texture_path: PathBuf,
}

impl TerrainGenerator {
    pub fn new(seed: u32, texture_path: Option<PathBuf>) -> Self {
        let mut noise = Simplex::new();
        noise = noise.set_seed(seed);
        
        let texture_path = texture_path.unwrap_or_else(|| PathBuf::from("textures/default"));

        Self {
            noise,
            seed,
            chunk_cache: HashMap::new(),
            texture_path,
        }
    }
    
    pub fn generate_chunk(&mut self, x: i32, z: i32) -> ChunkData {
        let chunk_key = (x, z);
        
        if let Some(chunk) = self.chunk_cache.get(&chunk_key) {
            return chunk.clone();
        }
        
        let mut chunk = ChunkData {
            blocks: [[[Block::new(BlockType::Air, BlockState::Full); 30]; 30]; 30],
            position: (x, z),
        };
        
        self.generate_terrain(&mut chunk);
        self.chunk_cache.insert(chunk_key, chunk.clone());
        chunk
    }
    
    fn generate_terrain(&self, chunk: &mut ChunkData) {
        let (chunk_x, chunk_z) = chunk.position;
        
        for x in 0..30 {
            for z in 0..30 {
                let world_x = (chunk_x * 30 + x as i32) as f64;
                let world_z = (chunk_z * 30 + z as i32) as f64;
                
                let height = self.sample_height(world_x, world_z);
                let height_int = height as i32;
                
                // Generate column
                for y in 0..30 {
                    let world_y = (y as i32) + (chunk_x.abs() % 16 * 30);
                    
                    if world_y == 0 {
                        chunk.blocks[x][z][y] = Block::new(BlockType::Bedrock, BlockState::Full);
                    } else if world_y < height_int - 4 {
                        chunk.blocks[x][z][y] = Block::new(BlockType::Stone, BlockState::Full);
                    } else if world_y < height_int - 1 {
                        chunk.blocks[x][z][y] = Block::new(BlockType::Dirt, BlockState::Full);
                    } else if world_y < height_int {
                        // Surface layer with partial blocks
                        if height_int % 7 == 0 && y == height_int as usize - 1 {
                            // Rare quarter stone block
                            let orientation = self.determine_orientation(chunk, x, y, z);
                            chunk.blocks[x][z][y] = Block::new(
                                BlockType::Stone,
                                BlockState::Quarter(orientation)
                            );
                        } else if height_int % 3 == 0 && y == height_int as usize - 1 {
                            // Half blocks
                            let orientation = if height_int % 2 == 0 {
                                BlockOrientation::Up
                            } else {
                                BlockOrientation::North
                            };
                            chunk.blocks[x][z][y] = Block::new(
                                if height < 48.0 { BlockType::Sand } else { BlockType::Grass },
                                BlockState::Half(orientation)
                            );
                        } else {
                            // Regular full block
                            chunk.blocks[x][z][y] = Block::new(
                                if height < 48.0 { BlockType::Sand } else { BlockType::Grass },
                                BlockState::Full
                            );
                        }
                    } else if world_y < 50 {
                        chunk.blocks[x][z][y] = Block::new(BlockType::Water, BlockState::Full);
                    }
                }
            }
        }
    }
    
    fn determine_orientation(&self, chunk: &ChunkData, x: usize, y: usize, z: usize) -> BlockOrientation {
        let directions = [
            (0, 0, 1, BlockOrientation::North),
            (0, 0, -1, BlockOrientation::South),
            (1, 0, 0, BlockOrientation::East),
            (-1, 0, 0, BlockOrientation::West),
            (0, 1, 0, BlockOrientation::Up),
            (0, -1, 0, BlockOrientation::Down),
        ];
        
        let mut max_diff = -1.0;
        let mut best_orientation = BlockOrientation::Up;
        
        for (dx, dy, dz, orient) in directions.iter() {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            let nz = z as i32 + dz;
            
            if nx >= 0 && nx < 30 && ny >= 0 && ny < 30 && nz >= 0 && nz < 30 {
                let neighbor = &chunk.blocks[nx as usize][nz as usize][ny as usize];
                if neighbor.block_type != BlockType::Air {
                    let diff = match orient {
                        BlockOrientation::Up => 1.0,
                        BlockOrientation::Down => -1.0,
                        _ => 0.5,
                    };
                    
                    if diff > max_diff {
                        max_diff = diff;
                        best_orientation = *orient;
                    }
                }
            }
        }
        
        best_orientation
    }
    
    fn sample_height(&self, x: f64, z: f64) -> f64 {
        let scale = 0.01;
        let amplitude = 20.0;
        
        let mut value = self.noise.get([x * scale, z * scale]) * amplitude;
        value += self.noise.get([x * scale * 2.0, z * scale * 2.0]) * amplitude * 0.5;
        
        if value < 40.0 {
            value = value * 0.7 + 40.0 * 0.3;
        }
        
        value + 50.0
    }
    
    pub fn get_texture_path(&self, block_type: BlockType) -> PathBuf {
        let custom_path = self.texture_path.join(format!("{}.png", block_type.to_id()));
        if custom_path.exists() {
            custom_path
        } else {
            block_type.default_texture_path()
        }
    }
    
    pub fn clear_cache(&mut self) {
        self.chunk_cache.clear();
    }
}
