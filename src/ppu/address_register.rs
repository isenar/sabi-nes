use crate::{Address, Byte};

#[derive(Debug, Clone, Copy)]
pub struct AddressRegister {
    high: Byte,
    low: Byte,
    hi_ptr: bool,
}

impl Default for AddressRegister {
    fn default() -> Self {
        Self {
            high: 0,
            low: 0,
            hi_ptr: true,
        }
    }
}

impl AddressRegister {
    pub fn update(&mut self, value: Byte) {
        if self.hi_ptr {
            self.high = value;
        } else {
            self.low = value;
        }
        self.hi_ptr = !self.hi_ptr;

        self.mirror_down();
    }

    pub fn increment(&mut self, increment: Byte) {
        let prev_low = self.low;
        self.low = self.low.wrapping_add(increment);

        if prev_low > self.low {
            self.high = self.high.wrapping_add(1);
        }

        self.mirror_down();
    }

    pub fn get(&self) -> Address {
        (self.high as Address) << 8 | self.low as Address
    }

    #[allow(unused)]
    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }

    fn set(&mut self, data: Address) {
        self.high = (data >> 8) as Byte;
        self.low = (data & 0xff) as Byte;
    }

    fn mirror_down(&mut self) {
        if self.get() > 0x3fff {
            self.set(self.get() & 0b0011_1111_1111_1111)
        }
    }
}
