mod addressing_mode;
mod opcodes;
pub mod status_register;

use crate::cpu::addressing_mode::AddressingMode;
use crate::cpu::opcodes::OPCODES_MAPPING;
use crate::cpu::status_register::StatusRegister;
use anyhow::{anyhow, bail, Result};

type Register = u8;
type Address = u16;
type ProgramCounter = Address;
type Value = u8;

const PROGRAM_ROM_BEGIN_ADDR: Address = 0x8000;
const PROGRAM_ROM_END_ADDR: Address = 0xffff;
const RESET_VECTOR_BEGIN_ADDR: Address = 0xfffc;

#[derive(Debug)]
pub struct Cpu {
    pub register_a: Register,
    pub register_x: Register,
    pub register_y: Register,
    pub status_register: StatusRegister,
    pub program_counter: ProgramCounter,

    memory: [Value; PROGRAM_ROM_END_ADDR as usize],
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status_register: StatusRegister::empty(),
            program_counter: 0,
            memory: [0; 0xffff],
        }
    }
}

impl Cpu {
    pub fn mem_read(&self, addr: u16) -> Value {
        self.memory[addr as usize]
    }

    pub fn mem_write(&mut self, addr: Address, value: Value) {
        self.memory[addr as usize] = value;
    }

    pub fn load_and_run(&mut self, data: &[Value]) -> Result<()> {
        self.load(data);
        self.reset();
        self.run()?;

        Ok(())
    }

    pub fn load(&mut self, data: &[Value]) {
        self.memory
            [PROGRAM_ROM_BEGIN_ADDR as usize..(PROGRAM_ROM_BEGIN_ADDR as usize + data.len())]
            .copy_from_slice(data);
        self.mem_write_u16(RESET_VECTOR_BEGIN_ADDR, PROGRAM_ROM_BEGIN_ADDR);
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;

            let opcode = OPCODES_MAPPING
                .get(&code)
                .ok_or_else(|| anyhow!("Unknown opcode: {}", code))?;

            match opcode.name {
                "AND" => {
                    self.and(opcode.mode);
                    self.program_counter += opcode.len();
                }
                "BRK" => return Ok(()),
                "CLC" => self.status_register.clear_carry_flag(),
                "CLD" => self.status_register.clear_decimal_flag(),
                "CLI" => self.status_register.clear_interrupt_flag(),
                "CLV" => self.status_register.clear_overflow_flag(),
                "INX" => self.inx(),
                "INY" => self.iny(),
                "LDA" => self.lda(opcode.mode),
                "LDX" => self.ldx(opcode.mode),
                "SEC" => self.status_register.set_carry_flag(),
                "SED" => self.status_register.set_decimal_flag(),
                "SEI" => self.status_register.set_interrupt_flag(),
                "STA" => self.sta(opcode.mode),
                "TAX" => self.tax(),

                _ => bail!("Unsupported opcode name: {}", opcode.name),
            }

            self.program_counter += opcode.len();
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status_register = StatusRegister::empty();
        self.program_counter = self.mem_read_u16(RESET_VECTOR_BEGIN_ADDR);
    }

    fn and(&mut self, mode: AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a &= value;
    }

    fn lda(&mut self, mode: AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.status_register
            .update_zero_and_negative_flags(self.register_a);
    }

    fn ldx(&mut self, mode: AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_x = value;
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
    }

    fn sta(&mut self, mode: AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    fn mem_read_u16(&self, addr: Address) -> u16 {
        let lo = self.mem_read(addr);
        let hi = self.mem_read(addr + 1);

        u16::from_le_bytes([lo, hi])
    }

    fn mem_write_u16(&mut self, addr: Address, data: u16) {
        let [lo, hi] = data.to_le_bytes();

        self.mem_write(addr, lo);
        self.mem_write(addr + 1, hi);
    }

    fn get_operand_address(&self, mode: AddressingMode) -> Address {
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
            AddressingMode::Implied => {
                unreachable!("Implied mode is never passed to get operand address")
            }
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
            assert!(!cpu.status_register.contains(StatusRegister::ZERO));
            assert!(!cpu.status_register.contains(StatusRegister::NEGATIVE));
        }

        #[test]
        fn zero_flag_set() {
            let mut cpu = Cpu::default();
            let data = [0xa9, 0x00, 0x00];

            cpu.load_and_run(&data).expect("Failed to load and run");

            assert!(cpu.status_register.contains(StatusRegister::ZERO));
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

    mod flags {
        use super::*;

        #[test]
        fn carry_flag_enabled() {
            let mut cpu = Cpu::default();
            let data = [0x38, 0x00];

            cpu.load_and_run(&data).expect("Failed to load and run");

            assert_eq!(cpu.status_register, StatusRegister::CARRY);
        }

        #[test]
        fn decimal_flag_enabled() {
            let mut cpu = Cpu::default();
            let data = [0xf8, 0x00];

            cpu.load_and_run(&data).expect("Failed to load and run");

            assert_eq!(cpu.status_register, StatusRegister::DECIMAL);
        }

        #[test]
        fn interrupt_flag_enabled() {
            let mut cpu = Cpu::default();
            let data = [0x78, 0x00];

            cpu.load_and_run(&data).expect("Failed to load and run");

            assert_eq!(cpu.status_register, StatusRegister::INTERRUPT_DISABLE);
        }
    }
}
