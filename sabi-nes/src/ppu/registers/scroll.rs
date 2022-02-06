use crate::Byte;

#[derive(Debug, Default)]
pub struct ScrollRegister {
    pub scroll_x: Byte,
    pub scroll_y: Byte,
    latch: bool,
}

impl ScrollRegister {
    pub fn write(&mut self, value: Byte) {
        if self.latch {
            self.scroll_y = value;
        } else {
            self.scroll_x = value;
        }

        self.latch = !self.latch;
    }

    pub fn reset_latch(&mut self) {
        self.latch = false;
    }
}
