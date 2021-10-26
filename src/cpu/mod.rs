mod addressing_mode;
mod opcodes;
mod stack_pointer;
mod status_register;

use crate::cpu::addressing_mode::AddressingMode;
use crate::cpu::opcodes::OPCODES_MAPPING;
use crate::cpu::stack_pointer::StackPointer;
use crate::cpu::status_register::StatusRegister;
use crate::utils::NthBit;
use anyhow::{anyhow, bail, Context, Result};

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
    pub stack_pointer: StackPointer,

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
            stack_pointer: StackPointer::default(),
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
                "AND" => self.and(opcode.mode)?,
                "ASL" => self.asl(opcode.mode)?,
                "BRK" => return Ok(()),
                "CLC" => self.status_register.clear_carry_flag(),
                "CLD" => self.status_register.clear_decimal_flag(),
                "CLI" => self.status_register.clear_interrupt_flag(),
                "CLV" => self.status_register.clear_overflow_flag(),
                "DEC" => self.dec(opcode.mode)?,
                "DEX" => self.dex(),
                "DEY" => self.dey(),
                "INC" => self.inc(opcode.mode)?,
                "INX" => self.inx(),
                "INY" => self.iny(),
                "LDA" => self.lda(opcode.mode)?,
                "LDX" => self.ldx(opcode.mode)?,
                "LDY" => self.ldy(opcode.mode)?,
                "LSR" => self.lsr(opcode.mode)?,
                "PHA" => self.stack_push(self.register_a),
                "PHP" => self.php(),
                "PLA" => self.pla(),
                "PLP" => self.plp(),
                "ROL" => self.rol(opcode.mode)?,
                "SEC" => self.status_register.set_carry_flag(),
                "SED" => self.status_register.set_decimal_flag(),
                "SEI" => self.status_register.set_interrupt_flag(),
                "STA" => self.sta(opcode.mode)?,
                "STX" => self.stx(opcode.mode)?,
                "STY" => self.sty(opcode.mode)?,
                "TAX" => self.tax(),
                "TAY" => self.tay(),
                "TSX" => self.tsx(),
                "TXA" => self.txa(),
                "TXS" => self.stack_pointer.set(self.register_x),
                "TYA" => self.tya(),
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
        self.stack_pointer.reset();
    }

    fn and(&mut self, mode: AddressingMode) -> Result<()> {
        let addr = self
            .get_operand_address(mode)
            .ok_or_else(|| anyhow!("Could not fetch address for {} in AND instruction"))?;
        let value = self.mem_read(addr);

        self.register_a &= value;

        Ok(())
    }

    fn asl(&mut self, mode: AddressingMode) -> Result<()> {
        let addr = self.get_operand_address(mode);
        let shifted = match addr {
            Some(addr) => {
                let value = self.mem_read(addr);
                let shifted_left = value << 1;

                self.mem_write(addr, shifted_left);
                self.status_register
                    .set(StatusRegister::CARRY, value.nth_bit(7));

                shifted_left
            }
            None => {
                let old_reg_a = self.register_a;
                self.register_a <<= 1;
                self.status_register
                    .set(StatusRegister::CARRY, old_reg_a.nth_bit(7));

                self.register_a
            }
        };

        self.status_register.update_zero_and_negative_flags(shifted);

        Ok(())
    }

    fn lsr(&mut self, mode: AddressingMode) -> Result<()> {
        let addr = self.get_operand_address(mode);
        let shifted = match addr {
            Some(addr) => {
                let value = self.mem_read(addr);
                let shifted = value >> 1;

                self.mem_write(addr, shifted);
                self.status_register
                    .set(StatusRegister::CARRY, value.nth_bit(0));

                shifted
            }
            None => {
                let old_reg_a = self.register_a;
                self.register_a >>= 1;
                self.status_register
                    .set(StatusRegister::CARRY, old_reg_a.nth_bit(0));
                self.register_a
            }
        };

        // "The N flag is always reset"
        self.status_register.clear_negative_flag();

        self.status_register.update_zero_and_negative_flags(shifted);

        Ok(())
    }

    fn rol(&mut self, mode: AddressingMode) -> Result<()> {
        // rotate 1 bit left with input carry being stored at bit 0
        // and

        let address = self.get_operand_address(mode);
        let shifted = match address {
            Some(addr) => {
                let value = self.mem_read(addr);
                let shifted = (value << 1) | value.nth_bit(7) as u8;
                self.mem_write(addr, shifted);

                self.status_register
                    .set(StatusRegister::CARRY, value.nth_bit(7));

                shifted
            }
            None => {
                let prev_acc_bits = self.register_a;
                self.register_a = (self.register_a << 1) | prev_acc_bits.nth_bit(7) as u8;

                self.status_register
                    .set(StatusRegister::CARRY, prev_acc_bits.nth_bit(7));

                self.register_a
            }
        };

        self.status_register.update_zero_and_negative_flags(shifted);

        Ok(())
    }

    fn lda(&mut self, mode: AddressingMode) -> Result<()> {
        self.register_a = self.load_value(mode)?;

        Ok(())
    }

    fn ldx(&mut self, mode: AddressingMode) -> Result<()> {
        self.register_x = self.load_value(mode)?;

        Ok(())
    }

    fn ldy(&mut self, mode: AddressingMode) -> Result<()> {
        self.register_y = self.load_value(mode).with_context(|| "In LDY")?;

        Ok(())
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.status_register
            .update_zero_and_negative_flags(self.register_y);
    }

    fn tsx(&mut self) {
        self.register_x = self.stack_pointer.value();
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn txa(&mut self) {
        self.register_a = self.register_x;
        self.status_register
            .update_zero_and_negative_flags(self.register_a);
    }

    fn tya(&mut self) {
        self.register_a = self.register_y;
        self.status_register
            .update_zero_and_negative_flags(self.register_a);
    }

    fn dec(&mut self, mode: AddressingMode) -> Result<()> {
        let addr = self
            .get_operand_address(mode)
            .ok_or_else(|| anyhow!("Could not fetch address for {} in DEC instruction"))?;

        let dec_value = self.mem_read(addr).wrapping_sub(1);

        self.mem_write(addr, dec_value);
        self.status_register
            .update_zero_and_negative_flags(dec_value);

        Ok(())
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.status_register
            .update_zero_and_negative_flags(self.register_y);
    }

    fn inc(&mut self, mode: AddressingMode) -> Result<()> {
        let addr = self
            .get_operand_address(mode)
            .ok_or_else(|| anyhow!("Could not fetch address for {} in INC instruction"))?;

        let inc_value = self.mem_read(addr).wrapping_add(1);

        self.mem_write(addr, inc_value);
        self.status_register
            .update_zero_and_negative_flags(inc_value);

        Ok(())
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
    }

    fn pla(&mut self) {
        let value = self.pop_stack();

        self.register_a = value;
        self.status_register
            .update_zero_and_negative_flags(self.register_a);
    }

    fn plp(&mut self) {
        let value = self.pop_stack();

        self.status_register = StatusRegister::from_bits_truncate(value);
    }

    fn php(&mut self) {
        let mut status_register_with_b_flags = self.status_register;
        status_register_with_b_flags.insert(StatusRegister::BREAK | StatusRegister::BREAK2);

        self.stack_push(status_register_with_b_flags.bits());
    }

    fn sta(&mut self, mode: AddressingMode) -> Result<()> {
        self.store_value(self.register_a, mode)
    }

    fn stx(&mut self, mode: AddressingMode) -> Result<()> {
        self.store_value(self.register_x, mode)
    }

    fn sty(&mut self, mode: AddressingMode) -> Result<()> {
        self.store_value(self.register_y, mode)
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

    fn get_operand_address(&self, mode: AddressingMode) -> Option<Address> {
        Some(match mode {
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
            AddressingMode::Accumulator | AddressingMode::Implied => return None,
        })
    }

    fn load_value(&mut self, mode: AddressingMode) -> Result<Value> {
        let addr = self.get_operand_address(mode).ok_or_else(|| {
            anyhow!(
                "Could not get operand address when loading value ({:?})",
                mode
            )
        })?;
        let value = self.mem_read(addr);

        self.status_register.update_zero_and_negative_flags(value);

        Ok(value)
    }

    fn store_value(&mut self, value: Value, mode: AddressingMode) -> Result<()> {
        let addr = self
            .get_operand_address(mode)
            .ok_or_else(|| anyhow!("Could not fetch address when storing value ({:?}", mode))?;

        self.mem_write(addr, value);

        Ok(())
    }

    fn stack_push(&mut self, value: Value) {
        self.mem_write(self.stack_pointer.address(), value);

        self.stack_pointer.decrement();
    }

    fn pop_stack(&mut self) -> Value {
        self.stack_pointer.increment();
        self.mem_read(self.stack_pointer.address())
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
