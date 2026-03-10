use crate::Byte;
use crate::render::Colour;

#[derive(Debug)]
pub struct Frame {
    pub pixel_data: [Byte; Self::WIDTH * Self::HEIGHT * 3],
    /// Track which pixels have non-transparent background (for sprite priority)
    background_mask: [bool; Self::WIDTH * Self::HEIGHT],
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            pixel_data: [0; Self::WIDTH * Self::HEIGHT * 3],
            background_mask: [false; Self::WIDTH * Self::HEIGHT],
        }
    }
}

impl Frame {
    pub const WIDTH: usize = 256;
    pub const HEIGHT: usize = 240;

    pub fn set_pixel_colour(&mut self, x: usize, y: usize, rgb: Colour) {
        let base = y * 3 * Self::WIDTH + x * 3;

        if base + 2 < self.pixel_data.len() {
            self.pixel_data[base] = rgb.0;
            self.pixel_data[base + 1] = rgb.1;
            self.pixel_data[base + 2] = rgb.2;
        }
    }

    /// Mark a pixel as having background (for sprite priority)
    pub fn set_bg_pixel(&mut self, x: usize, y: usize, rgb: Colour) {
        self.set_pixel_colour(x, y, rgb);
        let idx = y * Self::WIDTH + x;
        if idx < self.background_mask.len() {
            self.background_mask[idx] = true;
        }
    }

    /// Check if a pixel has background
    pub fn has_bg(&self, x: usize, y: usize) -> bool {
        let idx = y * Self::WIDTH + x;
        idx < self.background_mask.len() && self.background_mask[idx]
    }

    /// Clear the background mask for a new frame
    pub fn clear_background_mask(&mut self) {
        self.background_mask.fill(false);
    }
}
