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
    #[derive(Default, Debug)]
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Color {
    Red,
    Green,
    Blue,
}

impl MaskRegister {
    pub fn update(&mut self, value: Byte) {
        *self = Self::from_bits_retain(value);
    }

    pub fn show_background(&self) -> bool {
        self.contains(MaskRegister::SHOW_BACKGROUND)
    }

    pub fn show_sprites(&self) -> bool {
        self.contains(MaskRegister::SHOW_SPRITES)
    }

    #[allow(unused)]
    pub fn emphasized_colors(&self) -> Vec<Color> {
        let mut colors = Vec::with_capacity(3);

        if self.contains(MaskRegister::EMPHASISE_RED) {
            colors.push(Color::Red);
        }

        if self.contains(MaskRegister::EMPHASISE_GREEN) {
            colors.push(Color::Green);
        }

        if self.contains(MaskRegister::EMPHASISE_BLUE) {
            colors.push(Color::Blue);
        }

        colors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_colors_to_emphasize() {
        let register = MaskRegister::empty();
        let emphasized_colors = register.emphasized_colors();
        let expected = Vec::<Color>::new();

        assert_eq!(expected, emphasized_colors);
    }

    #[test]
    fn all_colors_emphasized() {
        let register = MaskRegister::EMPHASISE_RED
            | MaskRegister::EMPHASISE_GREEN
            | MaskRegister::EMPHASISE_BLUE;
        let emphasized_colors = register.emphasized_colors();
        let expected = vec![Color::Red, Color::Green, Color::Blue];

        assert_eq!(expected, emphasized_colors);
    }
}
