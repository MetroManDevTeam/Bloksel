use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct BlockFlags {
    pub is_solid: bool,
    pub is_transparent: bool,
    pub is_liquid: bool,
    pub is_flora: bool,
    pub is_decorative: bool,
    pub light_level: u8,
    pub break_resistance: u8,
}

impl BlockFlags {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_solid(mut self, value: bool) -> Self {
        self.is_solid = value;
        self
    }

    pub fn with_transparent(mut self, value: bool) -> Self {
        self.is_transparent = value;
        self
    }

    pub fn with_liquid(mut self, value: bool) -> Self {
        self.is_liquid = value;
        self
    }

    pub fn with_flora(mut self, value: bool) -> Self {
        self.is_flora = value;
        self
    }

    pub fn with_decorative(mut self, value: bool) -> Self {
        self.is_decorative = value;
        self
    }

    pub fn with_light_level(mut self, value: u8) -> Self {
        self.light_level = value;
        self
    }

    pub fn with_break_resistance(mut self, value: u8) -> Self {
        self.break_resistance = value;
        self
    }
}
