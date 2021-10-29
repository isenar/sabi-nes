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
    accumulator: Register,
    register_x: Register,
    register_y: Register,
    status_register: StatusRegister,
    program_counter: ProgramCounter,
    stack_pointer: StackPointer,
    memory: [Value; PROGRAM_ROM_END_ADDR as usize],
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            accumulator: 0,
            register_x: 0,
            register_y: 0,
            status_register: StatusRegister::empty(),
            program_counter: 0,
            stack_pointer: StackPointer::new(),
            memory: [0; 0xffff],
        }
    }
}

impl Cpu {
    pub fn mem_read(&self, addr: Address) -> Value {
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
                "ADC" => self.adc(opcode.mode)?,
                "AND" => self.and(opcode.mode)?,
                "ASL" => self.asl(opcode.mode)?,
                "BIT" => self.bit(opcode.mode)?,
                "BCC" => self.branch(!self.status_register.contains(StatusRegister::CARRY)),
                "BCS" => self.branch(self.status_register.contains(StatusRegister::CARRY)),
                "BEQ" => self.branch(self.status_register.contains(StatusRegister::ZERO)),
                "BMI" => self.branch(self.status_register.contains(StatusRegister::NEGATIVE)),
                "BNE" => self.branch(!self.status_register.contains(StatusRegister::ZERO)),
                "BPL" => self.branch(!self.status_register.contains(StatusRegister::NEGATIVE)),
                "BVC" => self.branch(!self.status_register.contains(StatusRegister::OVERFLOW)),
                "BVS" => self.branch(self.status_register.contains(StatusRegister::OVERFLOW)),
                "BRK" => return Ok(()),
                "CLC" => self.status_register.set_carry_flag(false),
                "CLD" => self.status_register.set_decimal_flag(false),
                "CLI" => self.status_register.set_interrupt_flag(false),
                "CLV" => self.status_register.set_overflow_flag(false),
                "CMP" => self.compare(opcode.mode, self.accumulator)?,
                "CPX" => self.compare(opcode.mode, self.register_x)?,
                "CPY" => self.compare(opcode.mode, self.register_y)?,
                "DEC" => self.dec(opcode.mode)?,
                "DEX" => self.dex(),
                "DEY" => self.dey(),
                "EOR" => self.eor(opcode.mode)?,
                "INC" => self.inc(opcode.mode)?,
                "INX" => self.inx(),
                "INY" => self.iny(),
                "LDA" => self.lda(opcode.mode)?,
                "LDX" => self.ldx(opcode.mode)?,
                "LDY" => self.ldy(opcode.mode)?,
                "LSR" => self.lsr(opcode.mode)?,
                "NOP" => {}
                "ORA" => self.ora(opcode.mode)?,
                "PHA" => self.stack_push(self.accumulator),
                "PHP" => self.php(),
                "PLA" => self.pla(),
                "PLP" => self.plp(),
                "ROL" => self.rol(opcode.mode),
                "ROR" => self.ror(opcode.mode),
                "SBC" => self.sbc(opcode.mode)?,
                "SEC" => self.status_register.set_carry_flag(true),
                "SED" => self.status_register.set_decimal_flag(true),
                "SEI" => self.status_register.set_interrupt_flag(true),
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
        self.accumulator = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status_register = StatusRegister::empty();
        self.program_counter = self.mem_read_u16(RESET_VECTOR_BEGIN_ADDR);
        self.stack_pointer.reset();
    }

    fn adc(&mut self, mode: AddressingMode) -> Result<()> {
        let addr = self
            .get_operand_address(mode)
            .ok_or_else(|| anyhow!("Could not fetch address for performing ADC instruction"))?;

        let value = self.mem_read(addr);
        let carry = self.status_register.contains(StatusRegister::CARRY) as u8;
        let acc_sign_bit = self.accumulator.nth_bit(7);
        let result = self.accumulator.wrapping_add(value).wrapping_add(carry);
        let result_sign_bit = result.nth_bit(7);

        self.status_register
            .set_carry_flag(result < self.accumulator);
        self.status_register
            .set_overflow_flag(acc_sign_bit != result_sign_bit);
        self.status_register.update_zero_and_negative_flags(result);
        self.accumulator = result;

        Ok(())
    }

