use crate::render::Rgb;
use crate::Byte;

#[derive(Debug)]
pub struct Frame {
    pub data: Vec<Byte>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            data: vec![0; Self::WIDTH * Self::HEIGHT * 3],
        }
    }
}

impl Frame {
    const WIDTH: usize = 256;
    const HEIGHT: usize = 240;

    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: Rgb) {
        let base = y * 3 * Self::WIDTH + x * 3;

        if base + 2 < self.data.len() {
            self.data[base] = rgb.0;
            self.data[base + 1] = rgb.1;
            self.data[base + 2] = rgb.2;
        }
    }
}
