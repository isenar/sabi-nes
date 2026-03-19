use anyhow::anyhow;
use once_cell::sync::Lazy;
use sabi_nes::cartridge::{CHR_ROM_BANK_SIZE, PRG_ROM_BANK_SIZE};
use sabi_nes::cpu::AddressingMode;
use sabi_nes::cpu::opcodes::{OPCODES_MAPPING, Opcode};
use sabi_nes::{Address, Cpu, Memory, Result};

pub static TEST_ROM: Lazy<Vec<u8>> = Lazy::new(|| {
    let mut rom = vec![];
    let header = [
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
    let code = cpu.read_byte(cpu.program_counter)?;
    let opcode = OPCODES_MAPPING
        .get(&code)
        .ok_or_else(|| anyhow!("Opcode `{code:#x}` not supported"))?;
    let opcode_hex = opcode_hex_representation(opcode, cpu)?;
    let opcode_asm = opcode_asm_representation(opcode, cpu)?;

    // Unofficial opcodes (starting with '*') use a 9-char hex field and 33-char ASM field.
    // This keeps the register columns aligned despite the extra '*' character.
    //   Official:   "C5F7  86 00     STX $00 = 00                    A:00 X:00..."
    //   Unofficial: "C6BD  04 A9    *NOP $A9 = 00                    A:AA X:97..."
    let (hex_width, asm_width) = if opcode.name.starts_with('*') {
        (9, 33)
    } else {
        (10, 32)
    };

    Ok(format!(
        "{:>04X}  {:<hex_width$}{:<asm_width$}A:{:02X} X:{:02X} Y:{:02X} P:{} SP:{}",
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
            cpu.read_byte(cpu.program_counter + 1)?
        ),
        2 => match opcode.addressing_mode {
            AddressingMode::Implied => {
                format!("{:02X}", opcode.code)
            }
            _ => {
                format!(
                    "{:02X} {:02X} {:02X}",
                    opcode.code,
                    cpu.read_byte(cpu.program_counter + 1)?,
                    cpu.read_byte(cpu.program_counter + 2)?,
                )
            }
        },
        _ => unreachable!("Got opcode with more than 2 args"),
    })
}

fn opcode_asm_representation(opcode: &Opcode, cpu: &mut Cpu) -> Result<String> {
    let value = cpu.read_byte(cpu.program_counter + 1)?;
    let address = cpu
        .read_word(cpu.program_counter + 1)?
        .as_address();
    let target_address = cpu.operand_address(opcode, cpu.program_counter + 1)?;

    let opcode_asm_args = match opcode.addressing_mode {
        AddressingMode::Immediate => {
            format!("#${:02X}", value)
        }
        AddressingMode::ZeroPage => {
            let stored_value = cpu.read_byte(value.into())?;
            format!("${:02X} = {:02X}", value, stored_value)
        }
        AddressingMode::ZeroPageX => {
            let stored_value = cpu.read_byte(target_address)?;

            format!(
                "${:02X},X @ {:02X} = {:02X}",
                value, target_address, stored_value
            )
        }
        AddressingMode::ZeroPageY => {
            let stored_value = cpu.read_byte(target_address)?;
            format!(
                "${:02X},Y @ {:02X} = {:02X}",
                value, target_address, stored_value
            )
        }
        AddressingMode::Absolute => {
            let stored_value = cpu.read_byte(target_address)?;
            match opcode.name {
                "JMP" | "JSR" => format!("${:04X}", target_address),
                _ => format!("${:04X} = {:02X}", target_address, stored_value),
            }
        }
        AddressingMode::AbsoluteX => {
            let incremented = address.wrapping_add(cpu.register_x.value());
            let value = cpu.read_byte(incremented)?;

            format!("${:04X},X @ {:04X} = {:02X}", address, incremented, value)
        }
        AddressingMode::AbsoluteY => {
            let incremented = address.wrapping_add(cpu.register_y.value());
            let value = cpu.read_byte(incremented)?;

            format!("${:04X},Y @ {:04X} = {:02X}", address, incremented, value)
        }
        AddressingMode::IndirectX => {
            let incremented = value.wrapping_add(cpu.register_x.value());
            let target_cell_value = cpu.read_byte(target_address)?;

            format!(
                "(${:02X},X) @ {:02X} = {:04X} = {:02X}",
                value, incremented, target_address, target_cell_value
            )
        }
        AddressingMode::IndirectY => {
            let address = target_address.wrapping_sub(cpu.register_y.value());
            let target_cell_value = cpu.read_byte(target_address)?;

            format!(
                "(${:02X}),Y = {:04X} @ {:04X} = {:02X}",
                value, address, target_address, target_cell_value,
            )
        }
        AddressingMode::Implied => String::new(),
        AddressingMode::Accumulator => "A".into(),
        AddressingMode::Relative => {
            let offset = value.value().cast_signed();
            #[allow(clippy::cast_sign_loss)]
            let jump_address = cpu
                .program_counter
                .wrapping_add(2u16)
                // NOTE: This is a quirky behaviour of what the NES CPU does
                //       for branching instructions, and it's intended here.
                .wrapping_add(offset as u16);

            format!("${jump_address:04X}")
        }
        AddressingMode::Indirect => {
            format!("(${address:04X}) = {target_address:04X}")
        }
    };

    Ok(format!("{} {opcode_asm_args}", opcode.name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use sabi_nes::{Bus, Rom};

    #[test]
    fn trace_format() -> Result<()> {
        let rom = Rom::from_bytes(&TEST_ROM)?;
        let mut bus = Bus::new(rom);
        bus.write_byte(Address::new(0x64), 0xa2.into())?;
        bus.write_byte(Address::new(0x65), 0x01.into())?;
        bus.write_byte(Address::new(0x66), 0xca.into())?;
        bus.write_byte(Address::new(0x67), 0x88.into())?;
        bus.write_byte(Address::new(0x68), 0x00.into())?;

        let mut cpu = Cpu::new(bus);
        cpu.program_counter = Address::new(0x64);
        cpu.accumulator = 1.into();
        cpu.register_x = 2.into();
        cpu.register_y = 3.into();

        let mut traces = vec![];
        loop {
            traces.push(trace(&mut cpu)?);
            if cpu.step()? {
                break; // BRK encountered
            }
        }
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
        let rom = Rom::from_bytes(&TEST_ROM)?;
        let mut bus = Bus::new(rom);

        // ORA ($33),Y
        bus.write_byte(Address::new(0x64), 0x11.into())?;
        bus.write_byte(Address::new(0x65), 0x33.into())?;

        //data
        bus.write_byte(Address::new(0x33), 0x00.into())?;
        bus.write_byte(Address::new(0x34), 0x04.into())?;

        //target cell
        bus.write_byte(Address::new(0x0405), 0xaa.into())?;

        let mut cpu = Cpu::new(bus);
        cpu.program_counter = Address::new(0x64);
        cpu.register_y = 0x05.into();

        let mut traces = vec![];
        loop {
            traces.push(trace(&mut cpu)?);
            if cpu.step()? {
                break;
            }
        }

        assert_eq!(
            "0064  11 33     ORA ($33),Y = 0400 @ 0405 = AA  A:00 X:00 Y:05 P:24 SP:FD",
            traces[0]
        );

        Ok(())
    }
}
