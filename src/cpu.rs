mod addressing_mode;
mod interrupts;
mod memory;
pub mod opcodes;
mod stack_pointer;
mod status_register;

pub use crate::cpu::addressing_mode::AddressingMode;
pub use crate::cpu::memory::Memory;

use crate::bus::Bus;
use crate::cpu::interrupts::Interrupt;
use crate::cpu::opcodes::{OPCODES_MAPPING, Opcode};
use crate::cpu::stack_pointer::StackPointer;
use crate::cpu::status_register::StatusRegister;
use crate::ppu::NmiStatus;
use crate::utils::NthBit;
use crate::{Address, Byte, Word};
use anyhow::{Context, Result, anyhow, bail};
use log::debug;

const PROGRAM_ROM_BEGIN_ADDR: Address = Address::new(0x0600);
const RESET_VECTOR_BEGIN_ADDR: Address = Address::new(0xfffc);

struct ByteUpdate {
    previous: Byte,
    new: Byte,
}

pub struct Cpu {
    pub accumulator: Byte,
    pub register_x: Byte,
    pub register_y: Byte,
    pub status_register: StatusRegister,
    pub program_counter: Address,
    stack_pointer: StackPointer,
    bus: Bus,
}

impl Memory for Cpu {
    fn read_byte(&mut self, addr: Address) -> Result<Byte> {
        self.bus.read_byte(addr)
    }

    fn write_byte(&mut self, addr: Address, value: Byte) -> Result<()> {
        self.bus.write_byte(addr, value)
    }

    fn read_word(&mut self, addr: Address) -> Result<Word> {
        self.bus.read_word(addr)
    }

    fn write_word(&mut self, addr: Address, word: Word) -> Result<()> {
        self.bus.write_word(addr, word)
    }
}

impl Cpu {
    pub fn new(bus: Bus) -> Cpu {
        Cpu {
            accumulator: Byte::default(),
            register_x: Byte::default(),
            register_y: Byte::default(),
            status_register: StatusRegister::INIT,
            program_counter: Address::default(),
            stack_pointer: StackPointer::default(),
            bus,
        }
    }

    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    /// Read a byte without side effects, for use by the trace/debugger.
    pub fn peek_byte(&self, address: Address) -> Byte {
        self.bus.peek_byte(address)
    }

    pub fn bus_mut(&mut self) -> &mut Bus {
        &mut self.bus
    }

    pub fn stack_pointer(&self) -> StackPointer {
        self.stack_pointer
    }

    pub fn load(&mut self, data: &[Byte]) -> Result<()> {
        for (index, &value) in data.iter().enumerate() {
            let index = u16::try_from(index)?;
            let addr = Address::from(index);
            self.write_byte(addr + PROGRAM_ROM_BEGIN_ADDR, value)?;
        }

        Ok(())
    }

