use crate::{Address, Byte};

pub trait NthBit {
    fn nth_bit(&self, bit_n: Self) -> bool;
}

pub trait MirroredAddress {
    fn mirror_cpu_vram_addr(&self) -> Self;
    fn mirror_ppu_addr(&self) -> Self;
}

impl NthBit for u8 {
    #[inline]
    fn nth_bit(&self, bit_n: u8) -> bool {
        self >> bit_n & 1 == 1
    }
}

impl MirroredAddress for Address {
    fn mirror_cpu_vram_addr(&self) -> Self {
        self & 0b0000_0111_1111_1111
    }

    fn mirror_ppu_addr(&self) -> Self {
        self & 0b0010_1111_1111_1111
    }
}

pub fn shift_right(value: Byte) -> u8 {
    value >> 1
}

pub fn shift_left(value: Byte) -> u8 {
    value << 1
}