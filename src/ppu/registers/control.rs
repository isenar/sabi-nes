use crate::Byte;
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct ControlRegister: Byte {
        const NAMETABLE1              = 0b00000001;
        const NAMETABLE2              = 0b00000010;
        const VRAM_ADDR_INCREMENT     = 0b00000100;
        const SPRITE_PATTERN_ADDR     = 0b00001000;
        const BACKROUND_PATTERN_ADDR  = 0b00010000;
        const SPRITE_SIZE             = 0b00100000;
        const MASTER_SLAVE_SELECT     = 0b01000000;
        const GENERATE_NMI            = 0b10000000;
    }
}

impl ControlRegister {
    pub fn update(&mut self, value: Byte) {
        self.bits = value;
    }

    pub fn vram_addr_increment(&self) -> Byte {
        if !self.contains(Self::VRAM_ADDR_INCREMENT) {
            1
        } else {
            32
        }
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