    fn sbc(&mut self, mode: AddressingMode) -> Result<()> {
        let addr = self
            .get_operand_address(mode)
            .ok_or_else(|| anyhow!("Could not fetch address for performing SBC instruction"))?;

        let value = self.mem_read(addr);
        let carry_neg = !self.status_register.contains(StatusRegister::CARRY) as u8;
        let acc_sign_bit = self.accumulator.nth_bit(7);
        let result = self.accumulator.wrapping_sub(value).wrapping_sub(carry_neg);
        let result_sign_bit = result.nth_bit(7);

        self.status_register
            .set_carry_flag(result > self.accumulator);
        self.status_register
            .set_overflow_flag(acc_sign_bit != result_sign_bit);
        self.status_register.update_zero_and_negative_flags(result);

        self.accumulator = result;

        Ok(())
    }

    fn compare(&mut self, mode: AddressingMode, register: Register) -> Result<()> {
        let addr = self
            .get_operand_address(mode)
            .ok_or_else(|| anyhow!("Failed to get operand address for compare instruction"))?;
        let value = self.mem_read(addr);
        let result = register.wrapping_sub(value);

        self.status_register.set_carry_flag(value <= register);
        self.status_register.update_zero_and_negative_flags(result);

        Ok(())
    }

    fn and(&mut self, mode: AddressingMode) -> Result<()> {
        let and = |acc, value| acc & value;
        self.logical_op_with_acc(mode, and).with_context(|| "AND")?;

        Ok(())
    }

    fn eor(&mut self, mode: AddressingMode) -> Result<()> {
        let xor = |acc, value| acc ^ value;
        self.logical_op_with_acc(mode, xor).with_context(|| "EOR")?;

        Ok(())
    }
    fn ora(&mut self, mode: AddressingMode) -> Result<()> {
        let or = |acc, value| acc | value;
        self.logical_op_with_acc(mode, or).with_context(|| "ORA")?;

        Ok(())
    }

    fn logical_op_with_acc(
        &mut self,
        mode: AddressingMode,
        logical_op: fn(Value, Value) -> Value,
    ) -> Result<()> {
        let addr = self
            .get_operand_address(mode)
            .ok_or_else(|| anyhow!("Could not fetch address for performing logical instruction"))?;
        let value = self.mem_read(addr);

        self.accumulator = logical_op(self.accumulator, value);
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);

