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

use crate::Byte;
use bitflags::bitflags;
use std::fmt::{Display, Formatter};

bitflags! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct StatusRegister: Byte {
        const CARRY             = 0b0000_0001;
        const ZERO              = 0b0000_0010;
        const INTERRUPT_DISABLE = 0b0000_0100;
        const DECIMAL           = 0b0000_1000;
        const BREAK             = 0b0001_0000;
        const BREAK2            = 0b0010_0000; // not used by NES
        const OVERFLOW          = 0b0100_0000;
        const NEGATIVE          = 0b1000_0000;

        const INIT = StatusRegister::INTERRUPT_DISABLE.bits() | StatusRegister::BREAK2.bits();
    }
}

impl Display for StatusRegister {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:02X}", self.bits())
    }
}

impl From<Byte> for StatusRegister {
    fn from(byte: Byte) -> Self {
        Self::from_bits_truncate(byte)
    }
}

impl StatusRegister {
    pub fn update_zero_and_negative_flags(&mut self, value: impl Into<Self>) -> &mut Self {
        let value_bits = value.into();

        self.set_zero_flag(value_bits.is_empty());
        self.set_negative_flag(value_bits.contains(StatusRegister::NEGATIVE));

        self
    }

    #[inline]
    pub fn set_carry_flag(&mut self, value: bool) -> &mut Self {
        self.set(StatusRegister::CARRY, value);
        self
    }

    #[inline]
    pub fn set_decimal_flag(&mut self, value: bool) -> &mut Self {
        self.set(StatusRegister::DECIMAL, value);
        self
    }

    #[inline]
    pub fn set_interrupt_flag(&mut self, value: bool) -> &mut Self {
        self.set(StatusRegister::INTERRUPT_DISABLE, value);
        self
    }

    #[inline]
    pub fn set_overflow_flag(&mut self, value: bool) -> &mut Self {
        self.set(StatusRegister::OVERFLOW, value);
        self
    }

    #[inline]
    pub fn set_negative_flag(&mut self, value: bool) -> &mut Self {
        self.set(StatusRegister::NEGATIVE, value);
        self
    }

    #[inline]
    pub fn set_zero_flag(&mut self, value: bool) -> &mut Self {
        self.set(StatusRegister::ZERO, value);
        self
    }

    #[inline]
    pub fn disable_interrupt(&mut self) -> &mut Self {
        self.remove(Self::INTERRUPT_DISABLE);
        self
    }
}
