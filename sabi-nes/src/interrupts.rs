use crate::{Address, Byte};

#[derive(Debug)]
pub struct Interrupt {
    pub vector_addr: Address,
    pub break_flag_mask: Byte,
    pub cpu_cycles: Byte,
}

pub const NMI: Interrupt = Interrupt {
    vector_addr: 0xfffa,
    break_flag_mask: 0b0010_0000,
    cpu_cycles: 2,
};
