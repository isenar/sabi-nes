use crate::Byte;

#[derive(Debug, Default)]
pub struct ScrollRegister {
    scroll_x: Byte,
    scroll_y: Byte,
    latch: bool,
}

impl ScrollRegister {
    pub fn write(&mut self, value: Byte) {
        if self.latch {
            self.scroll_x = value;
        } else {
            self.scroll_y = value;
        }

        self.latch = !self.latch;
    }
}
