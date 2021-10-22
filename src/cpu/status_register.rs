//! Representation of a 6502 CPU status register (a.k.a. flag register or processor status)
//! It is composed of six one-bit registers. Instructions modify one or more bits and leave others unchanged.
//! Instructions that save or restore the flags map them to bits in the architectural 'P' register as follows:
//! ```text
//! 7  bit  0
//! ---- ----
//! NVss DIZC
//! |||| ||||
//! |||| |||+- Carry
//! |||| ||+-- Zero
//! |||| |+--- Interrupt Disable
//! |||| +---- Decimal
//! ||++------ No CPU effect, see: the B flag
//! |+-------- Overflow
//! +--------- Negative
//! ```

use bitflags::bitflags;

bitflags! {
    pub struct StatusRegister: u8 {
        const CARRY             = 0b0000_0001;
        const ZERO              = 0b0000_0010;
        const INTERRUPT_DISABLE = 0b0000_0100;
        const DECIMAL           = 0b0000_1000;
        const BREAK             = 0b0001_0000;
        const BREAK2            = 0b0010_0000;
        const OVERFLOW          = 0b0100_0000;
        const NEGATIVE          = 0b1000_0000;
    }
}

impl StatusRegister {
    pub fn update_zero_and_negative_flags(&mut self, value: u8) {
        let value_bits = StatusRegister::from_bits_truncate(value);

        self.set(StatusRegister::ZERO, value_bits.is_empty());
        self.set(
            StatusRegister::NEGATIVE,
            value_bits.contains(StatusRegister::NEGATIVE),
        );
    }
}
