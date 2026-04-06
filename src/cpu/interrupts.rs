use crate::{Address, Byte};

#[derive(Debug)]
pub struct Interrupt {
    pub vector_addr: Address,
    pub break_flag_mask: Byte,
    pub cpu_cycles: usize,
}

pub const NMI: Interrupt = Interrupt {
    vector_addr: Address::new(0xfffa),
    break_flag_mask: Byte::new(0b0010_0000),
    cpu_cycles: 7,
};

pub const BRK: Interrupt = Interrupt {
    vector_addr: Address::new(0xfffe),
    break_flag_mask: Byte::new(0b0011_0000),
    cpu_cycles: 6, // 7 total: 1 opcode fetch (in step()) + 6 here
};

pub const IRQ: Interrupt = Interrupt {
    vector_addr: Address::new(0xfffe),
    break_flag_mask: Byte::new(0b0010_0000),
    cpu_cycles: 7,
};