    /// Execute a single CPU instruction.
    pub fn step(&mut self) -> Result<()> {
        // Handle NMI interrupt if pending
        if self.bus.poll_nmi_status() == NmiStatus::Active {
            self.interrupt(&interrupts::NMI)?;
        }

        // Handle IRQ if pending and interrupt flag is clear
        if self.bus.poll_irq_status()
            && !self
                .status_register
                .contains(StatusRegister::INTERRUPT_DISABLE)
        {
            self.interrupt(&interrupts::IRQ)?;
        }

        let code = self.read_byte(self.program_counter)?;
        let instruction_pc = self.program_counter;
        self.program_counter = self.program_counter.wrapping_add(1u16);

        let current_program_counter = self.program_counter;
        let opcode = OPCODES_MAPPING
            .get(&code)
            .ok_or_else(|| anyhow!("Unknown opcode: {code:02X} at PC ${instruction_pc:04X}"))?;
        let address = self
            .pc_operand_address(opcode)
            .with_context(|| format!("Failed to fetch address for {}", opcode.name))?;
        let opcode_name = opcode.name;

        match opcode_name {
            "ADC" => self.adc(address)?,
            "AND" => self.and(address)?,
            "ASL" => self.asl(address, opcode.addressing_mode)?,
            "BIT" => self.bit(address)?,
            "BCC" => self.branch(!self.status_register.contains(StatusRegister::CARRY))?,
            "BCS" => self.branch(self.status_register.contains(StatusRegister::CARRY))?,
            "BEQ" => self.branch(self.status_register.contains(StatusRegister::ZERO))?,
            "BMI" => self.branch(self.status_register.contains(StatusRegister::NEGATIVE))?,
            "BNE" => self.branch(!self.status_register.contains(StatusRegister::ZERO))?,
            "BPL" => self.branch(!self.status_register.contains(StatusRegister::NEGATIVE))?,
            "BVC" => self.branch(!self.status_register.contains(StatusRegister::OVERFLOW))?,
            "BVS" => self.branch(self.status_register.contains(StatusRegister::OVERFLOW))?,
            "BRK" => {
                self.program_counter = self.program_counter.wrapping_add(1u16); // skip the padding byte (BRK is a 2-byte instruction)
                self.interrupt(&interrupts::BRK)?;
                return Ok(());
            }
            "CLC" => {
                self.status_register.set_carry_flag(false);
            }
            "CLD" => {
                self.status_register.set_decimal_flag(false);
            }
            "CLI" => {
                self.status_register.set_interrupt_flag(false);
            }
            "CLV" => {
                self.status_register.set_overflow_flag(false);
            }
            "CMP" => self.compare(address, self.accumulator)?,
            "CPX" => self.compare(address, self.register_x)?,
            "CPY" => self.compare(address, self.register_y)?,
            "DEC" => self.dec(address)?,
            "DEX" => self.dex(),
            "DEY" => self.dey(),
            "EOR" => self.eor(address)?,
            "INC" => self.inc(address)?,
            "INX" => self.inx(),
            "INY" => self.iny(),
            "JMP" => self.program_counter = address,
            "JSR" => self.jsr()?,
            "LDA" => self.lda(address)?,
            "LDX" => self.ldx(address)?,
            "LDY" => self.ldy(address)?,
            "LSR" => self.lsr(address, opcode.addressing_mode)?,
            "NOP" | "*NOP" => {} // noop - do nothing
            "ORA" => self.ora(address)?,
            "PHA" => self.push_byte_to_stack(self.accumulator)?,
            "PHP" => self.php()?,
            "PLA" => self.pla()?,
            "PLP" => self.plp()?,
            "ROL" => self.rol(address, opcode.addressing_mode)?,
            "ROR" => self.ror(address, opcode.addressing_mode)?,
            "RTI" => {
                self.rti()?;
                self.bus.tick(opcode.cycles)?;
                return Ok(());
            }
            "RTS" => {
                self.rts()?;
                self.bus.tick(opcode.cycles)?;
                return Ok(());
            }
            "SBC" | "*SBC" => self.sbc(address)?,
            "SEC" => {
                self.status_register.set_carry_flag(true);
            }
            "SED" => {
                self.status_register.set_decimal_flag(true);
            }
            "SEI" => {
                self.status_register.set_interrupt_flag(true);
            }
            "STA" => self.write_byte(address, self.accumulator)?,
            "STX" => self.write_byte(address, self.register_x)?,
            "STY" => self.write_byte(address, self.register_y)?,
            "TAX" => self.tax(),
            "TAY" => self.tay(),
            "TSX" => self.tsx(),
            "TXA" => self.txa(),
            "TXS" => self.stack_pointer.set(self.register_x),
            "TYA" => self.tya(),

            "*LAX" => self.lax(address)?,
            "*SAX" => self.sax(address)?,
            "*DCP" => self.dcp(address)?,
            "*ISB" => self.isb(address)?,
            "*SLO" => self.slo(address)?,
            "*RLA" => self.rla(address, opcode.addressing_mode)?,
            "*SRE" => self.sre(address)?,
            "*RRA" => self.rra(address, opcode.addressing_mode)?,
            "*ANC" => self.anc(address)?,
            "*ALR" => self.alr(address)?,
            "*ARR" => self.arr(address)?,
            "*ANE" => self.ane(address)?,
            "*LXA" => self.lxa(address)?,
            "*AXS" => self.axs(address)?,
            "*SHA" | "*SHX" | "*SHY" | "*SHS" | "*LAS" => {} // unstable — treat as NOP
            _ => bail!("Unsupported opcode name: {opcode_name}"),
        }

        self.bus.tick(opcode.cycles)?;

        if current_program_counter == self.program_counter {
            let len: u16 = opcode.length().try_into()?;
            self.program_counter = self.program_counter.wrapping_add(len);
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            self.step()?;
        }
    }

