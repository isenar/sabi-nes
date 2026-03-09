use crate::{Address, Byte};

pub trait NthBit {
    fn nth_bit<const N: usize>(self) -> bool
    where
        ConstBit<N>: AllowedBit;
}

pub trait MirroredAddress {
    fn mirror_cpu_vram_addr(&self) -> Self;
    fn mirror_ppu_addr(&self) -> Self;
}

pub trait AllowedBit {}

pub struct ConstBit<const N: usize>;

impl AllowedBit for ConstBit<0> {}
impl AllowedBit for ConstBit<1> {}
impl AllowedBit for ConstBit<2> {}
impl AllowedBit for ConstBit<3> {}
impl AllowedBit for ConstBit<4> {}
impl AllowedBit for ConstBit<5> {}
impl AllowedBit for ConstBit<6> {}
impl AllowedBit for ConstBit<7> {}

impl NthBit for Byte {
    #[inline]
    fn nth_bit<const N: usize>(self) -> bool {
        (self >> N) & 1 == 1
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

pub const fn shift_right(value: Byte) -> Byte {
    value >> 1
}

pub const fn shift_left(value: Byte) -> Byte {
    value << 1
}
