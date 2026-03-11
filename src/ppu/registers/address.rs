use crate::utils::MirroredAddress;
use crate::{Address, Byte, Word};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressRegister {
    high: Byte,
    low: Byte,
    is_high: bool,
}

impl Default for AddressRegister {
    fn default() -> Self {
        Self {
            high: Byte::default(),
            low: Byte::default(),
            is_high: true,
        }
    }
}

impl AddressRegister {
    pub fn update(&mut self, value: Byte) {
        if self.is_high {
            self.high = value;
        } else {
            self.low = value;
        }
        self.is_high = !self.is_high;

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
        Word::from_le_bytes(self.low, self.high).as_address()
    }

    pub fn reset_latch(&mut self) {
        self.is_high = true;
    }

    fn set(&mut self, word: Word) {
        self.low = Byte::new((word.value() & 0xff) as u8);
        self.high = Byte::new((word.value() >> 8) as u8);
    }

    fn mirror_down(&mut self) {
        if self.get() > 0x3fff {
            self.set(self.get().mirror_ppu_addr().as_word());
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
            is_high: false,
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
            is_high: true,
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
            is_high: false,
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
            is_high: false,
        };

        assert_eq!(expected, addr_reg);
    }
}