    pub fn reset(&mut self) -> Result<()> {
        self.accumulator = Byte::default();
        self.register_x = Byte::default();
        self.register_y = Byte::default();
        self.status_register = StatusRegister::empty();
        self.program_counter = self.read_word(RESET_VECTOR_BEGIN_ADDR)?.as_address();
        debug!("CPU reset: PC set to ${:04X}", self.program_counter);
        self.stack_pointer.reset();

        Ok(())
    }

    fn adc(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        self.add_to_acc(value);

        Ok(())
    }

    fn sbc(&mut self, address: Address) -> Result<()> {
        let negated = self
            .read_byte(address)?
            .value()
            .cast_signed()
            .wrapping_neg()
            .wrapping_sub(1)
            .cast_unsigned();

        self.add_to_acc(negated.into());

        Ok(())
    }

    fn add_to_acc(&mut self, data: Byte) {
        let input_carry = u16::from(self.status_register.contains(StatusRegister::CARRY));
        let sum_wide = self.accumulator.as_word() + data.as_word() + input_carry;
        let result = Byte::from_word_lossy(sum_wide);

        self.status_register
            .set_carry_flag(sum_wide > 0xff)
            .set_overflow_flag((data ^ result) & (result ^ self.accumulator) & 0x80 != 0);

        self.accumulator = result;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn compare(&mut self, address: Address, register: Byte) -> Result<()> {
        let value = self.read_byte(address)?;
        let result = register.wrapping_sub(value);

        self.status_register
            .set_carry_flag(value <= register)
            .update_zero_and_negative_flags(result);

        Ok(())
    }

    fn and(&mut self, address: Address) -> Result<()> {
        let and = |acc, value| acc & value;
        self.logical_op_with_acc(address, and)
            .with_context(|| "AND")?;

        Ok(())
    }

    fn eor(&mut self, address: Address) -> Result<()> {
        let xor = |acc, value| acc ^ value;
        self.logical_op_with_acc(address, xor)
            .with_context(|| "EOR")?;

        Ok(())
    }
    fn ora(&mut self, address: Address) -> Result<()> {
        let or = |acc, value| acc | value;
        self.logical_op_with_acc(address, or)
            .with_context(|| "ORA")?;

        Ok(())
    }

    fn logical_op_with_acc(
        &mut self,
        address: Address,
        logical_op: impl Fn(Byte, Byte) -> Byte,
    ) -> Result<()> {
        let value = self.read_byte(address)?;

        self.accumulator = logical_op(self.accumulator, value);
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);

        Ok(())
    }

    fn bit(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;

        self.status_register
            .set_overflow_flag(value.nth_bit::<6>())
            .set_negative_flag(value.nth_bit::<7>())
            .set_zero_flag(value & self.accumulator == 0);

        Ok(())
    }

    fn asl(&mut self, address: Address, mode: AddressingMode) -> Result<()> {
        let ByteUpdate { previous, new } = self.shift(address, mode, 0.into(), |byte| byte << 1)?;

        self.status_register
            .set_carry_flag(previous.nth_bit::<7>())
            .update_zero_and_negative_flags(new);

        Ok(())
    }

    fn lsr(&mut self, address: Address, mode: AddressingMode) -> Result<()> {
        let ByteUpdate { previous: old, new } =
            self.shift(address, mode, 0.into(), |byte| byte >> 1)?;

        self.status_register
            .set_carry_flag(old.nth_bit::<0>())
            .update_zero_and_negative_flags(new);

        Ok(())
    }

