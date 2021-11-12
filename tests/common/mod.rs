use anyhow::anyhow;
use sabi_nes::cpu::opcodes::OPCODES_MAPPING;
use sabi_nes::{Cpu, Memory, Result};

#[allow(unused)]
pub fn trace(cpu: &Cpu) -> Result<()> {
    let code = cpu.read(cpu.program_counter)?;
    let opcode = OPCODES_MAPPING
        .get(&code)
        .ok_or_else(|| anyhow!("Opcode {:#x} not supported", code))?;

    Ok(())
}
