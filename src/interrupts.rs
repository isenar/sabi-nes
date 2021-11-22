use crate::{Address, Byte};

#[derive(Debug, Clone, Copy)]
pub enum InterruptType {
    Nmi,
}

#[derive(Debug)]
pub struct Interrupt {
    pub itype: InterruptType,
    pub vector_addr: Address,
    pub break_flag_mask: Byte,
    pub cpu_cycles: u8,
}

pub const NMI: Interrupt = Interrupt {
    itype: InterruptType::Nmi,
    vector_addr: 0xfffa,
    break_flag_mask: 0b0010_0000,
    cpu_cycles: 2,
};