    fn rol(&mut self, address: Address, mode: AddressingMode) -> Result<()> {
        let input_carry = u8::from(self.status_register.contains(StatusRegister::CARRY)).into();
        let ByteUpdate { previous: old, new } =
            self.shift(address, mode, input_carry, |byte| byte << 1)?;

        self.status_register
            .set_carry_flag(old.nth_bit::<7>())
            .update_zero_and_negative_flags(new);

        Ok(())
    }

    fn ror(&mut self, address: Address, mode: AddressingMode) -> Result<()> {
        let input_carry = self.status_register.contains(StatusRegister::CARRY);
        let input_carry = Byte::new(u8::from(input_carry) * 0b1000_0000);
        let ByteUpdate { previous, new } =
            self.shift(address, mode, input_carry, |byte| byte >> 1)?;

        self.status_register
            .set_carry_flag(previous.nth_bit::<0>())
            .update_zero_and_negative_flags(new);

        Ok(())
    }

    fn shift(
        &mut self,
        address: Address,
        mode: AddressingMode,
        input_carry: Byte,
        shift_op: impl Fn(Byte) -> Byte,
    ) -> Result<ByteUpdate> {
        let byte_update = match mode {
            AddressingMode::Accumulator => {
                let previous_accumulator = self.accumulator;
                self.accumulator = shift_op(self.accumulator) | input_carry;

                ByteUpdate {
                    previous: previous_accumulator,
                    new: self.accumulator,
                }
            }
            _ => {
                let value = self.read_byte(address)?;
                let shifted = shift_op(value) | input_carry;

                // RMW: dummy write of the original value before the real write.
                self.write_byte(address, value)?;
                self.write_byte(address, shifted)?;

                ByteUpdate {
                    previous: value,
                    new: shifted,
                }
            }
        };

        Ok(byte_update)
    }

    fn lda(&mut self, address: Address) -> Result<()> {
        self.accumulator = self.load_value(address)?;

        Ok(())
    }

    fn ldx(&mut self, address: Address) -> Result<()> {
        self.register_x = self.load_value(address)?;

        Ok(())
    }

    fn ldy(&mut self, address: Address) -> Result<()> {
        self.register_y = self.load_value(address)?;

        Ok(())
    }

    fn lax(&mut self, address: Address) -> Result<()> {
        self.accumulator = self.load_value(address)?;
        self.register_x = self.accumulator;

        Ok(())
    }

