use crate::{Address, Byte};

#[derive(Debug)]
pub struct Interrupt {
    pub vector_addr: Address,
    // TODO: use the mask
    #[allow(unused)]
    pub break_flag_mask: Byte,
    pub cpu_cycles: usize,
}

pub const NMI: Interrupt = Interrupt {
    vector_addr: Address::new(0xfffa),
    break_flag_mask: Byte::new(0b0010_0000),
    cpu_cycles: 2,
};
