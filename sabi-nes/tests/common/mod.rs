use anyhow::anyhow;
use once_cell::sync::Lazy;
use sabi_nes::cartridge::{CHR_ROM_BANK_SIZE, PRG_ROM_BANK_SIZE};
use sabi_nes::cpu::opcodes::{Opcode, OPCODES_MAPPING};
use sabi_nes::cpu::AddressingMode;
use sabi_nes::{Address, Byte, Cpu, Memory, Result};

pub static TEST_ROM: Lazy<Vec<Byte>> = Lazy::new(|| {
    let mut rom = vec![];
    let header = vec![
        0x4e, 0x45, 0x53, 0x1a, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    let prg_rom = vec![0x00; 2 * PRG_ROM_BANK_SIZE];
    let chr_rom = vec![0x00; CHR_ROM_BANK_SIZE];

    rom.extend(header);
    rom.extend(prg_rom);
    rom.extend(chr_rom);

    rom
});

pub fn trace(cpu: &mut Cpu) -> Result<String> {
    let code = cpu.read(cpu.program_counter)?;
    let opcode = OPCODES_MAPPING
        .get(&code)
        .ok_or_else(|| anyhow!("Opcode {code:#x} not supported"))?;
    let opcode_hex = opcode_hex_representation(opcode, cpu)?;
    let opcode_asm = opcode_asm_representation(opcode, cpu)?;
    let pc = cpu.program_counter;
    let acc = cpu.accumulator;
    let reg_x = cpu.register_x;
    let reg_y = cpu.register_y;
    let status = cpu.status_register;
    let sp = cpu.stack_pointer;

    let mut fmt = format!(
        "{pc:>04X}  {opcode_hex:<10}{opcode_asm:<32}A:{acc:02X} X:{reg_x:02X} Y:{reg_y:02X} P:{status} SP:{sp}");

    // TODO: there has to be a better way..
    if opcode.name.starts_with('*') {
        fmt.remove(15);
        fmt.insert(47, ' ');
    }

    Ok(fmt)
}

fn opcode_hex_representation(opcode: &Opcode, cpu: &mut Cpu) -> Result<String> {
    let opc = opcode.code;

    Ok(match opcode.length() {
        0 => format!("{opc:02X}"),
        1 => format!("{opc:02X} {:02X}", cpu.read(cpu.program_counter + 1)?),
        2 => match opcode.addressing_mode {
            AddressingMode::Implied => {
                format!("{opc:02X}")
            }
            _ => {
                format!(
                    "{opc:02X} {:02X} {:02X}",
                    cpu.read(cpu.program_counter + 1)?,
                    cpu.read(cpu.program_counter + 2)?,
                )
            }
        },
        _ => unreachable!("Got opcode with more than 2 args"),
    })
}

fn opcode_asm_representation(opcode: &Opcode, cpu: &mut Cpu) -> Result<String> {
    let value = cpu.read(cpu.program_counter + 1)?;
    let address = cpu.read_u16(cpu.program_counter + 1)?;
    let target_address = cpu.operand_address(opcode, cpu.program_counter + 1)?;

    let opcode_asm_args = match opcode.addressing_mode {
        AddressingMode::Immediate => {
            format!("#${value:02X}")
        }
        AddressingMode::ZeroPage => {
            let stored_value = cpu.read(value.into())?;
            format!("${value:02X} = {stored_value:02X}")
        }
        AddressingMode::ZeroPageX => {
            let stored_value = cpu.read(target_address)?;

            format!("${value:02X},X @ {target_address:02X} = {stored_value:02X}",)
        }
        AddressingMode::ZeroPageY => {
            let stored_value = cpu.read(target_address)?;
            format!("${value:02X},Y @ {target_address:02X} = {stored_value:02X}")
        }
        AddressingMode::Absolute => {
            let stored_value = cpu.read(target_address)?;
            match opcode.name {
                "JMP" | "JSR" => format!("${target_address:04X}"),
                _ => format!("${target_address:04X} = {stored_value:02X}"),
            }
        }
        AddressingMode::AbsoluteX => {
            let incremented = address.wrapping_add(cpu.register_x.into());
            let value = cpu.read(incremented)?;

            format!("${address:04X},X @ {incremented:04X} = {value:02X}")
        }
        AddressingMode::AbsoluteY => {
            let incremented = address.wrapping_add(cpu.register_y.into());
            let value = cpu.read(incremented)?;

            format!("${address:04X},Y @ {incremented:04X} = {value:02X}")
        }
        AddressingMode::IndirectX => {
            let incremented = value.wrapping_add(cpu.register_x);
            let target_cell_value = cpu.read(target_address)?;

            format!(
                "(${value:02X},X) @ {incremented:02X} = {target_address:04X} = {target_cell_value:02X}")
        }
        AddressingMode::IndirectY => {
            let address = target_address.wrapping_sub(cpu.register_y.into());
            let target_cell_value = cpu.read(target_address)?;

            format!(
                "(${value:02X}),Y = {address:04X} @ {target_address:04X} = {target_cell_value:02X}"
            )
        }
        AddressingMode::Implied => String::new(),
        AddressingMode::Accumulator => "A".into(),
        AddressingMode::Relative => {
            let offset = value as i8;
            let jmp_addr = cpu
                .program_counter
                .wrapping_add(2)
                .wrapping_add(offset as Address);

            format!("${jmp_addr:04X}")
        }
        AddressingMode::Indirect => {
            format!("(${address:04X}) = {target_address:04X}")
        }
    };
    let opcode_asm = format!("{} {opcode_asm_args}", opcode.name);

    Ok(opcode_asm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use sabi_nes::{Bus, Rom};

    #[test]
    fn trace_format() -> Result<()> {
        let rom = Rom::new(&TEST_ROM)?;
        let mut bus = Bus::new(rom);
        bus.write(0x64, 0xa2)?;
        bus.write(0x65, 0x01)?;
        bus.write(0x66, 0xca)?;
        bus.write(0x67, 0x88)?;
        bus.write(0x68, 0x00)?;

        let mut cpu = Cpu::new(bus);
        cpu.program_counter = 0x64;
        cpu.accumulator = 1;
        cpu.register_x = 2;
        cpu.register_y = 3;

        let mut traces = vec![];
        cpu.run_with_callback(|cpu| {
            traces.push(trace(cpu)?);
            Ok(())
        })?;
        let expected = vec![
            "0064  A2 01     LDX #$01                        A:01 X:02 Y:03 P:24 SP:FD",
            "0066  CA        DEX                             A:01 X:01 Y:03 P:24 SP:FD",
            "0067  88        DEY                             A:01 X:00 Y:03 P:26 SP:FD",
            "0068  00        BRK                             A:01 X:00 Y:02 P:24 SP:FD",
        ];

        assert_eq!(expected, traces);

        Ok(())
    }

    #[test]
    fn trace_format_mem_access() -> Result<()> {
        let rom = Rom::new(&TEST_ROM)?;
        let mut bus = Bus::new(rom);

        // ORA ($33),Y
        bus.write(0x64, 0x11)?;
        bus.write(0x65, 0x33)?;

        //data
        bus.write(0x33, 0x00)?;
        bus.write(0x34, 0x04)?;

        //target cell
        bus.write(0x0405, 0xaa)?;

        let mut cpu = Cpu::new(bus);
        cpu.program_counter = 0x64;
        cpu.register_y = 5;

        let mut traces = vec![];
        cpu.run_with_callback(|cpu| {
            traces.push(trace(cpu)?);
            Ok(())
        })?;

        assert_eq!(
            "0064  11 33     ORA ($33),Y = 0400 @ 0405 = AA  A:00 X:00 Y:05 P:24 SP:FD",
            traces[0]
        );

        Ok(())
    }
}
