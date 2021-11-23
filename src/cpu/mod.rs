mod addressing_mode;
mod memory;
pub mod opcodes;
mod stack_pointer;
mod status_register;

pub use crate::cpu::addressing_mode::AddressingMode;
pub use crate::cpu::memory::Memory;

use crate::bus::Bus;
use crate::cpu::opcodes::{Opcode, OPCODES_MAPPING};
use crate::cpu::stack_pointer::StackPointer;
use crate::cpu::status_register::StatusRegister;
use crate::interrupts::{Interrupt, NMI};
use crate::utils::{shift_left, shift_right, NthBit};
use crate::Byte;
use anyhow::{anyhow, bail, Context, Result};

pub type Register = u8;
pub type Address = u16;
pub type ProgramCounter = Address;

const PROGRAM_ROM_BEGIN_ADDR: Address = 0x0600;
const RESET_VECTOR_BEGIN_ADDR: Address = 0xfffc;

pub struct Cpu<'a> {
    pub accumulator: Register,
    pub register_x: Register,
    pub register_y: Register,
    pub status_register: StatusRegister,
    pub program_counter: ProgramCounter,
    pub stack_pointer: StackPointer,
    bus: Bus<'a>,
}

impl Memory for Cpu<'_> {
    fn read(&mut self, addr: Address) -> Result<Byte> {
        self.bus.read(addr)
    }

    fn write(&mut self, addr: Address, value: Byte) -> Result<()> {
        self.bus.write(addr, value)
    }

    fn read_u16(&mut self, addr: Address) -> Result<u16> {
        self.bus.read_u16(addr)
    }

    fn write_u16(&mut self, addr: Address, data: u16) -> Result<()> {
        self.bus.write_u16(addr, data)
    }
}

impl<'a> Cpu<'a> {
    pub fn new(bus: Bus) -> Cpu {
        Cpu {
            accumulator: 0,
            register_x: 0,
            register_y: 0,
            status_register: StatusRegister::INIT,
            program_counter: 0,
            stack_pointer: StackPointer::default(),
            bus,
        }
    }

    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    pub fn load_and_run(&mut self, data: &[Byte]) -> Result<()> {
        self.load(data)?;
        self.reset()?;
        self.program_counter = PROGRAM_ROM_BEGIN_ADDR;
        self.run()?;

        Ok(())
    }

