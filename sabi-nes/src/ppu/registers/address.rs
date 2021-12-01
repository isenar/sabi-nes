use crate::utils::MirroredAddress;
use crate::{Address, Byte};

#[derive(Debug, Clone, Copy, PartialEq)]
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

    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }

    fn set(&mut self, data: Address) {
        self.high = (data >> 8) as Byte;
        self.low = (data & 0xff) as Byte;
    }

    fn mirror_down(&mut self) {
        if self.get() > 0x3fff {
            self.set(self.get().mirror_ppu_addr());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_write_fills_high_byte() {
        let mut addr_reg = AddressRegister::default();
        addr_reg.update(0x01);

        let expected = AddressRegister {
            high: 0x01,
            low: 0x00,
            hi_ptr: false,
        };

        assert_eq!(expected, addr_reg);
        assert_eq!(0x0100, addr_reg.get());
    }

    #[test]
    fn double_write_fills_both_bites() {
        let mut addr_reg = AddressRegister::default();
        addr_reg.update(0x12);
        addr_reg.update(0x34);

        let expected = AddressRegister {
            high: 0x12,
            low: 0x34,
            hi_ptr: true,
        };

        assert_eq!(expected, addr_reg);
        assert_eq!(0x1234, addr_reg.get());
    }

    #[test]
    fn multiple_writes_store_only_two_last_values() {
        let mut addr_reg = AddressRegister::default();
        addr_reg.update(0x01);
        addr_reg.update(0x12);
        addr_reg.update(0x02);
        addr_reg.update(0x23);
        addr_reg.update(0x2f);

        let expected = AddressRegister {
            high: 0x2f,
            low: 0x23,
            hi_ptr: false,
        };

        assert_eq!(expected, addr_reg);
    }

    #[test]
    fn write_with_mirroring() {
        let mut addr_reg = AddressRegister::default();
        addr_reg.update(0x4f);

        let expected = AddressRegister {
            high: 0x0f,
            low: 0x00,
            hi_ptr: false,
        };

        assert_eq!(expected, addr_reg);
    }
}
