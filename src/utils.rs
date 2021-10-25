pub trait NthBit {
    fn nth_bit(&self, bit_n: Self) -> bool;
}

impl NthBit for u8 {
    fn nth_bit(&self, bit_n: u8) -> bool {
        self >> bit_n & 1 == 1
    }
}
