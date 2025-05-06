#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlockFlags {
    pub transparent: bool,
    pub emissive: bool,
    pub flammable: bool,
    pub conductive: bool,
    pub magnetic: bool,
    pub liquid: bool,
    pub climbable: bool,
    pub occludes: bool,
    pub solid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPhysics {
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
    pub dynamic: bool,
    pub passable: bool,
    pub break_resistance: f32,
    pub flammability: f32,
    pub thermal_conductivity: f32,
    pub emissive: bool,
    pub light_level: u8,
    pub 
    physics: HashMap<BlockId, BlockPhysics>,  // Add this
     
}

impl Default for BlockPhysics {
    fn default() -> Self {
        Self {
            density: 1000.0, // Water density as default
            friction: 0.6,
            restitution: 0.0,
            dynamic: false,
            passable: false,
            break_resistance: 1.0,
            flammability: 0.0,
            thermal_conductivity: 0.5,
            emissive: false,
            light_level: 0,
        }
    }
}

impl BlockPhysics {
    pub fn solid(density: f32) -> Self {
        Self {
            density,
            friction: 0.6,
            restitution: 0.1,
            dynamic: false,
            passable: false,
            ..Default::default()
        }
    }

    pub fn liquid(density: f32) -> Self {
        Self {
            density,
            friction: 0.0,
            restitution: 0.0,
            dynamic: true,
            passable: true,
            ..Default::default()
        }
    }

    pub fn gas() -> Self {
        Self {
            density: 1.2, // Air density
            friction: 0.0,
            restitution: 0.0,
            dynamic: true,
            passable: true,
            ..Default::default()
        }
    }

    pub fn mass(&self, volume: f32) -> f32 {
        self.density * volume
    }
}

