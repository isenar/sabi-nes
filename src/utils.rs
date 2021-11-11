use crate::Byte;

pub trait NthBit {
    fn nth_bit(&self, bit_n: Self) -> bool;
}

impl NthBit for u8 {
    #[inline]
    fn nth_bit(&self, bit_n: u8) -> bool {
        self >> bit_n & 1 == 1
    }
}

pub fn shift_right(value: Byte) -> u8 {
    value >> 1
}

pub fn shift_left(value: Byte) -> u8 {
    value << 1
}
