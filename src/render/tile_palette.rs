use crate::Byte;
use super::{Colour, Palette};

pub struct TilePalette(pub [Byte; 4]);

impl TilePalette {
    pub fn colour(&self, pixel_value: Byte, palette: &impl Palette) -> Colour {
        palette.get(self.0[pixel_value.as_usize()].as_usize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Byte;
    use crate::render::{Palette, SystemPalette};

    #[test]
    fn colour_uses_pixel_value_as_palette_entry_index() {
        // TilePalette entries: [0, 1, 2, 3] — each pixel value maps to that system palette index
        let tp = TilePalette([Byte::new(0), Byte::new(1), Byte::new(2), Byte::new(3)]);
        let system = SystemPalette::new();
        assert_eq!(tp.colour(Byte::new(0), &system), system.get(0));
        assert_eq!(tp.colour(Byte::new(1), &system), system.get(1));
        assert_eq!(tp.colour(Byte::new(2), &system), system.get(2));
        assert_eq!(tp.colour(Byte::new(3), &system), system.get(3));
    }

    #[test]
    fn colour_indexes_through_tile_palette_entry() {
        // Entry for pixel value 1 is system palette index 20
        let tp = TilePalette([Byte::new(0), Byte::new(20), Byte::new(0), Byte::new(0)]);
        let system = SystemPalette::new();
        assert_eq!(tp.colour(Byte::new(1), &system), system.get(20));
    }
}
