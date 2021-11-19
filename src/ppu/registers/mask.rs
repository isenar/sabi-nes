//! 7  bit  0
//! ---- ----
//! BGRs bMmG
//! |||| ||||
//! |||| |||+- Greyscale (0: normal color, 1: produce a greyscale display)
//! |||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
//! |||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
//! |||| +---- 1: Show background
//! |||+------ 1: Show sprites
//! ||+------- Emphasize red
//! |+-------- Emphasize green
//! +--------- Emphasize blue

use crate::Byte;
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct MaskRegister: Byte {
        const GREYSCALE                = 0b0000_0001;
        const LEFTMOST_8PXL_BACKGROUND = 0b0000_0010;
        const LEFTMOST_8PXL_SPRITE     = 0b0000_0100;
        const SHOW_BACKGROUND          = 0b0000_1000;
        const SHOW_SPRITES             = 0b0001_0000;
        const EMPHASISE_RED            = 0b0010_0000;
        const EMPHASISE_GREEN          = 0b0100_0000;
        const EMPHASISE_BLUE           = 0b1000_0000;
    }
}

impl MaskRegister {
    pub fn update(&mut self, value: Byte) {
        self.bits = value;
    }
}