    pub fn load(&mut self, data: &[Byte]) -> Result<()> {
        for (addr, &value) in data.iter().enumerate() {
            let addr = addr as Address;
            self.write(addr + PROGRAM_ROM_BEGIN_ADDR, value)?;
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        self.run_with_callback(|_| Ok(()))
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(&mut Cpu) -> Result<()>,
    {
        loop {
            if self.bus.poll_nmi_status().is_some() {
                self.interrupt(NMI)?;
            }

            callback(self)?;

            let code = self.read(self.program_counter)?;
            self.program_counter += 1;

            let current_program_counter = self.program_counter;

            let opcode = OPCODES_MAPPING
                .get(&code)
                .ok_or_else(|| anyhow!("Unknown opcode: {}", code))?;

            match opcode.name {
                "ADC" => self.adc(opcode)?,
                "AND" => self.and(opcode)?,
                "ASL" => self.asl(opcode)?,
                "BIT" => self.bit(opcode)?,
                "BCC" => self.branch(!self.status_register.contains(StatusRegister::CARRY))?,
                "BCS" => self.branch(self.status_register.contains(StatusRegister::CARRY))?,
                "BEQ" => self.branch(self.status_register.contains(StatusRegister::ZERO))?,
                "BMI" => self.branch(self.status_register.contains(StatusRegister::NEGATIVE))?,
                "BNE" => self.branch(!self.status_register.contains(StatusRegister::ZERO))?,
                "BPL" => self.branch(!self.status_register.contains(StatusRegister::NEGATIVE))?,
                "BVC" => self.branch(!self.status_register.contains(StatusRegister::OVERFLOW))?,
                "BVS" => self.branch(self.status_register.contains(StatusRegister::OVERFLOW))?,
                "BRK" => return Ok(()),
                "CLC" => self.status_register.set_carry_flag(false),
                "CLD" => self.status_register.set_decimal_flag(false),
                "CLI" => self.status_register.set_interrupt_flag(false),
                "CLV" => self.status_register.set_overflow_flag(false),
                "CMP" => self.compare(opcode, self.accumulator)?,
                "CPX" => self.compare(opcode, self.register_x)?,
                "CPY" => self.compare(opcode, self.register_y)?,
                "DEC" => self.dec(opcode)?,
                "DEX" => self.dex(),
                "DEY" => self.dey(),
                "EOR" => self.eor(opcode)?,
                "INC" => self.inc(opcode)?,
                "INX" => self.inx(),
                "INY" => self.iny(),
                "JMP" => self.jmp(opcode)?,
                "JSR" => self.jsr()?,
                "LDA" => self.lda(opcode)?,
                "LDX" => self.ldx(opcode)?,
                "LDY" => self.ldy(opcode)?,
                "LSR" => self.lsr(opcode)?,
                "NOP" | "*NOP" => {}
                "ORA" => self.ora(opcode)?,
                "PHA" => self.push_stack(self.accumulator)?,
                "PHP" => self.php()?,
                "PLA" => self.pla()?,
                "PLP" => self.plp()?,
                "ROL" => self.rol(opcode)?,
                "ROR" => self.ror(opcode)?,
                "RTI" => {
                    self.rti()?;
                    continue;
                }
                "RTS" => {
                    self.rts()?;
                    continue;
                }
                "SBC" | "*SBC" => self.sbc(opcode)?,
                "SEC" => self.status_register.set_carry_flag(true),
                "SED" => self.status_register.set_decimal_flag(true),
                "SEI" => self.status_register.set_interrupt_flag(true),
                "STA" => self.sta(opcode)?,
                "STX" => self.stx(opcode)?,
                "STY" => self.sty(opcode)?,
                "TAX" => self.tax(),
                "TAY" => self.tay(),
                "TSX" => self.tsx(),
                "TXA" => self.txa(),
                "TXS" => self.stack_pointer.set(self.register_x),
                "TYA" => self.tya(),

                "*LAX" => self.lax(opcode)?,
                "*SAX" => self.sax(opcode)?,
                "*DCP" => self.dcp(opcode)?,
                "*ISB" => self.isb(opcode)?,
                "*SLO" => self.slo(opcode)?,
                "*RLA" => self.rla(opcode)?,
                "*SRE" => self.sre(opcode)?,
                "*RRA" => self.rra(opcode)?,
                _ => bail!("Unsupported opcode name: {}", opcode.name),
            }

            self.bus.tick(opcode.length());

            if current_program_counter == self.program_counter {
                self.program_counter += opcode.length() as u16;
            }
        }
    }

    pub fn reset(&mut self) -> Result<()> {
        self.accumulator = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status_register = StatusRegister::empty();
        self.program_counter = self.read_u16(RESET_VECTOR_BEGIN_ADDR)?;
        self.stack_pointer.reset();

        Ok(())
    }

    fn adc(&mut self, opcode: &Opcode) -> Result<()> {
        let addr = self.operand_address(opcode)?;
        let addr =
            addr.ok_or_else(|| anyhow!("Could not fetch address for performing ADC instruction"))?;

        let value = self.read(addr)?;
        self.add_to_acc(value);

        Ok(())
    }

    fn sbc(&mut self, opcode: &Opcode) -> Result<()> {
        let addr = self
            .operand_address(opcode)?
            .ok_or_else(|| anyhow!("Could not fetch address for performing SBC instruction"))?;

        let value = self.read(addr)?;
        let neg = ((value as i8).wrapping_neg().wrapping_sub(1)) as Byte;

        self.add_to_acc(neg);

        Ok(())
    }

    fn add_to_acc(&mut self, data: u8) {
        let input_carry = self.status_register.contains(StatusRegister::CARRY) as u16;
        let sum_wide = self.accumulator as u16 + data as u16 + input_carry;

        let result = sum_wide as Byte;

        self.status_register.set_carry_flag(sum_wide > 0xff);
        self.status_register
            .set_overflow_flag((data ^ result) & (result ^ self.accumulator) & 0x80 != 0);

        self.accumulator = result;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn compare(&mut self, opcode: &Opcode, register: Register) -> Result<()> {
        let addr = self
            .operand_address(opcode)?
            .ok_or_else(|| anyhow!("Failed to get operand address for compare instruction"))?;
        let value = self.read(addr)?;
        let result = register.wrapping_sub(value);

        self.status_register.set_carry_flag(value <= register);
        self.status_register.update_zero_and_negative_flags(result);

        Ok(())
    }

    fn and(&mut self, opcode: &Opcode) -> Result<()> {
        let and = |acc, value| acc & value;
        self.logical_op_with_acc(opcode, and)
            .with_context(|| "AND")?;

        Ok(())
    }

    fn eor(&mut self, opcode: &Opcode) -> Result<()> {
        let xor = |acc, value| acc ^ value;
        self.logical_op_with_acc(opcode, xor)
            .with_context(|| "EOR")?;

        Ok(())
    }
    fn ora(&mut self, opcode: &Opcode) -> Result<()> {
        let or = |acc, value| acc | value;
        self.logical_op_with_acc(opcode, or)
            .with_context(|| "ORA")?;

        Ok(())
    }

    fn logical_op_with_acc(
        &mut self,
        opcode: &Opcode,
        logical_op: fn(Byte, Byte) -> Byte,
    ) -> Result<()> {
        let addr = self
            .operand_address(opcode)?
            .ok_or_else(|| anyhow!("Could not fetch address for performing logical instruction"))?;
        let value = self.read(addr)?;

        self.accumulator = logical_op(self.accumulator, value);
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);

        Ok(())
    }

    fn bit(&mut self, opcode: &Opcode) -> Result<()> {
        let addr = self
            .operand_address(opcode)?
            .ok_or_else(|| anyhow!("Could not fetch address for BIT instruction"))?;
        let value = self.read(addr)?;

        self.status_register.set_overflow_flag(value.nth_bit(6));
        self.status_register.set_negative_flag(value.nth_bit(7));
        self.status_register
            .set_zero_flag(value & self.accumulator == 0);

        Ok(())
    }

    fn asl(&mut self, opcode: &Opcode) -> Result<()> {
        let (old_value, shifted) = self.shift(opcode, 0, shift_left)?;

        self.status_register.set_carry_flag(old_value.nth_bit(7));
        self.status_register.update_zero_and_negative_flags(shifted);

        Ok(())
    }

    fn lsr(&mut self, opcode: &Opcode) -> Result<()> {
        let (old_value, shifted) = self.shift(opcode, 0, shift_right)?;

        self.status_register.set_carry_flag(old_value.nth_bit(0));
        self.status_register.update_zero_and_negative_flags(shifted);

        Ok(())
    }

    fn rol(&mut self, opcode: &Opcode) -> Result<()> {
        let input_carry = self.status_register.contains(StatusRegister::CARRY) as u8;
        let (previous, shifted) = self.shift(opcode, input_carry, shift_left)?;

        self.status_register.set_carry_flag(previous.nth_bit(7));
        self.status_register.update_zero_and_negative_flags(shifted);

        Ok(())
    }

    fn ror(&mut self, opcode: &Opcode) -> Result<()> {
        let input_carry =
            self.status_register.contains(StatusRegister::CARRY) as Byte * 0b1000_0000;
        let (previous, shifted) = self.shift(opcode, input_carry, shift_right)?;

        self.status_register.set_carry_flag(previous.nth_bit(0));
        self.status_register.update_zero_and_negative_flags(shifted);

        Ok(())
    }

    fn shift(
        &mut self,
        opcode: &Opcode,
        input_carry: Byte,
        shift_op: fn(Byte) -> Byte,
    ) -> Result<(Byte, Byte)> {
        let address = self.operand_address(opcode)?;

        let (old_value, shifted) = match address {
            Some(addr) => {
                let value = self.read(addr)?;
                let shifted = shift_op(value) | input_carry;

                self.write(addr, shifted)?;

                (value, shifted)
            }
            None => {
                let old_acc = self.accumulator;
                self.accumulator = shift_op(self.accumulator) | input_carry;

                (old_acc, self.accumulator)
            }
        };

        Ok((old_value, shifted))
    }

    fn lda(&mut self, opcode: &Opcode) -> Result<()> {
        self.accumulator = self.load_value(opcode)?;

        Ok(())
    }

    fn ldx(&mut self, opcode: &Opcode) -> Result<()> {
        self.register_x = self.load_value(opcode)?;

        Ok(())
    }

    fn ldy(&mut self, opcode: &Opcode) -> Result<()> {
        self.register_y = self.load_value(opcode).with_context(|| "In LDY")?;

        Ok(())
    }

    fn lax(&mut self, opcode: &Opcode) -> Result<()> {
        self.accumulator = self.load_value(opcode)?;
        self.register_x = self.accumulator;

        Ok(())
    }

    fn sax(&mut self, opcode: &Opcode) -> Result<()> {
        let address = self
            .operand_address(opcode)?
            .ok_or_else(|| anyhow!("Could not fetch address in LAX instruction"))?;
        let result = self.accumulator & self.register_x;

        self.write(address, result)?;

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

    fn dec(&mut self, opcode: &Opcode) -> Result<()> {
        let addr = self
            .operand_address(opcode)?
            .ok_or_else(|| anyhow!("Could not fetch address in DEC instruction"))?;
        let dec_value = self.read(addr)?.wrapping_sub(1);

        self.write(addr, dec_value)?;
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

    fn inc(&mut self, opcode: &Opcode) -> Result<()> {
        let addr = self
            .operand_address(opcode)?
            .ok_or_else(|| anyhow!("Could not fetch address for in INC instruction"))?;
        let inc_value = self.read(addr)?.wrapping_add(1);

        self.write(addr, inc_value)?;
        self.status_register
            .update_zero_and_negative_flags(inc_value);

        Ok(())
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.status_register
            .update_zero_and_negative_flags(self.register_y);
    }

    fn jmp(&mut self, opcode: &Opcode) -> Result<()> {
        let addr = self
            .operand_address(opcode)?
            .ok_or_else(|| anyhow!("Failed to fetch operand address for JMP instruction"))?;
        self.program_counter = addr;

        Ok(())
    }

    fn jsr(&mut self) -> Result<()> {
        self.push_stack_u16(self.program_counter + 1)?;
        let target_address = self.read_u16(self.program_counter)?;
        self.program_counter = target_address;

        Ok(())
    }

    fn rti(&mut self) -> Result<()> {
        self.status_register = self.pop_stack()?.into();
        self.status_register.remove(StatusRegister::BREAK);
        self.status_register.insert(StatusRegister::BREAK2);

        self.program_counter = self.pop_stack_u16()?;

        Ok(())
    }

    fn rts(&mut self) -> Result<()> {
        self.program_counter = self.pop_stack_u16()? + 1;

        Ok(())
    }

    fn pla(&mut self) -> Result<()> {
        let value = self.pop_stack()?;

        self.accumulator = value;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);

        Ok(())
    }

    fn plp(&mut self) -> Result<()> {
        let value = self.pop_stack()?;

        self.status_register = StatusRegister::from(value);
        self.status_register.remove(StatusRegister::BREAK);
        self.status_register.insert(StatusRegister::BREAK2);

        Ok(())
    }

    fn php(&mut self) -> Result<()> {
        let mut status_register_with_b_flags = self.status_register;
        status_register_with_b_flags.insert(StatusRegister::BREAK | StatusRegister::BREAK2);

        self.push_stack(status_register_with_b_flags.bits())
    }

    fn sta(&mut self, opcode: &Opcode) -> Result<()> {
        self.store_value(opcode, self.accumulator)
    }

    fn stx(&mut self, opcode: &Opcode) -> Result<()> {
        self.store_value(opcode, self.register_x)
    }

    fn sty(&mut self, opcode: &Opcode) -> Result<()> {
        self.store_value(opcode, self.register_y)
    }

    fn branch(&mut self, condition: bool) -> Result<()> {
        if condition {
            self.bus.tick(1);

            let jump = self.read(self.program_counter)? as i8;
            let jump_addr = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(jump as u16);

            if is_page_crossed(self.program_counter, jump_addr) {
                self.bus.tick(1);
            }

            self.program_counter = jump_addr;
        }

        Ok(())
    }

    pub fn operand_address(&mut self, opcode: &Opcode) -> Result<Option<Address>> {
        Ok(Some(match opcode.mode {
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::ZeroPage => self.read(self.program_counter)?.into(),
            AddressingMode::Absolute => self.read_u16(self.program_counter)?,
            AddressingMode::ZeroPageX => {
                let pos = self.read(self.program_counter)?;
                let addr = pos.wrapping_add(self.register_x);

                addr.into()
            }

            AddressingMode::ZeroPageY => {
                let pos = self.read(self.program_counter)?;
                let addr = pos.wrapping_add(self.register_y);

                addr.into()
            }
            AddressingMode::AbsoluteX => {
                let base = self.read_u16(self.program_counter)?;
                let incremented = base.wrapping_add(self.register_x.into());

                if opcode.needs_page_cross_check && is_page_crossed(base, incremented) {
                    self.bus.tick(1);
                }

                incremented
            }
            AddressingMode::AbsoluteY => {
                let base = self.read_u16(self.program_counter)?;
                let incremented = base.wrapping_add(self.register_y.into());

                if opcode.needs_page_cross_check && is_page_crossed(base, incremented) {
                    self.bus.tick(1);
                }

                incremented
            }
            AddressingMode::IndirectX => {
                let base = self.read(self.program_counter)?;
                let ptr = base.wrapping_add(self.register_x);
                let lo = self.read(ptr.into())?;
                let hi = self.read(ptr.wrapping_add(1).into())?;

                u16::from_le_bytes([lo, hi])
            }
            AddressingMode::IndirectY => {
                let base = self.read(self.program_counter)?;
                let lo = self.read(base.into())?;
                let hi = self.read(base.wrapping_add(1).into())?;
                let deref_base = u16::from_le_bytes([lo, hi]);
                let incremented = deref_base.wrapping_add(self.register_y.into());

                if opcode.needs_page_cross_check && is_page_crossed(deref_base, incremented) {
                    self.bus.tick(1);
                }

                incremented
            }
            AddressingMode::Indirect => {
                let address = self.read_u16(self.program_counter)?;

                // recreate the CPU bug with page boundaries:
                // "The indirect jump instruction does not increment the page address when the indirect pointer
                // crosses a page boundary.
                // JMP ($xxFF) will fetch the address from $xxFF and $xx00."
                if address & 0x00ff == 0x00ff {
                    let lo = self.read(address)? as Address;
                    let hi = self.read(address & 0xff00)? as Address;

                    hi << 8 | lo
                } else {
                    self.read_u16(address)?
                }
            }
            _ => return Ok(None),
        }))
    }

    fn load_value(&mut self, opcode: &Opcode) -> Result<Byte> {
        let addr = self.operand_address(opcode)?.ok_or_else(|| {
            anyhow!(
                "Could not get operand address when loading value ({:?})",
                opcode.mode
            )
        })?;
        let value = self.read(addr)?;

        self.status_register.update_zero_and_negative_flags(value);

        Ok(value)
    }

    fn store_value(&mut self, opcode: &Opcode, value: Byte) -> Result<()> {
        let addr = self.operand_address(opcode)?.ok_or_else(|| {
            anyhow!(
                "Could not fetch address when storing value ({:?}",
                opcode.mode
            )
        })?;

        self.write(addr, value)?;

        Ok(())
    }

    fn push_stack(&mut self, value: Byte) -> Result<()> {
        self.write(self.stack_pointer.address(), value)?;

        self.stack_pointer.decrement();

        Ok(())
    }

    fn push_stack_u16(&mut self, value: u16) -> Result<()> {
        let [lo, hi] = value.to_le_bytes();

        self.push_stack(hi)?;
        self.push_stack(lo)?;

        Ok(())
    }

    fn pop_stack(&mut self) -> Result<Byte> {
        self.stack_pointer.increment();
        self.read(self.stack_pointer.address())
    }

    fn pop_stack_u16(&mut self) -> Result<u16> {
        let lo = self.pop_stack()? as u16;
        let hi = self.pop_stack()? as u16;

        Ok(hi << 8 | lo)
    }

    fn interrupt(&mut self, interrupt: Interrupt) -> Result<()> {
        self.push_stack_u16(self.program_counter)?;
        let mut status = self.status_register;
        status.remove(StatusRegister::BREAK);
        status.insert(StatusRegister::BREAK2);

        self.push_stack(status.bits())?;
        self.status_register.disable_interrupt();

        self.bus.tick(interrupt.cpu_cycles);
        self.program_counter = self.read_u16(interrupt.vector_addr)?;

        Ok(())
    }

    // TODO
    fn dcp(&mut self, _opcode: &Opcode) -> Result<()> {
        Ok(())
    }

    // TODO
    fn isb(&mut self, _opcode: &Opcode) -> Result<()> {
        Ok(())
    }

    // TODO
    fn slo(&mut self, _opcode: &Opcode) -> Result<()> {
        Ok(())
    }

    // TODO
    fn rla(&mut self, _opcode: &Opcode) -> Result<()> {
        Ok(())
    }

    // TODO
    fn sre(&mut self, _opcode: &Opcode) -> Result<()> {
        Ok(())
    }

    // TODO
    fn rra(&mut self, _opcode: &Opcode) -> Result<()> {
        Ok(())
    }
}

fn is_page_crossed(before: Address, after: Address) -> bool {
    let page_before = before & 0xff00;
    let page_after = after & 0xff00;

    page_before != page_after
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Rom;
    use assert_matches::assert_matches;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref TEST_ROM: Vec<Byte> = {
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

    #[derive(Debug)]
    enum Write {
        Single(Address, Byte),
        U16(Address, u16),
    }

    struct CpuBuilder {
        writes: Vec<Write>,
    }

    impl CpuBuilder {
        fn new() -> Self {
            Self { writes: vec![] }
        }

        fn write(mut self, address: Address, value: Byte) -> Self {
            self.writes.push(Write::Single(address, value));

            self
        }

        fn write_u16(mut self, address: Address, value: u16) -> Self {
            self.writes.push(Write::U16(address, value));

            self
        }

        fn build_and_run(self, data: &[Byte]) -> Cpu {
            let rom = Rom::new(&TEST_ROM).expect("Failed to parse test ROM");
            let bus = Bus::new(rom, |_ppu| {});
            let mut cpu = Cpu::new(bus);
            cpu.status_register = StatusRegister::empty();

            for write in self.writes {
                match write {
                    Write::Single(address, value) => {
                        cpu.write(address, value).expect("Failed to write")
                    }
                    Write::U16(address, value) => {
                        cpu.write_u16(address, value).expect("Failed to write u16")
                    }
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
            let data = [0xa9, 0x00, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

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
            let data = [0xa4, 0xaa, 0x00];
            let cpu = CpuBuilder::new().write(0xaa, 0x66).build_and_run(&data);

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
            let mut cpu = CpuBuilder::new().build_and_run(&data);

            assert_matches!(cpu.read_u16(0x1234), Ok(0x75));
            assert!(cpu.status_register.is_empty());
        }

        #[test]
        fn stx_zero_page() -> Result<()> {
            // 1. Store 0x12 in memory location 0xee (setup)
            // 2. Store register X value (0) in memory location 0xee
            let data = [0x86, 0xee, 0x00];
            let mut cpu = CpuBuilder::new().write(0xee, 0x12).build_and_run(&data);

            assert_eq!(cpu.read(0xee)?, 0x00);
            // STX does not modify any flags
            assert_eq!(cpu.status_register, StatusRegister::empty());

            Ok(())
        }

        #[test]
        fn sty_zero_page_x() -> Result<()> {
            // 1. store 0x01 in memory location 0x01
            // 2. store 0x02 in memory location 0x03
            // 3. load register X with value from address 0x01
            // 4. load register Y with value from address 0x03
            // 5. call STY with ZeroPageX addressing mode (store registry value Y in byte X on page zero
            let data = [0xa6, 0x01, 0xa4, 0x03, 0x94, 0x00];
            let mut cpu = CpuBuilder::new()
                .write(0x01, 0x02)
                .write(0x03, 0x04)
                .build_and_run(&data);

            assert_eq!(cpu.register_x, 0x02);
            assert_eq!(cpu.register_y, 0x04);
            assert_eq!(cpu.read(0x02)?, 0x04);
            assert_eq!(cpu.status_register, StatusRegister::empty());

            Ok(())
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
            let mut cpu = CpuBuilder::new().write(0x11, 0xf1).build_and_run(&data);

            assert_matches!(cpu.read(0x11), Ok(0xf0));
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
            let mut cpu = CpuBuilder::new().build_and_run(&data);

            assert_matches!(cpu.read(0x1234), Ok(0x02));
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
            let data = [0xa9, 0b1101_0110, 0x4d, 0xef, 0x1a, 0x00];
            let cpu = CpuBuilder::new()
                .write(0x1aef, 0b1010_0000)
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
            let mut cpu = CpuBuilder::new()
                .write(0xab, 0b0100_1101)
                .build_and_run(&data);

            assert_eq!(cpu.register_x, 1);
            assert_matches!(cpu.read(0xab), Ok(0b1001_1010));
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
            let data = [0x4e, 0xda, 0x0a, 0x00];
            let mut cpu = CpuBuilder::new()
                .write(0x0ada, 0b01010111)
                .build_and_run(&data);

            assert_matches!(cpu.read(0x0ada), Ok(0b00101011));
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
            let mut cpu = CpuBuilder::new()
                .write(0xff, 0b1010_1101)
                .build_and_run(&data);

            assert_matches!(cpu.read(0xff), Ok(0b0101_1011));
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
            let mut cpu = CpuBuilder::new()
                .write(0x1234, 0b0100_1101)
                .build_and_run(&data);

            assert_matches!(cpu.read(0x1234), Ok(0b0010_0110));
            assert_eq!(cpu.status_register, StatusRegister::CARRY);
        }
    }

    mod branch {
        use super::*;

        #[test]
        fn bcc_skips_lda() {
            // Call BCC with jumping two bytes forward (skips the immediate LDA instruction)
            let data = [0x90, 0x02, 0xa9, 0xff, 0x00];
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
            let data = [0xa9, 0x11, 0xcd, 0xde, 0x1e, 0x00];
            let cpu = CpuBuilder::new().write(0x1ede, 0x11).build_and_run(&data);

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

    mod control {
        use super::*;

        #[test]
        fn jmp_absolute() {
            let data = [0x4c, 0x33, 0x12, 0x00];
            let cpu = CpuBuilder::new().build_and_run(&data);

            assert_eq!(cpu.program_counter, 0x1234);
            assert_eq!(cpu.status_register, StatusRegister::empty());
        }

        #[test]
        fn jmp_indirect() {
            let data = [0x6c, 0x34, 0x12];
            let cpu = CpuBuilder::new()
                .write_u16(0x1234, 0xbeee)
                .build_and_run(&data);

            assert_eq!(cpu.program_counter, 0xbeef);
            assert_eq!(cpu.status_register, StatusRegister::empty());
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
