use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlockPhysics {
    pub solid: bool,
    pub liquid: bool,
    pub gas: bool,
    pub physics: PhysicsProperties,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhysicsProperties {
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
    pub viscosity: f32,
}

impl Default for BlockPhysics {
    fn default() -> Self {
        Self {
            solid: false,
            liquid: false,
            gas: false,
            physics: PhysicsProperties {
                density: 1000.0, // Water density as default
                friction: 0.6,
                restitution: 0.0,
                viscosity: 0.0,
            },
        }
    }
}

impl BlockPhysics {
    pub fn new(solid: bool, liquid: bool, gas: bool, physics: PhysicsProperties) -> Self {
        Self {
            solid,
            liquid,
            gas,
            physics,
        }
    }

    pub fn solid() -> Self {
        Self {
            solid: true,
            liquid: false,
            gas: false,
            physics: PhysicsProperties {
                density: 1.0,
                friction: 0.5,
                restitution: 0.2,
                viscosity: 0.0,
            },
        }
    }

    pub fn liquid() -> Self {
        Self {
            solid: false,
            liquid: true,
            gas: false,
            physics: PhysicsProperties {
                density: 0.8,
                friction: 0.1,
                restitution: 0.0,
                viscosity: 0.5,
            },
        }
    }

    pub fn gas() -> Self {
        Self {
            solid: false,
            liquid: false,
            gas: true,
            physics: PhysicsProperties {
                density: 0.1,
                friction: 0.0,
                restitution: 0.0,
                viscosity: 0.0,
            },
        }
    }

    pub fn mass(&self, volume: f32) -> f32 {
        self.physics.density * volume
    }
}

impl From<BlockFlags> for BlockPhysics {
    fn from(flags: BlockFlags) -> Self {
        BlockPhysics {
            solid: flags.solid,
            liquid: flags.liquid,
            gas: false,
            physics: PhysicsProperties {
                density: if flags.solid { 1.0 } else { 0.0 },
                friction: if flags.solid { 0.5 } else { 0.1 },
                restitution: if flags.solid { 0.2 } else { 0.8 },
                viscosity: if flags.liquid { 0.5 } else { 0.0 },
            },
        }
    }
}
