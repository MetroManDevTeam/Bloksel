use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct BlockFlags: u32 {
        const NONE = 0;
        const SOLID = 1 << 0;
        const TRANSPARENT = 1 << 1;
        const LIQUID = 1 << 2;
        const FLORA = 1 << 3;
        const DECORATIVE = 1 << 4;
        const LIGHT_LEVEL_MASK = 0xFF << 8;
        const BREAK_RESISTANCE_MASK = 0xFF << 16;
    }
}

impl BlockFlags {
    pub fn new() -> Self {
        Self::NONE
    }

    pub fn with_solid(self, value: bool) -> Self {
        if value {
            self | Self::SOLID
        } else {
            self & !Self::SOLID
        }
    }

    pub fn with_transparent(self, value: bool) -> Self {
        if value {
            self | Self::TRANSPARENT
        } else {
            self & !Self::TRANSPARENT
        }
    }

    pub fn with_liquid(self, value: bool) -> Self {
        if value {
            self | Self::LIQUID
        } else {
            self & !Self::LIQUID
        }
    }

    pub fn with_flora(self, value: bool) -> Self {
        if value {
            self | Self::FLORA
        } else {
            self & !Self::FLORA
        }
    }

    pub fn with_decorative(self, value: bool) -> Self {
        if value {
            self | Self::DECORATIVE
        } else {
            self & !Self::DECORATIVE
        }
    }

    pub fn with_light_level(self, value: u8) -> Self {
        (self & !Self::LIGHT_LEVEL_MASK) | (((value as u32) << 8) & Self::LIGHT_LEVEL_MASK)
    }

    pub fn with_break_resistance(self, value: u8) -> Self {
        (self & !Self::BREAK_RESISTANCE_MASK)
            | (((value as u32) << 16) & Self::BREAK_RESISTANCE_MASK)
    }

    pub fn is_solid(&self) -> bool {
        self.contains(Self::SOLID)
    }

    pub fn is_transparent(&self) -> bool {
        self.contains(Self::TRANSPARENT)
    }

    pub fn is_liquid(&self) -> bool {
        self.contains(Self::LIQUID)
    }

    pub fn is_flora(&self) -> bool {
        self.contains(Self::FLORA)
    }

    pub fn is_decorative(&self) -> bool {
        self.contains(Self::DECORATIVE)
    }

    pub fn light_level(&self) -> u8 {
        ((self.bits() & Self::LIGHT_LEVEL_MASK.bits()) >> 8) as u8
    }

    pub fn break_resistance(&self) -> u8 {
        ((self.bits() & Self::BREAK_RESISTANCE_MASK.bits()) >> 16) as u8
    }
}
