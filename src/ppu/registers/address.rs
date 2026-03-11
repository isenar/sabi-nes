use crate::utils::MirroredAddress;
use crate::{Address, Byte};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressRegister {
    high: Byte,
    low: Byte,
    hi_ptr: bool,
}

impl Default for AddressRegister {
    fn default() -> Self {
        Self {
            high: Byte::default(),
            low: Byte::default(),
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
        self.low = self.low.wrapping_add(increment.value());

        if prev_low > self.low {
            self.high = self.high.wrapping_add(1);
        }

        self.mirror_down();
    }

    pub fn get(&self) -> Address {
        Address::new(((self.high.as_word()) << 8) | self.low.as_word())
    }

    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }

    // TODO: Should be word! Byte should implement a method for truncated value from word
    fn set(&mut self, data: Address) {
        self.high = Byte::new((data.value() >> 8) as u8);
        self.low = Byte::new((data.value() & 0xff) as u8);
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
        addr_reg.update(0x01.into());

        let expected = AddressRegister {
            high: 0x01.into(),
            low: 0x00.into(),
            hi_ptr: false,
        };

        assert_eq!(expected, addr_reg);
        assert_eq!(addr_reg.get(), 0x0100);
    }

    #[test]
    fn double_write_fills_both_bites() {
        let mut addr_reg = AddressRegister::default();
        addr_reg.update(0x12.into());
        addr_reg.update(0x34.into());

        let expected = AddressRegister {
            high: 0x12.into(),
            low: 0x34.into(),
            hi_ptr: true,
        };

        assert_eq!(expected, addr_reg);
        assert_eq!(addr_reg.get(), 0x1234);
    }

    #[test]
    fn multiple_writes_store_only_two_last_values() {
        let mut addr_reg = AddressRegister::default();
        addr_reg.update(0x01.into());
        addr_reg.update(0x12.into());
        addr_reg.update(0x02.into());
        addr_reg.update(0x23.into());
        addr_reg.update(0x2f.into());

        let expected = AddressRegister {
            high: 0x2f.into(),
            low: 0x23.into(),
            hi_ptr: false,
        };

        assert_eq!(expected, addr_reg);
    }

    #[test]
    fn write_with_mirroring() {
        let mut addr_reg = AddressRegister::default();
        addr_reg.update(0x4f.into());

        let expected = AddressRegister {
            high: 0x0f.into(),
            low: 0x00.into(),
            hi_ptr: false,
        };

        assert_eq!(expected, addr_reg);
    }
}
