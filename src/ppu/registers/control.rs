//! 7  bit  0
// ---- ----
// VPHB SINN
// |||| ||||
// |||| ||++- Base nametable address
// |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
// |||| |+--- VRAM address increment per CPU read/write of PPUDATA
// |||| |     (0: add 1, going across; 1: add 32, going down)
// |||| +---- Sprite pattern table address for 8x8 sprites
// ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
// |||+------ Background pattern table address (0: $0000; 1: $1000)
// ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels)
// |+-------- PPU master/slave select
// |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
// +--------- Generate an NMI at the start of the
//            vertical blanking interval (0: off; 1: on)

use crate::{Address, Byte};
use bitflags::bitflags;

const NAMETABLE_BASE_ADDR: Address = 0x2000;

bitflags! {
    #[derive(Default)]
    pub struct ControlRegister: Byte {
        const NAMETABLE1              = 0b0000_0001;
        const NAMETABLE2              = 0b0000_0010;
        const VRAM_ADDR_INCREMENT     = 0b0000_0100;
        const SPRITE_PATTERN_ADDR     = 0b0000_1000;
        const BACKROUND_PATTERN_ADDR  = 0b0001_0000;
        const SPRITE_SIZE             = 0b0010_0000;
        const MASTER_SLAVE_SELECT     = 0b0100_0000;
        const GENERATE_NMI            = 0b1000_0000;
    }
}

impl ControlRegister {
    pub fn update(&mut self, value: Byte) {
        self.bits = value;
    }

    pub fn vram_addr_increment(&self) -> Byte {
        if self.contains(Self::VRAM_ADDR_INCREMENT) {
            32
        } else {
            1
        }
    }

    pub fn sprite_pattern_address(&self) -> Address {
        Address::from(self.contains(Self::SPRITE_PATTERN_ADDR)) * 0x1000
    }

    pub const fn name_table_address(&self) -> Address {
        let address_lower = self.contains(Self::NAMETABLE1) as Address * 0x400;
        let address_higher = self.contains(Self::NAMETABLE2) as Address * 0x800;

        NAMETABLE_BASE_ADDR + address_lower + address_higher
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vram_addr_increment_disabled() {
        let register = ControlRegister::empty();

        assert_eq!(1, register.vram_addr_increment());
    }

    #[test]
    fn vram_addr_increment_enabled() {
        let register = ControlRegister::VRAM_ADDR_INCREMENT;

        assert_eq!(32, register.vram_addr_increment());
    }
}
