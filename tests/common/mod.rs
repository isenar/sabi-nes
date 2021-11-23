use anyhow::{anyhow, bail};
use sabi_nes::cpu::opcodes::{Opcode, OPCODES_MAPPING};
use sabi_nes::cpu::AddressingMode;
use sabi_nes::{Address, Cpu, Memory, Result};

pub fn trace(cpu: &mut Cpu) -> Result<String> {
    let code = cpu.read(cpu.program_counter)?;
    let opcode = OPCODES_MAPPING
        .get(&code)
        .ok_or_else(|| anyhow!("Opcode {:#x} not supported", code))?;

    let opcode_hex = opcode_hex_representation(opcode, cpu)?;
    let opcode_asm = match opcode.length() {
        0 => format_zero_arg_instruction(opcode)?,
        1 => format_single_arg_instruction(opcode, cpu)?,
        2 => {
            let address = cpu.read_u16(cpu.program_counter + 1)?;
            format_double_arg_instruction(opcode, address, cpu)?
        }
        _ => unreachable!(),
    };

    Ok(format!(
        "{:>04X}  {:<10}{:<32}A:{:02X} X:{:02X} Y:{:02X} P:{} SP:{}",
        cpu.program_counter,
        opcode_hex,
        opcode_asm,
        cpu.accumulator,
        cpu.register_x,
        cpu.register_y,
        cpu.status_register,
        cpu.stack_pointer,
    ))
}

fn opcode_hex_representation(opcode: &Opcode, cpu: &mut Cpu) -> Result<String> {
    Ok(match opcode.length() {
        0 => format!("{:02X}", opcode.code),
        1 => format!(
            "{:02X} {:02X}",
            opcode.code,
            cpu.read(cpu.program_counter + 1)?
        ),
        2 => match opcode.mode {
            AddressingMode::Implied => {
                format!("{:02X}", opcode.code)
            }
            _ => {
                format!(
                    "{:02X} {:02X} {:02X}",
                    opcode.code,
                    cpu.read(cpu.program_counter + 1)?,
                    cpu.read(cpu.program_counter + 2)?,
                )
            }
        },
        _ => bail!("Unreachable - got opcode with more than 2 args"),
    })
}

fn format_zero_arg_instruction(opcode: &Opcode) -> Result<String> {
    Ok(match opcode.mode {
        AddressingMode::Implied => opcode.name.to_owned(),
        AddressingMode::Accumulator => format!("{} A", opcode.name),
        _ => bail!("Should not occur"),
    })
}

fn format_single_arg_instruction(opcode: &Opcode, cpu: &mut Cpu) -> Result<String> {
    let arg = cpu.read(cpu.program_counter + 1)?;

    let arg = match opcode.mode {
        AddressingMode::Immediate => {
            format!("#${:02X}", arg)
        }
        AddressingMode::ZeroPage => {
            let stored_value = cpu.read(arg.into())?;
            format!("${:02X} = {:02X}", arg, stored_value)
        }
        AddressingMode::ZeroPageX => {
            let incremented = arg.wrapping_add(cpu.register_x);
            let stored_value = cpu.read(incremented.into())?;
            format!(
                "${:02X},X @ {:02X} = {:02X}",
                arg, incremented, stored_value
            )
        }
        AddressingMode::ZeroPageY => {
            let incremented = arg.wrapping_add(cpu.register_y);
            let stored_value = cpu.read(incremented.into())?;
            format!(
                "${:02X},Y @ {:02X} = {:02X}",
                arg, incremented, stored_value
            )
        }
        AddressingMode::IndirectX => {
            let shifted = arg.wrapping_add(cpu.register_x);
            let first = cpu.read(shifted.into())?;
            let second = cpu.read(shifted.wrapping_add(1).into())?;
            let address = Address::from_le_bytes([first, second]);
            let target_cell_value = cpu.read(address)?;

            format!(
                "(${:02X},X) @ {:02X} = {:04X} = {:02X}",
                arg,
                arg.wrapping_add(cpu.register_x),
                address,
                target_cell_value
            )
        }
        AddressingMode::IndirectY => {
            let first = cpu.read(arg.into())?;
            let second = cpu.read(arg.wrapping_add(1).into())?;
            let address = Address::from_le_bytes([first, second]);
            let address_shifted = address.wrapping_add(cpu.register_y.into());
            let target_cell_value = cpu.read(address_shifted)?;

            format!(
                "(${:02X}),Y = {:04X} @ {:04X} = {:02X}",
                arg,
                address,
                address.wrapping_add(cpu.register_y.into()),
                target_cell_value,
            )
        }
        AddressingMode::Relative => {
            let jmp_addr = cpu.program_counter.wrapping_add(2).wrapping_add(arg.into());

            format!("${:04X}", jmp_addr)
        }
        _ => bail!(
            "All single arg variants exhausted. Got addressing mode: {:?}",
            opcode.mode
        ),
    };

    Ok(format!("{} {}", opcode.name, arg))
}

fn format_double_arg_instruction(
    opcode: &Opcode,
    address: Address,
    cpu: &mut Cpu,
) -> Result<String> {
    let value = cpu.read(address)?;
    let arg = match opcode.mode {
        AddressingMode::Absolute => match opcode.name {
            "JMP" | "JSR" => format!("${:04X}", address),
            _ => format!("${:04X} = {:02X}", address, value),
        },
        AddressingMode::AbsoluteX => {
            let incremented = address.wrapping_add(cpu.register_x.into());
            let value = cpu.read(incremented)?;

            format!("${:04X},X @ {:04X} = {:02X}", address, incremented, value)
        }
        AddressingMode::AbsoluteY => {
            let incremented = address.wrapping_add(cpu.register_y.into());
            let value = cpu.read(incremented)?;

            format!("${:04X},Y @ {:04X} = {:02X}", address, incremented, value)
        }
        AddressingMode::Indirect => {
            let x = if address & 0x00ff == 0x00ff {
                let lo = cpu.read(address)? as Address;
                let hi = cpu.read(address & 0xff00)? as Address;
                let addr = hi << 8 | lo;

                addr
            } else {
                cpu.read_u16(address)?
            };

            format!("(${:04X}) = {:04X}", address, x)
        }
        AddressingMode::Implied => String::new(),

        other => bail!("Unreachable - 2 args, got {:?}", other),
    };

    Ok(format!("{} {}", opcode.name, arg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use pretty_assertions::assert_eq;
    use sabi_nes::{Bus, Byte, Rom};

    lazy_static! {
        pub static ref TEST_ROM: Vec<Byte> = {
            let mut rom = vec![];
            let header = vec![
                0x4e, 0x45, 0x53, 0x1a, 0x02, 0x01, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ];
            let prg_rom = vec![0x00; 2 * 16384];
            let chr_rom = vec![0x00; 8192];

            rom.extend(header);
            rom.extend(prg_rom);
            rom.extend(chr_rom);

            rom
        };
    }
    #[test]
    fn trace_format() -> Result<()> {
        let rom = Rom::new(&TEST_ROM)?;
        let mut bus = Bus::new(rom, |_| {});
        bus.write(100, 0xa2)?;
        bus.write(101, 0x01)?;
        bus.write(102, 0xca)?;
        bus.write(103, 0x88)?;
        bus.write(104, 0x00)?;

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
        let mut bus = Bus::new(rom, |_| {});

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
