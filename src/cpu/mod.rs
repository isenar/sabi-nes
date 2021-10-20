mod addressing_mode;
mod opcodes;

use crate::cpu::opcodes::OPCODES_MAPPING;
use addressing_mode::AddressingMode;
use anyhow::{anyhow, Result};

const PROGRAM_ROM_BEGIN_ADDR: usize = 0x8000;

#[derive(Debug)]
pub struct Cpu {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,

    memory: [u8; 0xffff],
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0,
            program_counter: 0,
            memory: [0; 0xffff],
        }
    }
}

impl Cpu {
    pub fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    pub fn mem_write(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value;
    }

    pub fn load_and_run(&mut self, data: &[u8]) -> Result<()> {
        self.load(data);
        self.reset();
        self.run()?;

        Ok(())
    }

    pub fn load(&mut self, data: &[u8]) {
        self.memory[PROGRAM_ROM_BEGIN_ADDR..(PROGRAM_ROM_BEGIN_ADDR + data.len())]
            .copy_from_slice(data);
        self.mem_write_u16(0xFFFC, PROGRAM_ROM_BEGIN_ADDR as u16);
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;

            let opcode = OPCODES_MAPPING
                .get(&code)
                .ok_or_else(|| anyhow!("Unknown opcode: {}", code))?;

            match opcode.name {
                "BRK" => {
                    return Ok(());
                }
                "TAX" => self.tax(),
                "INX" => self.inx(),
                "LDA" => {
                    self.lda(opcode.mode);
                    self.program_counter += opcode.bytes as u16 - 1;
                }
                "STA" => {
                    self.sta(opcode.mode);
                    self.program_counter += opcode.bytes as u16 - 1;
                }
                _ => todo!("Unsupported opcode name: {}", opcode.name),
            }
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = 0;

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    fn lda(&mut self, mode: AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
    }

    fn sta(&mut self, mode: AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.status |= 0b0000_0010;
        } else {
            self.status &= 0b1111_1101;
        }

        if result & 0b1000_0000 != 0 {
            self.status |= 0b1000_0000;
        } else {
            self.status &= 0b0111_1111;
        }
    }

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos);
        let hi = self.mem_read(pos + 1);

        u16::from_le_bytes([lo, hi])
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }

    fn get_operand_address(&self, mode: AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::ZeroPage => self.mem_read(self.program_counter).into(),
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
            AddressingMode::ZeroPageX => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_x);

                addr.into()
            }

            AddressingMode::ZeroPageY => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_y);

                addr.into()
            }
            AddressingMode::AbsoluteX => {
                let base = self.mem_read_u16(self.program_counter);
                base.wrapping_add(self.register_x.into())
            }
            AddressingMode::AbsoluteY => {
                let base = self.mem_read_u16(self.program_counter);
                base.wrapping_add(self.register_y.into())
            }
            AddressingMode::IndirectX => {
                let base = self.mem_read(self.program_counter);
                let ptr = base.wrapping_add(self.register_x);
                let lo = self.mem_read(ptr.into());
                let hi = self.mem_read(ptr.wrapping_add(1).into());

                u16::from_le_bytes([lo, hi])
            }
            AddressingMode::IndirectY => {
                let base = self.mem_read(self.program_counter);
                let lo = self.mem_read(base.into());
                let hi = self.mem_read(base.wrapping_add(1).into());
                let deref_base = u16::from_le_bytes([lo, hi]);

                deref_base.wrapping_add(self.register_y as u16)
            }
            AddressingMode::Implied => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod lda {
        use super::*;

        #[test]
        fn immediate_load() {
            let mut cpu = Cpu::default();
            let data = [0xa9, 0x05, 0x00];

            cpu.load_and_run(&data).expect("Failed to load and run");

            assert_eq!(cpu.register_a, 0x05);
            assert_eq!(cpu.status & 0b000_0010, 0b00);
            assert_eq!(cpu.status & 0b1000_0000, 0b00);
        }

        #[test]
        fn zero_flag() {
            let mut cpu = Cpu::default();
            let data = [0xa9, 0x00, 0x00];

            cpu.load_and_run(&data).expect("Failed to load and run");

            assert_eq!(cpu.status & 0b000_0010, 0b10);
        }

        #[test]
        fn load_from_memory() {
            let mut cpu = Cpu::default();
            let data = [0xa5, 0x10, 0x00];

            cpu.mem_write(0x10, 0x55);
            cpu.load_and_run(&data).expect("Failed to load and run");

            assert_eq!(cpu.register_a, 0x55);
        }
    }

    mod tax {
        use super::*;

        #[test]
        fn moves_reg_a_value_to_reg_x() {
            let mut cpu = Cpu::default();
            let data = [0xa9, 0x0a, 0xaa, 0x00];

            cpu.load_and_run(&data).expect("Failed to load and run");

            assert_eq!(cpu.register_a, 10);
            assert_eq!(cpu.register_x, 10);
        }
    }

    mod inx {
        use super::*;

        #[test]
        fn inx_overflow() {
            let mut cpu = Cpu::default();
            let data = [0xa9, 0xff, 0xaa, 0xe8, 0xe8, 0x00];

            cpu.load_and_run(&data).expect("Failed to load and run");

            assert_eq!(cpu.register_x, 1);
        }
    }

    mod mixed {
        use super::*;

        #[test]
        fn simple_5_ops_working_together() {
            let mut cpu = Cpu::default();
            let data = [0xa9, 0xc0, 0xaa, 0xe8, 0x00];

            cpu.load_and_run(&data).expect("Failed to load and run");

            assert_eq!(cpu.register_x, 0xc1);
        }
    }
}
