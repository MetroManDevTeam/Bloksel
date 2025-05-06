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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockPhysics {
    pub solid: bool,
    pub liquid: bool,
    pub gas: bool,
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
    pub viscosity: f32,
}

impl Default for BlockPhysics {
    fn default() -> Self {
        Self {
            solid: true,
            liquid: false,
            gas: false,
            density: 1.0,
            friction: 0.5,
            restitution: 0.5,
            viscosity: 0.0,
        }
    }
}

impl BlockPhysics {
    pub fn new(solid: bool, liquid: bool, gas: bool, physics: PhysicsProperties) -> Self {
        Self {
            solid,
            liquid,
            gas,
            physics: physics,
        }
    }

    pub fn solid() -> Self {
        Self {
            solid: true,
            liquid: false,
            gas: false,
            density: 1.0,
            friction: 0.5,
            restitution: 0.2,
            viscosity: 0.0,
        }
    }

    pub fn liquid() -> Self {
        Self {
            solid: false,
            liquid: true,
            gas: false,
            density: 0.8,
            friction: 0.1,
            restitution: 0.0,
            viscosity: 0.5,
        }
    }

    pub fn gas() -> Self {
        Self {
            solid: false,
            liquid: false,
            gas: true,
            density: 0.1,
            friction: 0.0,
            restitution: 0.0,
            viscosity: 0.0,
        }
    }

    pub fn mass(&self, volume: f32) -> f32 {
        self.density * volume
    }
}

impl From<BlockFlags> for BlockPhysics {
    fn from(flags: BlockFlags) -> Self {
        let solid = flags.contains(BlockFlags::SOLID);
        let liquid = flags.contains(BlockFlags::LIQUID);
        let gas = flags.contains(BlockFlags::GAS);

        let mut physics = Self {
            solid,
            liquid,
            gas,
            density: if solid {
                2.0
            } else if liquid {
                1.0
            } else {
                0.001
            },
            friction: if flags.contains(BlockFlags::SLIPPERY) {
                0.1
            } else {
                0.5
            },
            restitution: if flags.contains(BlockFlags::BOUNCY) {
                0.8
            } else {
                0.2
            },
            viscosity: if liquid { 1.0 } else { 0.0 },
        };

        if flags.contains(BlockFlags::HEAVY) {
            physics.density *= 2.0;
        }
        if flags.contains(BlockFlags::LIGHT) {
            physics.density *= 0.5;
        }

        physics
    }
}
