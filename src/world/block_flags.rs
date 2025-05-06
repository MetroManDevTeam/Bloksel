use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Serialize, Deserialize, Default, Debug, Clone, Copy)]
    pub struct BlockFlags: u32 {
        const SOLID = 0x1;
        const TRANSPARENT = 0x2;
        const LIQUID = 0x4;
        const FLORA = 0x8;
        const DECORATIVE = 0x10;
        const LIGHT_LEVEL_MASK = 0xFF00;
        const BREAK_RESISTANCE_MASK = 0xFF0000;
    }
}

impl BlockFlags {
    pub fn with_solid(mut self, solid: bool) -> Self {
        if solid {
            self.insert(Self::SOLID);
        } else {
            self.remove(Self::SOLID);
        }
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        if transparent {
            self.insert(Self::TRANSPARENT);
        } else {
            self.remove(Self::TRANSPARENT);
        }
        self
    }

    pub fn with_liquid(mut self, liquid: bool) -> Self {
        if liquid {
            self.insert(Self::LIQUID);
        } else {
            self.remove(Self::LIQUID);
        }
        self
    }

    pub fn with_flora(mut self, flora: bool) -> Self {
        if flora {
            self.insert(Self::FLORA);
        } else {
            self.remove(Self::FLORA);
        }
        self
    }

    pub fn with_decorative(mut self, decorative: bool) -> Self {
        if decorative {
            self.insert(Self::DECORATIVE);
        } else {
            self.remove(Self::DECORATIVE);
        }
        self
    }

    pub fn with_light_level(mut self, light_level: u8) -> Self {
        self.0 = (self.0 & !Self::LIGHT_LEVEL_MASK.bits()) | ((light_level as u32) << 8);
        self
    }

    pub fn with_break_resistance(mut self, break_resistance: u8) -> Self {
        self.0 = (self.0 & !Self::BREAK_RESISTANCE_MASK.bits()) | ((break_resistance as u32) << 16);
        self
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
        ((self.0 & Self::LIGHT_LEVEL_MASK.bits()) >> 8) as u8
    }

    pub fn break_resistance(&self) -> u8 {
        ((self.0 & Self::BREAK_RESISTANCE_MASK.bits()) >> 16) as u8
    }
}