        Ok(())
    }

    fn bit(&mut self, mode: AddressingMode) -> Result<()> {
        let addr = self
            .get_operand_address(mode)
            .ok_or_else(|| anyhow!("Could not fetch address for {} in BIT instruction"))?;

        let value = self.mem_read(addr);

        self.status_register.set_overflow_flag(value.nth_bit(6));
        self.status_register.set_negative_flag(value.nth_bit(7));
        self.status_register
            .set_zero_flag(value & self.accumulator == 0);

        Ok(())
    }

    fn asl(&mut self, mode: AddressingMode) -> Result<()> {
        let addr = self.get_operand_address(mode);
        let shifted = match addr {
            Some(addr) => {
                let value = self.mem_read(addr);
                let shifted_left = value << 1;

                self.mem_write(addr, shifted_left);
                self.status_register.set_carry_flag(value.nth_bit(7));

                shifted_left
            }
            None => {
                let old_reg_a = self.accumulator;
                self.accumulator <<= 1;
                self.status_register.set_carry_flag(old_reg_a.nth_bit(7));

                self.accumulator
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
                self.status_register.set_carry_flag(value.nth_bit(0));

                shifted
            }
            None => {
                let old_reg_a = self.accumulator;
                self.accumulator >>= 1;
                self.status_register.set_carry_flag(old_reg_a.nth_bit(0));
                self.accumulator
            }
        };

        self.status_register.set_negative_flag(false);
        self.status_register.update_zero_and_negative_flags(shifted);

        Ok(())
    }

    fn rol(&mut self, mode: AddressingMode) {
        let address = self.get_operand_address(mode);
        let input_carry = self.status_register.contains(StatusRegister::CARRY) as u8;

        let (previous, shifted) = match address {
            Some(addr) => {
                let value = self.mem_read(addr);
                let shifted = (value << 1) | input_carry;
                self.mem_write(addr, shifted);

                (value, shifted)
            }
            None => {
                let current_acc = self.accumulator;
                self.accumulator = (self.accumulator << 1) | input_carry;

                (current_acc, self.accumulator)
            }
        };

        self.status_register.set_carry_flag(previous.nth_bit(7));
        self.status_register.update_zero_and_negative_flags(shifted);
    }

    fn ror(&mut self, mode: AddressingMode) {
        let address = self.get_operand_address(mode);
        let input_carry =
            self.status_register.contains(StatusRegister::CARRY) as Value * 0b1000_0000;

        let (previous, shifted) = match address {
            Some(addr) => {
                let value = self.mem_read(addr);
                let shifted = (value >> 1) | input_carry;

                self.mem_write(addr, shifted);

                self.status_register.set_carry_flag(value.nth_bit(0));

                (value, shifted)
            }
            None => {
                let prev_acc_bits = self.accumulator;
                let shifted = (self.accumulator >> 1) | input_carry;

                self.accumulator = shifted;

                (prev_acc_bits, self.accumulator)
            }
        };

        self.status_register.set_carry_flag(previous.nth_bit(0));
        self.status_register.set_negative_flag(input_carry != 0);
        self.status_register.set_zero_flag(shifted == 0);
    }

    fn lda(&mut self, mode: AddressingMode) -> Result<()> {
        self.accumulator = self.load_value(mode)?;

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
        self.register_x = self.accumulator;
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn tay(&mut self) {
        self.register_y = self.accumulator;
        self.status_register
            .update_zero_and_negative_flags(self.register_y);
    }

    fn tsx(&mut self) {
        self.register_x = self.stack_pointer.value();
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn txa(&mut self) {
        self.accumulator = self.register_x;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn tya(&mut self) {
        self.accumulator = self.register_y;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
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

        self.accumulator = value;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn plp(&mut self) {
        let value = self.pop_stack();

        self.status_register = StatusRegister::from(value);
    }

    fn php(&mut self) {
        let mut status_register_with_b_flags = self.status_register;
        status_register_with_b_flags.insert(StatusRegister::BREAK | StatusRegister::BREAK2);

        self.stack_push(status_register_with_b_flags.bits());
    }

    fn sta(&mut self, mode: AddressingMode) -> Result<()> {
        self.store_value(self.accumulator, mode)
    }

    fn stx(&mut self, mode: AddressingMode) -> Result<()> {
        self.store_value(self.register_x, mode)
    }

    fn sty(&mut self, mode: AddressingMode) -> Result<()> {
        self.store_value(self.register_y, mode)
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            let offset = self.mem_read(self.program_counter) as i8;
            self.program_counter = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(offset as u16);
        }
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

                deref_base.wrapping_add(self.register_y.into())
            }
            _ => return None,
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

    #[derive(Debug)]
    enum Write {
        Single(Address, Value),
        U16(Address, u16),
    }

    struct CpuBuilder {
        writes: Vec<Write>,
    }

    impl CpuBuilder {
        fn new() -> Self {
            Self { writes: vec![] }
        }

        fn write(mut self, address: Address, value: Value) -> Self {
            self.writes.push(Write::Single(address, value));

            self
        }

        fn write_u16(mut self, address: Address, value: u16) -> Self {
            self.writes.push(Write::U16(address, value));

            self
        }

        fn build_and_run(self, data: &[Value]) -> Cpu {
            let mut cpu = Cpu::default();

            for write in self.writes {
                match write {
                    Write::Single(address, value) => cpu.mem_write(address, value),
                    Write::U16(address, value) => cpu.mem_write_u16(address, value),
                }
            }

            cpu.load_and_run(data).expect("Failed to load and run");

            cpu
        }
    }

    mod load {
        use super::*;

        #[test]
        fn immediate_load() {
            let data = [0xa9, 0x05, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.accumulator, 0x05);
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
            let data = [0xa5, 0x10, 0x00];
            let cpu = CpuBuilder::new().write(0x10, 0x55).build_and_run(&data);

            assert_eq!(cpu.accumulator, 0x55);
        }

        #[test]
        fn ldx_absolute() {
            let data = [0xae, 0x34, 0x12, 0x00];
            let cpu = CpuBuilder::new()
                .write_u16(0x1234, 0xff)
                .build_and_run(&data);

            assert_eq!(cpu.register_x, 0xff);
            assert!(!cpu.status_register.contains(StatusRegister::ZERO));
            assert!(cpu.status_register.contains(StatusRegister::NEGATIVE));
        }

        #[test]
        fn ldy_zero_page() {
            let mut cpu = Cpu::default();
            let data = [0xa4, 0xaa, 0x00];

            cpu.mem_write(0xaa, 0x66);
            cpu.load_and_run(&data).expect("Failed to load and run");

            assert_eq!(cpu.register_y, 0x66);
            assert!(!cpu.status_register.contains(StatusRegister::ZERO));
            assert!(!cpu.status_register.contains(StatusRegister::NEGATIVE));
        }
    }

    mod store {
        use super::*;

        #[test]
        fn sta_absolute() {
            // 1. store 0x75 in accumulator
            // 2. store accumulator value under address 0x1234
            let data = [0x0a9, 0x75, 0x8d, 0x34, 0x12, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.mem_read_u16(0x1234), 0x75);
            assert!(cpu.status_register.is_empty());
        }

        #[test]
        fn stx_zero_page() {
            // 1. Store 0x12 in memory location 0xee (setup)
            // 2. Store register X value (0) in memory location 0xee
            let data = [0x86, 0xee, 0x00];
            let cpu = CpuBuilder::new().write(0xee, 0x12).build_and_run(&data);

            assert_eq!(cpu.mem_read(0xee), 0x00);
            // STX does not modify any flags
            assert_eq!(cpu.status_register, StatusRegister::empty());
        }

        #[test]
        fn sty_zero_page_x() {
            // 1. store 0x01 in memory location 0x01
            // 2. store 0x02 in memory location 0x03
            // 3. load register X with value from address 0x01
            // 4. load register Y with value from address 0x03
            // 5. call STY with ZeroPageX addressing mode (store registry value Y in byte X on page zero
            let data = [0xa6, 0x01, 0xa4, 0x03, 0x94, 0x00];
            let cpu = CpuBuilder::new()
                .write(0x01, 0x02)
                .write(0x03, 0x04)
                .build_and_run(&data);

            assert_eq!(cpu.register_x, 0x02);
            assert_eq!(cpu.register_y, 0x04);
            assert_eq!(cpu.mem_read(0x02), 0x04);
            assert_eq!(cpu.status_register, StatusRegister::empty());
        }
    }

    mod transfer {
        use super::*;

        #[test]
        fn tax() {
            let data = [0xa9, 0x0a, 0xaa, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.accumulator, 10);
            assert_eq!(cpu.register_x, 10);
        }
    }

    mod increment_decrement {
        use super::*;

        #[test]
        fn dec_zero_page() {
            let data = [0xc6, 0x11, 0x00];
            let cpu = CpuBuilder::new().write(0x11, 0xf1).build_and_run(&data);

            assert_eq!(cpu.mem_read(0x11), 0xf0);
        }

        #[test]
        fn dex_underflow() {
            let data = [0xca, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.register_x, 0xff);
            assert_eq!(cpu.status_register, StatusRegister::NEGATIVE);
        }

        #[test]
        fn dey_underflow() {
            let data = [0x88, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.register_y, 0xff);
            assert_eq!(cpu.status_register, StatusRegister::NEGATIVE);
        }

        #[test]
        fn inc_absolute_two_times() {
            // increment value under address 0x1234 two times (0 -> 2)
            let data = [0xee, 0x34, 0x12, 0xee, 0x34, 0x12, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.mem_read(0x1234), 2);
            assert_eq!(cpu.status_register, StatusRegister::empty());
        }

        #[test]
        fn inx_overflow() {
            let data = [0xa9, 0xff, 0xaa, 0xe8, 0xe8, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.register_x, 1);
        }

        #[test]
        fn iny_three_times() {
            let data = [0xc8, 0xc8, 0xc8, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.register_y, 3);
            assert_eq!(cpu.status_register, StatusRegister::empty());
        }
    }

    mod logical {
        use super::*;

        #[test]
        fn and_immediate() {
            // 1. Load 0b1101_1010 to accumulator
            // 2. perform bitwise AND on the accumulator with 0b1100_0110
            let data = [0xa9, 0b1101_1010, 0x29, 0b1100_0110, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.accumulator, 0b1100_0010);
            assert_eq!(cpu.status_register, StatusRegister::NEGATIVE);
        }

        #[test]
        fn bit_zero_page() {
            let data = [0xa9, 0b1101_1010, 0x24, 0xdd, 0x00];
            let cpu = CpuBuilder::new()
                .write(0xdd, 0b1110_1010)
                .build_and_run(&data);

            // accumulator value should not change
            assert_eq!(cpu.accumulator, 0b1101_1010);
            assert_eq!(
                cpu.status_register,
                StatusRegister::OVERFLOW | StatusRegister::NEGATIVE
            );
        }

        #[test]
        fn eor_absolute() {
            // 1. Store 0b1010_0000 in address 0xbeef
            // 2. Load 0b1101_0110 to accumulator
            // 3. perform bitwise XOR on the accumulator with value under address 0xbeef
            let data = [0xa9, 0b1101_0110, 0x4d, 0xef, 0xbe, 0x00];
            let cpu = CpuBuilder::new()
                .write(0xbeef, 0b1010_0000)
                .build_and_run(&data);

            assert_eq!(cpu.accumulator, 0b0111_0110);
            assert_eq!(cpu.status_register, StatusRegister::empty());
        }

        #[test]
        fn ora_zero_page() {
            // 1. Store 0b1101_0110 in address 0xcc
            // 2. Load 0b1101_0110 to accumulator
            // 3. perform bitwise OR on the accumulator with value under address 0xcc
            let data = [0xa9, 0b1101_0110, 0x05, 0xcc, 0x00];
            let cpu = CpuBuilder::new()
                .write(0xcc, 0b0011_1011)
                .build_and_run(&data);

            assert_eq!(cpu.accumulator, 0b1111_1111);
            assert_eq!(cpu.status_register, StatusRegister::NEGATIVE);
        }
    }

    mod shifts {
        use super::*;

        #[test]
        fn asl_accumulator() {
            // 1. load 0b01010111 to accumulator
            // 2. call ASL - shifts accumulator left
            let data = [0xa9, 0b1101_0111, 0x0a, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            // store bit0 == 1 to original acc value in carry flag
            assert!(cpu.status_register.contains(StatusRegister::CARRY));
            // result was not a zero value, zero flag is reset
            assert!(!cpu.status_register.contains(StatusRegister::ZERO));
            // negative flag is set, due to bit 7 in result being 1
            assert!(cpu.status_register.contains(StatusRegister::NEGATIVE));
            assert_eq!(cpu.accumulator, 0b1010_1110);
        }

        #[test]
        fn asl_zero_page_x() {
            // 1. INX (register X = 1)
            // 2. store 0b0100_1101 in 0xab
            // 3. call ASL with 0xaa (zero page X mode) - shifts bits left in
            //    address 0xaa + 1 = 0xab
            let data = [0xe8, 0x16, 0xaa, 0x00];
            let cpu = CpuBuilder::new()
                .write(0xab, 0b0100_1101)
                .build_and_run(&data);

            assert_eq!(cpu.register_x, 1);
            assert_eq!(cpu.mem_read(0xab), 0b1001_1010);
            // result bit7 = 1
            assert_eq!(cpu.status_register, StatusRegister::NEGATIVE);
        }

        #[test]
        fn lsr_accumulator() {
            // 1. load 0b01010111 to accumulator
            // 2. call LSR - shifts accumulator right
            let data = [0xa9, 0b1101_0111, 0x4a, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.accumulator, 0b0110_1011);
            // store bit0 == 1 of original acc value in carry flag
            assert!(cpu.status_register.contains(StatusRegister::CARRY));
            // result was not a zero value, zero flag is reset
            assert!(!cpu.status_register.contains(StatusRegister::ZERO));
            // negative flag should always be cleared on LSR calls
            assert!(!cpu.status_register.contains(StatusRegister::NEGATIVE));
        }

        #[test]
        fn lsr_absolute_shift_into_carry() {
            let data = [0x4e, 0xda, 0xda, 0x00];
            let cpu = CpuBuilder::new()
                .write(0xdada, 0b01010111)
                .build_and_run(&data);

            assert_eq!(cpu.mem_read(0xdada), 0b00101011);
            assert_eq!(cpu.status_register, StatusRegister::CARRY);
        }

        #[test]
        fn rol_accumulator_with_carry() {
            // 1. load 0b01010111 to accumulator
            // 2. set carry flag
            // 3. call ROL
            let data = [0xa9, 0b0101_0010, 0x38, 0x2a, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.accumulator, 0b1010_0101);
            // store bit7 == 0 of original acc value in carry flag
            assert!(!cpu.status_register.contains(StatusRegister::CARRY));
            // result was not a zero value, zero flag is reset
            assert!(!cpu.status_register.contains(StatusRegister::ZERO));
            // negative flag is set, due to bit7 == 1 in rotated value
            assert!(cpu.status_register.contains(StatusRegister::NEGATIVE));
        }

        #[test]
        fn rol_with_carry_zero_page() {
            let data = [0x38, 0x26, 0xff, 0x00];
            let cpu = CpuBuilder::new()
                .write(0xff, 0b1010_1101)
                .build_and_run(&data);

            assert_eq!(cpu.mem_read(0xff), 0b0101_1011);
            assert_eq!(cpu.status_register, StatusRegister::CARRY);
        }

        #[test]
        fn ror_accumulator_with_carry() {
            // 1. load 0b01010111 to accumulator
            // 2. set carry flag
            // 3. call ROR
            let data = [0xa9, 0b0101_0010, 0x38, 0x6a, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.accumulator, 0b1010_1001);
            // store bit0 == 0 of original acc value in carry flag
            assert!(!cpu.status_register.contains(StatusRegister::CARRY));
            // result was not a zero value, zero flag is reset
            assert!(!cpu.status_register.contains(StatusRegister::ZERO));
            // negative flag is set, due to bit7 == 1 in rotated value
            assert!(cpu.status_register.contains(StatusRegister::NEGATIVE));
        }

        #[test]
        fn ror_absolute_x_without_carry() {
            let data = [0xe8, 0x7e, 0x33, 0x12, 0x00];
            let cpu = CpuBuilder::new()
                .write(0x1234, 0b0100_1101)
                .build_and_run(&data);

            assert_eq!(cpu.mem_read(0x1234), 0b0010_0110);
            assert_eq!(cpu.status_register, StatusRegister::CARRY);
        }
    }

    mod branch {
        use super::*;

        #[test]
        fn bcc_skips_lda() {
            // Call BCC with jumping two bytes forward (skips the immediate LDA instruction)
            let data = [0x90, 0x01, 0xa9, 0xff, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.accumulator, 0);
            assert_eq!(cpu.status_register, StatusRegister::empty());
        }

        #[test]
        fn bcs_does_not_skip() {
            // Call BCS which does not jump, because carry flag is not set
            let data = [0xb0, 0x01, 0xa9, 0xff, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.accumulator, 0xff);
            assert_eq!(cpu.status_register, StatusRegister::NEGATIVE);
        }
    }

    mod arithmetic {
        use super::*;

        #[test]
        fn adc_immediate_with_carry() {
            let data = [0xa9, 0xab, 0x38, 0x69, 0x11, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            // 0xab + 0x11 + 0x1
            assert_eq!(cpu.accumulator, 0xbd);
            assert_eq!(cpu.status_register, StatusRegister::NEGATIVE);
        }

        #[test]
        fn adc_zero_page_with_wrapping() {
            let data = [0xa9, 0xfe, 0x38, 0x65, 0x11, 0x00];
            let cpu = CpuBuilder::new().write(0x11, 0xaa).build_and_run(&data);

            // 0xfe + 0x1 + 0xaa wrapped
            assert_eq!(cpu.accumulator, 0xa9);
            // bit7 has changed and carry is set due to wrapping
            assert_eq!(
                cpu.status_register,
                StatusRegister::CARRY | StatusRegister::NEGATIVE
            );
        }

        #[test]
        fn cmp_absolute_same_values() {
            let data = [0xa9, 0x11, 0xcd, 0xde, 0xde, 0x00];
            let cpu = CpuBuilder::new().write(0xdede, 0x11).build_and_run(&data);

            // CMP should not change the value of accumulator
            assert_eq!(cpu.accumulator, 0x11);
            // values are the same, so zero and carry flags are set
            assert_eq!(
                cpu.status_register,
                StatusRegister::ZERO | StatusRegister::CARRY
            );
        }

        #[test]
        fn cmp_immediate_with_greater_value() {
            let data = [0xa9, 0xaa, 0xc9, 0xbb, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            // CMP should not change the value of accumulator
            assert_eq!(cpu.accumulator, 0xaa);
            // result bit7 is 1, so negative flag is set
            assert_eq!(cpu.status_register, StatusRegister::NEGATIVE);
        }

        #[test]
        fn cpx_zero_page() {
            // compare reg X = 0x2 with value 0xfe
            let data = [0xe8, 0xe8, 0xe4, 0xdd, 0x00];
            let cpu = CpuBuilder::new().write(0xdd, 0xfe).build_and_run(&data);

            assert_eq!(cpu.register_x, 0x2);
            assert_eq!(cpu.status_register, StatusRegister::empty());
        }

        #[test]
        fn cpy_immediate() {
            // compare reg Y = 0x3 with immediate value = 0x0
            let data = [0xc8, 0xc8, 0xc8, 0xc0, 0x00, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.register_y, 0x3);
            assert_eq!(cpu.status_register, StatusRegister::CARRY);
        }
    }

    mod flags {
        use super::*;

        #[test]
        fn carry_flag_enabled() {
            let data = [0x38, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.status_register, StatusRegister::CARRY);
        }

        #[test]
        fn decimal_flag_enabled() {
            let data = [0xf8, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.status_register, StatusRegister::DECIMAL);
        }

        #[test]
        fn interrupt_flag_enabled() {
            let data = [0x78, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.status_register, StatusRegister::INTERRUPT_DISABLE);
        }
    }
}
