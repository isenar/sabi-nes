//! Representation of a 6502 CPU status register (a.k.a. flag register or processor status)
//! It is composed of six one-bit registers. Instructions modify one or more bits and leave others unchanged.
//! Instructions that save or restore the flags map them to bits in the architectural 'P' register as follows:
//!
//! ```text
//! 7  bit  0
//! ---- ----
//! NVss DIZC
//! |||| ||||
//! |||| |||+- Carry
//! |||| ||+-- Zero
//! |||| |+--- Interrupt Disable
//! |||| +---- Decimal
//! ||++------ No CPU effect, see: `the B flag` in the link below
//! |+-------- Overflow
//! +--------- Negative
//! ```
//! - [the B flag](https://wiki.nesdev.org/w/index.php/Status_flags#The_B_flag)

use bitflags::bitflags;

bitflags! {
    pub struct StatusRegister: u8 {
        const CARRY             = 0b0000_0001;
        const ZERO              = 0b0000_0010;
        const INTERRUPT_DISABLE = 0b0000_0100;
        const DECIMAL           = 0b0000_1000;
        const BREAK             = 0b0001_0000;
        const BREAK2            = 0b0010_0000; // not used by NES
        const OVERFLOW          = 0b0100_0000;
        const NEGATIVE          = 0b1000_0000;
    }
}

impl From<u8> for StatusRegister {
    fn from(value: u8) -> Self {
        Self::from_bits_truncate(value)
    }
}

impl StatusRegister {
    pub fn update_zero_and_negative_flags(&mut self, value: impl Into<Self>) {
        let value_bits = value.into();

        self.set_zero_flag(value_bits.is_empty());
        self.set_negative_flag(value_bits.contains(StatusRegister::NEGATIVE));
    }

    #[inline]
    pub fn set_carry_flag(&mut self, value: bool) {
        self.set(StatusRegister::CARRY, value);
    }

    #[inline]
    pub fn set_decimal_flag(&mut self, value: bool) {
        self.set(StatusRegister::DECIMAL, value);
    }

    #[inline]
    pub fn set_interrupt_flag(&mut self, value: bool) {
        self.set(StatusRegister::INTERRUPT_DISABLE, value);
    }

    #[inline]
    pub fn set_overflow_flag(&mut self, value: bool) {
        self.set(StatusRegister::OVERFLOW, value);
    }

    #[inline]
    pub fn set_negative_flag(&mut self, value: bool) {
        self.set(StatusRegister::NEGATIVE, value);
    }

    #[inline]
    pub fn set_zero_flag(&mut self, value: bool) {
        self.set(StatusRegister::ZERO, value);
    }
}
