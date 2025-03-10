use crate::Byte;
use crate::render::Rgb;

#[derive(Debug)]
pub struct Frame {
    pub pixel_data: [Byte; Self::WIDTH * Self::HEIGHT * 3],
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            pixel_data: [0; Self::WIDTH * Self::HEIGHT * 3],
        }
    }
}

impl Frame {
    pub const WIDTH: usize = 256;
    pub const HEIGHT: usize = 240;

    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: Rgb) {
        let base = y * 3 * Self::WIDTH + x * 3;

        if base + 2 < self.pixel_data.len() {
            self.pixel_data[base] = rgb.0;
            self.pixel_data[base + 1] = rgb.1;
            self.pixel_data[base + 2] = rgb.2;
        }
    }
}