    fn sax(&mut self, address: Address) -> Result<()> {
        let result = self.accumulator & self.register_x;
        self.write_byte(address, result)?;

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

    fn dec(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        let decremented = value.wrapping_sub(1);

        self.write_byte(address, value)?; // dummy write
        self.write_byte(address, decremented)?;
        self.status_register
            .update_zero_and_negative_flags(decremented);

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

    fn inc(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        let incremented = value.wrapping_add(1);

        self.write_byte(address, value)?; // dummy write
        self.write_byte(address, incremented)?;
        self.status_register
            .update_zero_and_negative_flags(incremented);

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

    fn jsr(&mut self) -> Result<()> {
        self.push_word_to_stack((self.program_counter + 1).as_word())?;
        let target_address = self.read_word(self.program_counter)?;
        self.program_counter = target_address.as_address();

        Ok(())
    }

    fn rti(&mut self) -> Result<()> {
        self.status_register = self.pop_byte_from_stack()?.into();
        self.status_register.remove(StatusRegister::BREAK);
        self.status_register.insert(StatusRegister::BREAK2);

        self.program_counter = self.pop_word_from_stack()?.as_address();

        Ok(())
    }

    fn rts(&mut self) -> Result<()> {
        self.program_counter = (self.pop_word_from_stack()? + 1).as_address();

        Ok(())
    }

    fn pla(&mut self) -> Result<()> {
        let value = self.pop_byte_from_stack()?;

        self.accumulator = value;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);

        Ok(())
    }

    fn plp(&mut self) -> Result<()> {
        let value = self.pop_byte_from_stack()?;

        self.status_register = StatusRegister::from(value);
        self.status_register.remove(StatusRegister::BREAK);
        self.status_register.insert(StatusRegister::BREAK2);

        Ok(())
    }

    fn php(&mut self) -> Result<()> {
        let mut status_register_with_b_flags = self.status_register;
        status_register_with_b_flags.insert(StatusRegister::BREAK | StatusRegister::BREAK2);

        self.push_byte_to_stack(status_register_with_b_flags.bits().into()) // TODO?
    }

    fn branch(&mut self, condition: bool) -> Result<()> {
        if condition {
            self.bus.tick(1)?;

            let jump = self.read_byte(self.program_counter)?.value().cast_signed();
            // NOTE: This is intended!
            #[allow(clippy::cast_sign_loss)]
            let jump_addr = self.program_counter.wrapping_add(1 + jump as u16);

            if is_page_crossed(self.program_counter, jump_addr) {
                self.bus.tick(1)?;
            }

            self.program_counter = jump_addr;
        }

        Ok(())
    }

    pub fn pc_operand_address(&mut self, opcode: &Opcode) -> Result<Address> {
        self.operand_address(opcode, self.program_counter)
    }

    pub fn operand_address(&mut self, opcode: &Opcode, address: Address) -> Result<Address> {
        Ok(match opcode.addressing_mode {
            AddressingMode::Immediate => address,
            AddressingMode::ZeroPage => self.read_byte(address)?.into(),
            AddressingMode::Absolute => self.read_word(address)?.as_address(),
            AddressingMode::ZeroPageX => {
                let pos = self.read_byte(address)?;
                let addr = pos.wrapping_add(self.register_x);

                addr.into()
            }

            AddressingMode::ZeroPageY => {
                let pos = self.read_byte(address)?;
                let addr = pos.wrapping_add(self.register_y);

                addr.into()
            }
            AddressingMode::AbsoluteX => {
                let base = self.read_word(address)?.as_address();
                let incremented = base.wrapping_add(self.register_x);

                if is_page_crossed(base, incremented) {
                    // On a page cross the CPU reads the unfixed address first (dummy read),
                    // then corrects to the real address. The read has side effects (e.g.
                    // clearing PPU status vblank). For read-only ops this is the extra cycle;
                    // for RMW/write ops (needs_page_cross_check=false) it also always fires.
                    let unfixed = (base & 0xFF00) | (incremented & 0x00FF);
                    self.read_byte(unfixed)?;
                    self.bus.tick(1)?;
                } else if !opcode.needs_page_cross_check {
                    // No page cross but RMW/write op: dummy read at the same address.
                    self.read_byte(incremented)?;
                }

                incremented
            }
            AddressingMode::AbsoluteY => {
                let base = self.read_word(address)?.as_address();
                let incremented = base.wrapping_add(self.register_y);

                if is_page_crossed(base, incremented) {
                    let unfixed = (base & 0xFF00) | (incremented & 0x00FF);
                    self.read_byte(unfixed)?;
                    self.bus.tick(1)?;
                } else if !opcode.needs_page_cross_check {
                    self.read_byte(incremented)?;
                }

                incremented
            }
            AddressingMode::IndirectX => {
                let base = self.read_byte(address)?;
                let ptr = base.wrapping_add(self.register_x);
                let low = self.read_byte(ptr.into())?;
                let high = self.read_byte(ptr.wrapping_add(1).into())?;

                Word::from_le_bytes(low, high).as_address()
            }
            AddressingMode::IndirectY => {
                let base = self.read_byte(address)?;
                let low = self.read_byte(base.into())?;
                let high = self.read_byte(base.wrapping_add(1).into())?;
                let deref_base = Word::from_le_bytes(low, high).as_address();
                let incremented = deref_base.wrapping_add(self.register_y);

                if is_page_crossed(deref_base, incremented) {
                    let unfixed = Address::new(
                        (deref_base.value() & 0xFF00) | (incremented.value() & 0x00FF),
                    );
                    self.read_byte(unfixed)?;
                    self.bus.tick(1)?;
                } else if !opcode.needs_page_cross_check {
                    self.read_byte(incremented)?;
                }

                incremented
            }
            AddressingMode::Indirect => {
                let target_address = self.read_word(address)?.as_address();

                // recreate the CPU bug with page boundaries:
                // "The indirect jump instruction does not increment the page address when the indirect pointer
                // crosses a page boundary.
                // JMP ($xxFF) will fetch the address from $xxFF and $xx00."
                if target_address & 0x00ff == 0x00ff {
                    let low = self.read_byte(target_address)?;
                    let buggy_address = target_address & 0xff00;
                    let high = self.read_byte(buggy_address)?;

                    Word::from_le_bytes(low, high).as_address()
                } else {
                    self.read_word(target_address)?.as_address()
                }
            }
            _ => 0u16.into(),
        })
    }

    fn load_value(&mut self, address: Address) -> Result<Byte> {
        let value = self.read_byte(address)?;
        self.status_register.update_zero_and_negative_flags(value);

        Ok(value)
    }

    fn push_byte_to_stack(&mut self, byte: Byte) -> Result<()> {
        self.write_byte(self.stack_pointer.address(), byte)?;

        self.stack_pointer.decrement();

        Ok(())
    }

    fn push_word_to_stack(&mut self, word: Word) -> Result<()> {
        let [low, high] = word.to_le_bytes();

        self.push_byte_to_stack(high)?;
        self.push_byte_to_stack(low)?;

        Ok(())
    }

    fn pop_byte_from_stack(&mut self) -> Result<Byte> {
        self.stack_pointer.increment();
        self.read_byte(self.stack_pointer.address())
    }

    fn pop_word_from_stack(&mut self) -> Result<Word> {
        let low_byte = self.pop_byte_from_stack()?;
        let high_byte = self.pop_byte_from_stack()?;

        Ok(Word::from_le_bytes(low_byte, high_byte))
    }

    fn interrupt(&mut self, interrupt: &Interrupt) -> Result<()> {
        self.push_word_to_stack(self.program_counter.as_word())?;
        let mut status = self.status_register;
        status.remove(StatusRegister::BREAK | StatusRegister::BREAK2);
        status |= StatusRegister::from_bits_truncate(interrupt.break_flag_mask.value());

        self.push_byte_to_stack(status.bits().into())?;
        self.status_register.set_interrupt_flag(true);

        self.bus.tick(interrupt.cpu_cycles)?;
        self.program_counter = self.read_word(interrupt.vector_addr)?.as_address();

        Ok(())
    }

    fn dcp(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        let decremented = value.wrapping_sub(1);
        self.write_byte(address, decremented)?;

        self.compare(address, self.accumulator)?;

        Ok(())
    }

    fn isb(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        let incremented = value.wrapping_add(1);

        self.write_byte(address, incremented)?;
        self.sbc(address)?;

        Ok(())
    }

    fn slo(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        let shifted_left = value << 1;
        self.status_register.set_carry_flag(value.nth_bit::<7>());

        self.write_byte(address, shifted_left)?;
        self.ora(address)?;

        Ok(())
    }

    fn rla(&mut self, address: Address, mode: AddressingMode) -> Result<()> {
        self.rol(address, mode)?;
        let value = self.read_byte(address)?;

        self.accumulator &= value;

        Ok(())
    }

    fn sre(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        let shifted_right = value >> 1;

        self.status_register.set_carry_flag(value.nth_bit::<0>());
        self.write_byte(address, shifted_right)?;
        self.eor(address)?;

        Ok(())
    }

    fn rra(&mut self, address: Address, mode: AddressingMode) -> Result<()> {
        self.ror(address, mode)?;
        let value = self.read_byte(address)?;
        self.add_to_acc(value);

        Ok(())
    }

    fn anc(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        self.accumulator &= value;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
        self.status_register.set(
            StatusRegister::CARRY,
            self.status_register.contains(StatusRegister::NEGATIVE),
        );
        Ok(())
    }

    fn alr(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        let and = self.accumulator & value;
        self.status_register
            .set(StatusRegister::CARRY, (and & 1) == 1);
        self.accumulator = and >> 1;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
        Ok(())
    }

    fn arr(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        let and = (self.accumulator & value).value();
        let carry_in = u8::from(self.status_register.contains(StatusRegister::CARRY));
        let result = (carry_in << 7) | (and >> 1);
        self.accumulator = result.into();
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
        let bit6 = (result >> 6) & 1 != 0;
        let bit5 = (result >> 5) & 1 != 0;
        self.status_register.set(StatusRegister::CARRY, bit6);
        self.status_register
            .set(StatusRegister::OVERFLOW, bit6 ^ bit5);
        Ok(())
    }

    fn ane(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        self.accumulator = (self.accumulator | Byte::new(0xEE)) & self.register_x & value;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
        Ok(())
    }

    fn lxa(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        let result = (self.accumulator | Byte::new(0xEE)) & value;
        self.accumulator = result;
        self.register_x = result;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
        Ok(())
    }

    fn axs(&mut self, address: Address) -> Result<()> {
        let value = self.read_byte(address)?;
        let ax = self.accumulator & self.register_x;
        let result = ax.wrapping_sub(value);
        self.status_register.set(StatusRegister::CARRY, ax >= value);
        self.register_x = result;
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
        Ok(())
    }
}

fn is_page_crossed(before: Address, after: Address) -> bool {
    (before & 0xff00) != (after & 0xff00)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Rom;
    use once_cell::sync::Lazy;

    pub static TEST_ROM: Lazy<Vec<u8>> = Lazy::new(|| {
        let mut rom = vec![];
        let header = vec![
            0x4e, 0x45, 0x53, 0x1a, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let prg_rom = vec![0x00; 2 * 16384];
        let chr_rom = vec![0x00; 8192];

        rom.extend(header);
        rom.extend(prg_rom);
        rom.extend(chr_rom);

        rom
    });

    #[derive(Debug)]
    enum Write {
        Byte(Address, Byte),
        Word(Address, Word),
    }

    struct CpuBuilder {
        writes: Vec<Write>,
    }

    impl CpuBuilder {
        fn new() -> Self {
            Self { writes: vec![] }
        }

        fn write_byte(mut self, address: Address, byte: Byte) -> Self {
            self.writes.push(Write::Byte(address, byte));

            self
        }

        fn write_word(mut self, address: impl Into<Address>, word: impl Into<Word>) -> Self {
            self.writes.push(Write::Word(address.into(), word.into()));

            self
        }

        fn build_and_run(self, data: &[u8]) -> Cpu {
            let rom = Rom::from_bytes(&TEST_ROM).expect("Failed to parse test ROM");
            let bus = Bus::new(rom);
            let mut cpu = Cpu::new(bus);
            cpu.status_register = StatusRegister::empty();

            for write in self.writes {
                match write {
                    Write::Byte(address, value) => cpu
                        .write_byte(address, value)
                        .expect("Failed to write byte"),
                    Write::Word(address, value) => cpu
                        .write_word(address, value)
                        .expect("Failed to write word"),
                }
            }

            let data = data.iter().map(|&byte| Byte::new(byte)).collect::<Vec<_>>(); // TODO?

            cpu.load(&data).expect("Failed to load");
            cpu.reset().expect("Failed to reset");
            cpu.program_counter = PROGRAM_ROM_BEGIN_ADDR;
            loop {
                let code = cpu
                    .read_byte(cpu.program_counter)
                    .expect("Failed to read opcode");
                if code.value() == 0x00 {
                    break; // BRK — test program is done
                }
                cpu.step().expect("Failed to step");
            }

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
            let cpu = CpuBuilder::new()
                .write_byte(Address::new(0x10), 0x55.into())
                .build_and_run(&data);

            assert_eq!(cpu.accumulator, 0x55);
        }

        #[test]
        fn ldx_absolute() {
            let data = [0xae, 0x34, 0x12, 0x00];
            let cpu = CpuBuilder::new()
                .write_word(0x1234u16, 0xff)
                .build_and_run(&data);

            assert_eq!(cpu.register_x, 0xff);
            assert!(!cpu.status_register.contains(StatusRegister::ZERO));
            assert!(cpu.status_register.contains(StatusRegister::NEGATIVE));
        }

        #[test]
        fn ldy_zero_page() {
            let data = [0xa4, 0xaa, 0x00];
            let cpu = CpuBuilder::new()
                .write_byte(Address::new(0xaa), 0x66.into())
                .build_and_run(&data);

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

            assert_eq!(cpu.read_word(Address::new(0x1234)).unwrap(), 0x75);
            assert!(cpu.status_register.is_empty());
        }

        #[test]
        fn stx_zero_page() {
            // 1. Store 0x12 in memory location 0xee (setup)
            // 2. Store register X value (0) in memory location 0xee
            let data = [0x86, 0xee, 0x00];
            let address = Address::new(0xee);
            let mut cpu = CpuBuilder::new()
                .write_byte(address, 0x12.into())
                .build_and_run(&data);

            assert_eq!(cpu.read_byte(address).unwrap(), 0x00);
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
            let mut cpu = CpuBuilder::new()
                .write_byte(Address::new(0x01), 0x02.into())
                .write_byte(Address::new(0x03), 0x04.into())
                .build_and_run(&data);

            assert_eq!(cpu.register_x, 0x02);
            assert_eq!(cpu.register_y, 0x04);
            assert_eq!(cpu.read_byte(Address::new(0x02)).unwrap(), 0x04);
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
            let address = Address::new(0x11);
            let mut cpu = CpuBuilder::new()
                .write_byte(address, 0xf1.into())
                .build_and_run(&data);

            assert_eq!(cpu.read_byte(address).unwrap(), 0xf0);
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

            assert_eq!(cpu.read_byte(Address::new(0x1234)).unwrap(), 0x02);
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
                .write_byte(Address::new(0xdd), 0b1110_1010.into())
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
                .write_byte(Address::new(0x1aef), 0b1010_0000.into())
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
                .write_byte(Address::new(0xcc), 0b0011_1011.into())
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
            let address = Address::new(0xab);
            let mut cpu = CpuBuilder::new()
                .write_byte(address, 0b0100_1101.into())
                .build_and_run(&data);

            assert_eq!(cpu.register_x, 1);
            assert_eq!(cpu.read_byte(address).unwrap(), 0b1001_1010);
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
            let address = Address::new(0x0ada);
            let mut cpu = CpuBuilder::new()
                .write_byte(address, 0b0101_0111.into())
                .build_and_run(&data);

            assert_eq!(cpu.read_byte(address).unwrap(), 0b0010_1011);
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
            let address = Address::new(0xff);
            let mut cpu = CpuBuilder::new()
                .write_byte(address, 0b1010_1101.into())
                .build_and_run(&data);

            assert_eq!(cpu.read_byte(address).unwrap(), 0b0101_1011);
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
            let address = Address::new(0x1234);
            let mut cpu = CpuBuilder::new()
                .write_byte(address, 0b0100_1101.into())
                .build_and_run(&data);

            assert_eq!(cpu.read_byte(address).unwrap(), 0b0010_0110);
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
            let cpu = CpuBuilder::new()
                .write_byte(Address::new(0x11), 0xaa.into())
                .build_and_run(&data);

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
            let cpu = CpuBuilder::new()
                .write_byte(Address::new(0x1ede), 0x11.into())
                .build_and_run(&data);

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
            let cpu = CpuBuilder::new()
                .write_byte(Address::new(0xdd), 0xfe.into())
                .build_and_run(&data);

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

            assert_eq!(cpu.program_counter, 0x1233);
            assert_eq!(cpu.status_register, StatusRegister::empty());
        }

        #[test]
        fn jmp_indirect() {
            let data = [0x6c, 0x34, 0x12];
            let cpu = CpuBuilder::new()
                .write_word(0x1234u16, 0xbeee)
                .build_and_run(&data);

            assert_eq!(cpu.program_counter, 0xbeee);
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
