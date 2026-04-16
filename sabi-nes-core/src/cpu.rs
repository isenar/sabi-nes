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
    fn read_byte(&mut self, addr: Address) -> Byte {
        self.bus.read_byte(addr)
    }

    fn write_byte(&mut self, addr: Address, value: Byte) {
        self.bus.write_byte(addr, value);
    }

    fn read_word(&mut self, addr: Address) -> Word {
        self.bus.read_word(addr)
    }

    fn write_word(&mut self, addr: Address, word: Word) {
        self.bus.write_word(addr, word);
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

    pub fn load(&mut self, data: &[Byte]) {
        for (index, &value) in data.iter().enumerate() {
            let index = u16::try_from(index).unwrap();
            let addr = Address::from(index);
            self.write_byte(addr + PROGRAM_ROM_BEGIN_ADDR, value);
        }
    }

    /// Execute a single CPU instruction.
    pub fn step(&mut self) -> Result<()> {
        // Handle NMI interrupt if pending
        if self.bus.poll_nmi_status() == NmiStatus::Active {
            self.interrupt(&interrupts::NMI);
        }

        // Handle IRQ if pending and interrupt flag is clear
        if self.bus.poll_irq_status()
            && !self
                .status_register
                .contains(StatusRegister::INTERRUPT_DISABLE)
        {
            self.interrupt(&interrupts::IRQ);
        }

        let code = self.read_byte(self.program_counter);
        let instruction_pc = self.program_counter;
        self.program_counter = self.program_counter.wrapping_add(1u16);
        self.bus.tick_one(); // opcode fetch cycle

        let current_program_counter = self.program_counter;
        let opcode = OPCODES_MAPPING
            .get(&code)
            .ok_or_else(|| anyhow!("Unknown opcode: {code:02X} at PC ${instruction_pc:04X}"))?;
        let address = self
            .pc_operand_address(opcode)
            .with_context(|| format!("Failed to fetch address for {}", opcode.name))?;
        let opcode_name = opcode.name;

        match opcode_name {
            "ADC" => self.adc(address),
            "AND" => self.and(address),
            "ASL" => self.asl(address, opcode.addressing_mode),
            "BIT" => self.bit(address),
            "BCC" => self.branch(!self.status_register.contains(StatusRegister::CARRY)),
            "BCS" => self.branch(self.status_register.contains(StatusRegister::CARRY)),
            "BEQ" => self.branch(self.status_register.contains(StatusRegister::ZERO)),
            "BMI" => self.branch(self.status_register.contains(StatusRegister::NEGATIVE)),
            "BNE" => self.branch(!self.status_register.contains(StatusRegister::ZERO)),
            "BPL" => self.branch(!self.status_register.contains(StatusRegister::NEGATIVE)),
            "BVC" => self.branch(!self.status_register.contains(StatusRegister::OVERFLOW)),
            "BVS" => self.branch(self.status_register.contains(StatusRegister::OVERFLOW)),
            "BRK" => {
                self.program_counter = self.program_counter.wrapping_add(1u16); // skip the padding byte (BRK is a 2-byte instruction)
                self.interrupt(&interrupts::BRK);
                self.bus.tick(0); // drain any pending OAM DMA cycles
                return Ok(());
            }
            "CLC" => {
                self.bus.tick_one(); // internal cycle
                self.status_register.set_carry_flag(false);
            }
            "CLD" => {
                self.bus.tick_one(); // internal cycle
                self.status_register.set_decimal_flag(false);
            }
            "CLI" => {
                self.bus.tick_one(); // internal cycle
                self.status_register.set_interrupt_flag(false);
            }
            "CLV" => {
                self.bus.tick_one(); // internal cycle
                self.status_register.set_overflow_flag(false);
            }
            "CMP" => self.compare(address, self.accumulator),
            "CPX" => self.compare(address, self.register_x),
            "CPY" => self.compare(address, self.register_y),
            "DEC" => self.dec(address),
            "DEX" => self.dex(),
            "DEY" => self.dey(),
            "EOR" => self.eor(address),
            "INC" => self.inc(address),
            "INX" => self.inx(),
            "INY" => self.iny(),
            "JMP" => self.program_counter = address,
            "JSR" => self.jsr(),
            "LDA" => self.lda(address),
            "LDX" => self.ldx(address),
            "LDY" => self.ldy(address),
            "LSR" => self.lsr(address, opcode.addressing_mode),
            "NOP" | "*NOP" => {
                self.bus.tick_one(); // internal cycle
            }
            "ORA" => self.ora(address),
            "PHA" => {
                self.bus.tick_one(); // internal cycle
                self.write_byte(self.stack_pointer.address(), self.accumulator);
                self.stack_pointer.decrement();
                self.bus.tick_one(); // push cycle
            }
            "PHP" => self.php(),
            "PLA" => self.pla(),
            "PLP" => self.plp(),
            "ROL" => self.rol(address, opcode.addressing_mode),
            "ROR" => self.ror(address, opcode.addressing_mode),
            "RTI" => {
                self.rti();
                self.bus.tick(0); // drain any pending OAM DMA cycles
                return Ok(());
            }
            "RTS" => {
                self.rts();
                self.bus.tick(0); // drain any pending OAM DMA cycles
                return Ok(());
            }
            "SBC" | "*SBC" => self.sbc(address),
            "SEC" => {
                self.bus.tick_one(); // internal cycle
                self.status_register.set_carry_flag(true);
            }
            "SED" => {
                self.bus.tick_one(); // internal cycle
                self.status_register.set_decimal_flag(true);
            }
            "SEI" => {
                self.bus.tick_one(); // internal cycle
                self.status_register.set_interrupt_flag(true);
            }
            "STA" => {
                self.write_byte(address, self.accumulator);
                self.bus.tick_one(); // data write cycle
            }
            "STX" => {
                self.write_byte(address, self.register_x);
                self.bus.tick_one(); // data write cycle
            }
            "STY" => {
                self.write_byte(address, self.register_y);
                self.bus.tick_one(); // data write cycle
            }
            "TAX" => self.tax(),
            "TAY" => self.tay(),
            "TSX" => self.tsx(),
            "TXA" => self.txa(),
            "TXS" => {
                self.bus.tick_one(); // internal cycle
                self.stack_pointer.set(self.register_x);
            }
            "TYA" => self.tya(),

            "*LAX" => self.lax(address),
            "*SAX" => self.sax(address),
            "*DCP" => self.dcp(address),
            "*ISB" => self.isb(address),
            "*SLO" => self.slo(address),
            "*RLA" => self.rla(address, opcode.addressing_mode),
            "*SRE" => self.sre(address),
            "*RRA" => self.rra(address, opcode.addressing_mode),
            "*ANC" => self.anc(address),
            "*ALR" => self.alr(address),
            "*ARR" => self.arr(address),
            "*ANE" => self.ane(address),
            "*LXA" => self.lxa(address),
            "*AXS" => self.axs(address),
            "*SHA" | "*SHX" | "*SHY" | "*SHS" | "*LAS" => {} // unstable — treat as NOP
            _ => bail!("Unsupported opcode name: {opcode_name}"),
        }

        // Drain any pending cycles accumulated during the instruction (e.g. OAM DMA stall).
        self.bus.tick(0);

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
        self.program_counter = self.read_word(RESET_VECTOR_BEGIN_ADDR).as_address();
        debug!("CPU reset: PC set to ${:04X}", self.program_counter);
        self.stack_pointer.reset();

        Ok(())
    }

    fn adc(&mut self, address: Address) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // data read cycle
        self.add_to_acc(value);
    }

    fn sbc(&mut self, address: Address) {
        let negated = self
            .read_byte(address)
            .value()
            .cast_signed()
            .wrapping_neg()
            .wrapping_sub(1)
            .cast_unsigned();
        self.bus.tick_one(); // data read cycle

        self.add_to_acc(negated.into());
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

    fn compare(&mut self, address: Address, register: Byte) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // data read cycle
        let result = register.wrapping_sub(value);

        self.status_register
            .set_carry_flag(value <= register)
            .update_zero_and_negative_flags(result);
    }

    fn and(&mut self, address: Address) {
        let and = |acc, value| acc & value;
        self.logical_op_with_acc(address, and);
    }

    fn eor(&mut self, address: Address) {
        let xor = |acc, value| acc ^ value;
        self.logical_op_with_acc(address, xor);
    }

    fn ora(&mut self, address: Address) {
        let or = |acc, value| acc | value;
        self.logical_op_with_acc(address, or);
    }

    fn logical_op_with_acc(&mut self, address: Address, logical_op: impl Fn(Byte, Byte) -> Byte) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // data read cycle

        self.accumulator = logical_op(self.accumulator, value);
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn bit(&mut self, address: Address) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // data read cycle

        self.status_register
            .set_overflow_flag(value.nth_bit::<6>())
            .set_negative_flag(value.nth_bit::<7>())
            .set_zero_flag(value & self.accumulator == 0);
    }

    fn asl(&mut self, address: Address, mode: AddressingMode) {
        let ByteUpdate { previous, new } = self.shift(address, mode, 0.into(), |byte| byte << 1);

        self.status_register
            .set_carry_flag(previous.nth_bit::<7>())
            .update_zero_and_negative_flags(new);
    }

    fn lsr(&mut self, address: Address, mode: AddressingMode) {
        let ByteUpdate { previous, new } = self.shift(address, mode, 0.into(), |byte| byte >> 1);

        self.status_register
            .set_carry_flag(previous.nth_bit::<0>())
            .update_zero_and_negative_flags(new);
    }

    fn rol(&mut self, address: Address, mode: AddressingMode) {
        let input_carry = u8::from(self.status_register.contains(StatusRegister::CARRY)).into();
        let ByteUpdate { previous, new } = self.shift(address, mode, input_carry, |byte| byte << 1);

        self.status_register
            .set_carry_flag(previous.nth_bit::<7>())
            .update_zero_and_negative_flags(new);
    }

    fn ror(&mut self, address: Address, mode: AddressingMode) {
        let input_carry = self.status_register.contains(StatusRegister::CARRY);
        let input_carry = match input_carry {
            true => Byte::new(0b1000_0000),
            false => Byte::new(0b0000_0000),
        };
        let ByteUpdate { previous, new } = self.shift(address, mode, input_carry, |byte| byte >> 1);

        self.status_register
            .set_carry_flag(previous.nth_bit::<0>())
            .update_zero_and_negative_flags(new);
    }

    fn shift(
        &mut self,
        address: Address,
        mode: AddressingMode,
        input_carry: Byte,
        shift_op: impl Fn(Byte) -> Byte,
    ) -> ByteUpdate {
        match mode {
            AddressingMode::Accumulator => {
                self.bus.tick_one(); // internal cycle
                let previous_accumulator = self.accumulator;
                self.accumulator = shift_op(self.accumulator) | input_carry;

                ByteUpdate {
                    previous: previous_accumulator,
                    new: self.accumulator,
                }
            }
            _ => {
                let value = self.read_byte(address);
                self.bus.tick_one(); // read cycle
                let shifted = shift_op(value) | input_carry;

                // RMW: dummy write of the original value before the real write.
                self.write_byte(address, value);
                self.bus.tick_one(); // dummy write cycle
                self.write_byte(address, shifted);
                self.bus.tick_one(); // real write cycle

                ByteUpdate {
                    previous: value,
                    new: shifted,
                }
            }
        }
    }

    fn lda(&mut self, address: Address) {
        self.accumulator = self.load_value(address);
    }

    fn ldx(&mut self, address: Address) {
        self.register_x = self.load_value(address);
    }

    fn ldy(&mut self, address: Address) {
        self.register_y = self.load_value(address);
    }

    fn lax(&mut self, address: Address) {
        self.accumulator = self.load_value(address);
        self.register_x = self.accumulator;
    }

    fn sax(&mut self, address: Address) {
        let result = self.accumulator & self.register_x;
        self.write_byte(address, result);
        self.bus.tick_one(); // data write cycle
    }

    fn tax(&mut self) {
        self.bus.tick_one(); // internal cycle
        self.register_x = self.accumulator;
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn tay(&mut self) {
        self.bus.tick_one(); // internal cycle
        self.register_y = self.accumulator;
        self.status_register
            .update_zero_and_negative_flags(self.register_y);
    }

    fn tsx(&mut self) {
        self.bus.tick_one(); // internal cycle
        self.register_x = self.stack_pointer.value();
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn txa(&mut self) {
        self.bus.tick_one(); // internal cycle
        self.accumulator = self.register_x;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn tya(&mut self) {
        self.bus.tick_one(); // internal cycle
        self.accumulator = self.register_y;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn dec(&mut self, address: Address) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // read cycle
        let decremented = value.wrapping_sub(1);

        self.write_byte(address, value); // dummy write
        self.bus.tick_one(); // dummy write cycle
        self.write_byte(address, decremented);
        self.bus.tick_one(); // real write cycle
        self.status_register
            .update_zero_and_negative_flags(decremented);
    }

    fn dex(&mut self) {
        self.bus.tick_one(); // internal cycle
        self.register_x = self.register_x.wrapping_sub(1);
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn dey(&mut self) {
        self.bus.tick_one(); // internal cycle
        self.register_y = self.register_y.wrapping_sub(1);
        self.status_register
            .update_zero_and_negative_flags(self.register_y);
    }

    fn inc(&mut self, address: Address) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // read cycle
        let incremented = value.wrapping_add(1);

        self.write_byte(address, value); // dummy write
        self.bus.tick_one(); // dummy write cycle
        self.write_byte(address, incremented);
        self.bus.tick_one(); // real write cycle
        self.status_register
            .update_zero_and_negative_flags(incremented);
    }

    fn inx(&mut self) {
        self.bus.tick_one(); // internal cycle
        self.register_x = self.register_x.wrapping_add(1);
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
    }

    fn iny(&mut self) {
        self.bus.tick_one(); // internal cycle
        self.register_y = self.register_y.wrapping_add(1);
        self.status_register
            .update_zero_and_negative_flags(self.register_y);
    }

    fn jsr(&mut self) {
        // JSR 6-cycle breakdown (matches real 6502 behaviour):
        //   Cycle 1: fetch opcode                        — done in step()
        //   Cycle 2: fetch addr_low from PC, PC stays    — done here
        //   Cycle 3: internal read from SP (open-bus)    — done here
        //   Cycle 4: push PCH to stack                   — done here
        //   Cycle 5: push PCL to stack                   — done here
        //   Cycle 6: fetch addr_high from PC+1, jump     — done here (last bus read → correct open bus)
        //
        // addr_high must be fetched LAST so that cpu_open_bus ends up holding addr_high,
        // which is what the real hardware does.  operand_address(Absolute) would fetch
        // both bytes eagerly and in the wrong order relative to the stack writes.

        // Cycle 2: read addr_low
        let addr_low_pos = self.program_counter;
        let addr_low = self.read_byte(addr_low_pos);
        self.bus.tick_one();

        // Cycle 3: internal read from stack pointer (the 6502 reads the stack here, updating
        // the data-bus latch, but discards the value).
        self.read_byte(self.stack_pointer.address());
        self.bus.tick_one();

        // The return address is the address of the high-byte operand (addr_low_pos + 1).
        // RTS will pop this and increment by 1 to land on the instruction after JSR.
        let return_addr = (addr_low_pos + 1u16).as_word();
        let [ret_low, ret_high] = return_addr.to_le_bytes();

        // Cycle 4: push PCH (high byte of return address)
        self.write_byte(self.stack_pointer.address(), ret_high);
        self.stack_pointer.decrement();
        self.bus.tick_one();

        // Cycle 5: push PCL (low byte of return address)
        self.write_byte(self.stack_pointer.address(), ret_low);
        self.stack_pointer.decrement();
        self.bus.tick_one();

        // Cycle 6: read addr_high — this is the last bus access, so cpu_open_bus = addr_high
        let addr_high = self.read_byte(addr_low_pos + 1u16);
        self.bus.tick_one();

        let target = Word::from_le_bytes(addr_low, addr_high).as_address();
        self.program_counter = target;
    }

    fn rti(&mut self) {
        // RTI: opcode(1) + dummy_read(1) + SP_inc(1) + pop_P(1) + pop_PCL(1) + pop_PCH(1) = 6
        self.read_byte(self.program_counter);
        self.bus.tick_one(); // dummy read
        self.stack_pointer.increment();
        self.bus.tick_one(); // SP increment
        let status = self.read_byte(self.stack_pointer.address());
        self.bus.tick_one(); // pop P
        self.stack_pointer.increment();
        let pcl = self.read_byte(self.stack_pointer.address());
        self.bus.tick_one(); // pop PCL
        self.stack_pointer.increment();
        let pch = self.read_byte(self.stack_pointer.address());
        self.bus.tick_one(); // pop PCH

        self.status_register = StatusRegister::from(status);
        self.status_register.remove(StatusRegister::BREAK);
        self.status_register.insert(StatusRegister::BREAK2);
        self.program_counter = Word::from_le_bytes(pcl, pch).as_address();
    }

    fn rts(&mut self) {
        // RTS: opcode(1) + dummy_read(1) + SP_inc(1) + pop_PCL(1) + pop_PCH(1) + inc_PC(1) = 6
        self.read_byte(self.program_counter);
        self.bus.tick_one(); // dummy read
        self.stack_pointer.increment();
        self.bus.tick_one(); // SP increment
        let pcl = self.read_byte(self.stack_pointer.address());
        self.bus.tick_one(); // pop PCL
        self.stack_pointer.increment();
        let pch = self.read_byte(self.stack_pointer.address());
        self.bus.tick_one(); // pop PCH
        self.program_counter = (Word::from_le_bytes(pcl, pch) + 1).as_address();
        self.bus.tick_one(); // increment PC
    }

    fn pla(&mut self) {
        // PLA: opcode(1) + internal(1) + SP_inc(1) + pop(1) = 4
        self.bus.tick_one(); // internal cycle
        self.stack_pointer.increment();
        self.bus.tick_one(); // SP increment
        let value = self.read_byte(self.stack_pointer.address());
        self.bus.tick_one(); // pop cycle

        self.accumulator = value;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn plp(&mut self) {
        // PLP: opcode(1) + internal(1) + SP_inc(1) + pop(1) = 4
        self.bus.tick_one(); // internal cycle
        self.stack_pointer.increment();
        self.bus.tick_one(); // SP increment
        let value = self.read_byte(self.stack_pointer.address());
        self.bus.tick_one(); // pop cycle

        self.status_register = StatusRegister::from(value);
        self.status_register.remove(StatusRegister::BREAK);
        self.status_register.insert(StatusRegister::BREAK2);
    }

    fn php(&mut self) {
        // PHP: opcode(1) + internal(1) + push(1) = 3
        self.bus.tick_one(); // internal cycle
        let mut status_register_with_b_flags = self.status_register;
        status_register_with_b_flags.insert(StatusRegister::BREAK | StatusRegister::BREAK2);
        self.write_byte(
            self.stack_pointer.address(),
            status_register_with_b_flags.bits().into(),
        );
        self.stack_pointer.decrement();
        self.bus.tick_one(); // push cycle
    }

    fn branch(&mut self, condition: bool) {
        let jump = self.read_byte(self.program_counter).value().cast_signed();
        self.bus.tick_one(); // offset byte read cycle

        if condition {
            // NOTE: This is intended!
            #[allow(clippy::cast_sign_loss)]
            let jump_addr = self.program_counter.wrapping_add(1 + jump as u16);

            self.bus.tick_one(); // branch-taken cycle

            if is_page_crossed(self.program_counter, jump_addr) {
                self.bus.tick_one(); // page-cross fixup cycle
            }

            self.program_counter = jump_addr;
        }
    }

    pub fn pc_operand_address(&mut self, opcode: &Opcode) -> Result<Address> {
        self.operand_address(opcode, self.program_counter)
    }

    pub fn operand_address(&mut self, opcode: &Opcode, address: Address) -> Result<Address> {
        Ok(match opcode.addressing_mode {
            AddressingMode::Immediate => address,
            AddressingMode::ZeroPage => {
                let zp = self.read_byte(address);
                self.bus.tick_one();
                zp.into()
            }
            AddressingMode::ZeroPageX => {
                let zp = self.read_byte(address);
                self.bus.tick_one();
                self.bus.tick_one(); // add X (internal)
                zp.wrapping_add(self.register_x).into()
            }
            AddressingMode::ZeroPageY => {
                let zp = self.read_byte(address);
                self.bus.tick_one();
                self.bus.tick_one(); // add Y (internal)
                zp.wrapping_add(self.register_y).into()
            }
            AddressingMode::Absolute => {
                let low = self.read_byte(address);
                self.bus.tick_one();
                let high = self.read_byte(address.wrapping_add(1u16));
                self.bus.tick_one();
                Word::from_le_bytes(low, high).as_address()
            }
            AddressingMode::AbsoluteX => {
                let low = self.read_byte(address);
                self.bus.tick_one();
                let high = self.read_byte(address.wrapping_add(1u16));
                self.bus.tick_one();
                let base = Word::from_le_bytes(low, high).as_address();
                let incremented = base.wrapping_add(self.register_x);

                if is_page_crossed(base, incremented) {
                    // On a page cross the CPU reads the unfixed address first (dummy read),
                    // then corrects to the real address. The read has side effects (e.g.
                    // clearing PPU status vblank). For read-only ops this is the extra cycle;
                    // for RMW/write ops (needs_page_cross_check=false) it also always fires.
                    let unfixed =
                        Address::new((base.value() & 0xFF00) | (incremented.value() & 0x00FF));
                    self.read_byte(unfixed);
                    self.bus.tick_one();
                } else if !opcode.needs_page_cross_check {
                    // No page cross but RMW/write op: dummy read at the same address.
                    self.read_byte(incremented);
                    self.bus.tick_one();
                }

                incremented
            }
            AddressingMode::AbsoluteY => {
                let low = self.read_byte(address);
                self.bus.tick_one();
                let high = self.read_byte(address.wrapping_add(1u16));
                self.bus.tick_one();
                let base = Word::from_le_bytes(low, high).as_address();
                let incremented = base.wrapping_add(self.register_y);

                if is_page_crossed(base, incremented) {
                    let unfixed = (base & 0xFF00) | (incremented & 0x00FF);
                    self.read_byte(unfixed);
                    self.bus.tick_one();
                } else if !opcode.needs_page_cross_check {
                    self.read_byte(incremented);
                    self.bus.tick_one();
                }

                incremented
            }
            AddressingMode::IndirectX => {
                let base = self.read_byte(address);
                self.bus.tick_one();
                self.bus.tick_one(); // add X (internal)
                let ptr = base.wrapping_add(self.register_x);
                let low = self.read_byte(ptr.into());
                self.bus.tick_one();
                let high = self.read_byte(ptr.wrapping_add(1).into());
                self.bus.tick_one();
                Word::from_le_bytes(low, high).as_address()
            }
            AddressingMode::IndirectY => {
                let base = self.read_byte(address);
                self.bus.tick_one();
                let low = self.read_byte(base.into());
                self.bus.tick_one();
                let high = self.read_byte(base.wrapping_add(1).into());
                self.bus.tick_one();
                let deref_base = Word::from_le_bytes(low, high).as_address();
                let incremented = deref_base.wrapping_add(self.register_y);

                if is_page_crossed(deref_base, incremented) {
                    let unfixed = (deref_base & 0xFF00) | (incremented & 0x00FF);
                    self.read_byte(unfixed);
                    self.bus.tick_one();
                } else if !opcode.needs_page_cross_check {
                    self.read_byte(incremented);
                    self.bus.tick_one();
                }

                incremented
            }
            AddressingMode::Indirect => {
                let low_addr = self.read_byte(address);
                self.bus.tick_one();
                let high_addr = self.read_byte(address.wrapping_add(1u16));
                self.bus.tick_one();
                let target = Word::from_le_bytes(low_addr, high_addr).as_address();

                // Reproduce the CPU page-boundary bug:
                // "The indirect jump instruction does not increment the page address
                // when the indirect pointer crosses a page boundary.
                // JMP ($xxFF) will fetch the address from $xxFF and $xx00."
                let low = self.read_byte(target);
                self.bus.tick_one();
                let high_bug_addr = (target & 0xFF00) | ((target + 1) & 0x00FF);
                let high = self.read_byte(high_bug_addr);
                self.bus.tick_one();
                Word::from_le_bytes(low, high).as_address()
            }
            _ => Address::default(),
        })
    }

    fn load_value(&mut self, address: Address) -> Byte {
        let value = self.read_byte(address);
        self.bus.tick_one(); // data read cycle
        self.status_register.update_zero_and_negative_flags(value);

        value
    }

    fn push_byte_to_stack(&mut self, byte: Byte) {
        self.write_byte(self.stack_pointer.address(), byte);

        self.stack_pointer.decrement();
    }

    fn push_word_to_stack(&mut self, word: Word) {
        let [low, high] = word.to_le_bytes();

        self.push_byte_to_stack(high);
        self.push_byte_to_stack(low);
    }

    fn interrupt(&mut self, interrupt: &Interrupt) {
        self.push_word_to_stack(self.program_counter.as_word());
        let mut status = self.status_register;
        status.remove(StatusRegister::BREAK | StatusRegister::BREAK2);
        status |= StatusRegister::from_bits_truncate(interrupt.break_flag_mask.value());

        self.push_byte_to_stack(status.bits().into());
        self.status_register.set_interrupt_flag(true);

        self.bus.tick(interrupt.cpu_cycles);
        self.program_counter = self.read_word(interrupt.vector_addr).as_address();
    }

    fn dcp(&mut self, address: Address) {
        // RMW: read + dummy_write + real_write, then CMP using already-read (decremented) value
        let value = self.read_byte(address);
        self.bus.tick_one(); // read cycle
        let decremented = value.wrapping_sub(1);
        self.write_byte(address, value); // dummy write
        self.bus.tick_one(); // dummy write cycle
        self.write_byte(address, decremented);
        self.bus.tick_one(); // real write cycle

        // Inline CMP using the decremented value (no extra read)
        let result = self.accumulator.wrapping_sub(decremented);
        self.status_register
            .set_carry_flag(decremented <= self.accumulator)
            .update_zero_and_negative_flags(result);
    }

    fn isb(&mut self, address: Address) {
        // RMW: read + dummy_write + real_write, then SBC using already-read (incremented) value
        let value = self.read_byte(address);
        self.bus.tick_one(); // read cycle
        let incremented = value.wrapping_add(1);
        self.write_byte(address, value); // dummy write
        self.bus.tick_one(); // dummy write cycle
        self.write_byte(address, incremented);
        self.bus.tick_one(); // real write cycle

        // Inline SBC using the incremented value (no extra read)
        let negated = incremented
            .value()
            .cast_signed()
            .wrapping_neg()
            .wrapping_sub(1)
            .cast_unsigned();
        self.add_to_acc(negated.into());
    }

    fn slo(&mut self, address: Address) {
        // RMW: read + dummy_write + real_write, then ORA acc with shifted value (no extra read)
        let value = self.read_byte(address);
        self.bus.tick_one(); // read cycle
        let shifted_left = value << 1;
        self.status_register.set_carry_flag(value.nth_bit::<7>());
        self.write_byte(address, value); // dummy write
        self.bus.tick_one(); // dummy write cycle
        self.write_byte(address, shifted_left);
        self.bus.tick_one(); // real write cycle

        // Inline ORA using the shifted value (no extra read)
        self.accumulator |= shifted_left;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn rla(&mut self, address: Address, mode: AddressingMode) {
        // ROL (RMW with ticks), then AND acc with rotated value (no extra read)
        let input_carry = u8::from(self.status_register.contains(StatusRegister::CARRY)).into();
        let ByteUpdate {
            previous: old,
            new: rotated,
        } = self.shift(address, mode, input_carry, |byte| byte << 1);
        self.status_register
            .set_carry_flag(old.nth_bit::<7>())
            .update_zero_and_negative_flags(rotated);

        self.accumulator &= rotated;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn sre(&mut self, address: Address) {
        // RMW: read + dummy_write + real_write, then EOR acc with shifted value (no extra read)
        let value = self.read_byte(address);
        self.bus.tick_one(); // read cycle
        let shifted_right = value >> 1;
        self.status_register.set_carry_flag(value.nth_bit::<0>());
        self.write_byte(address, value); // dummy write
        self.bus.tick_one(); // dummy write cycle
        self.write_byte(address, shifted_right);
        self.bus.tick_one(); // real write cycle

        // Inline EOR using the shifted value (no extra read)
        self.accumulator = self.accumulator ^ shifted_right;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn rra(&mut self, address: Address, mode: AddressingMode) {
        // ROR (RMW with ticks), then ADC acc with rotated value (no extra read)
        let input_carry = self.status_register.contains(StatusRegister::CARRY);
        let input_carry = Byte::new(u8::from(input_carry) * 0b1000_0000);
        let ByteUpdate {
            previous,
            new: rotated,
        } = self.shift(address, mode, input_carry, |byte| byte >> 1);
        self.status_register
            .set_carry_flag(previous.nth_bit::<0>())
            .update_zero_and_negative_flags(rotated);

        self.add_to_acc(rotated);
    }

    fn anc(&mut self, address: Address) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // data read cycle
        self.accumulator &= value;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
        self.status_register.set(
            StatusRegister::CARRY,
            self.status_register.contains(StatusRegister::NEGATIVE),
        );
    }

    fn alr(&mut self, address: Address) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // data read cycle
        let and = self.accumulator & value;
        self.status_register
            .set(StatusRegister::CARRY, and.nth_bit::<0>());
        self.accumulator = and >> 1;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn arr(&mut self, address: Address) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // data read cycle
        let and = self.accumulator & value;
        let carry_in = Byte::from(self.status_register.contains(StatusRegister::CARRY));
        let result = (carry_in << 7) | (and >> 1);
        self.accumulator = result;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
        let carry_bit = result.nth_bit::<6>();
        self.status_register.set(StatusRegister::CARRY, carry_bit);
        self.status_register
            .set(StatusRegister::OVERFLOW, carry_bit ^ result.nth_bit::<5>());
    }

    fn ane(&mut self, address: Address) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // data read cycle
        self.accumulator = (self.accumulator | Byte::new(0xee)) & self.register_x & value;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn lxa(&mut self, address: Address) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // data read cycle
        let result = (self.accumulator | Byte::new(0xee)) & value;
        self.accumulator = result;
        self.register_x = result;
        self.status_register
            .update_zero_and_negative_flags(self.accumulator);
    }

    fn axs(&mut self, address: Address) {
        let value = self.read_byte(address);
        self.bus.tick_one(); // data read cycle
        let ax = self.accumulator & self.register_x;
        let result = ax.wrapping_sub(value);
        self.status_register.set(StatusRegister::CARRY, ax >= value);
        self.register_x = result;
        self.status_register
            .update_zero_and_negative_flags(self.register_x);
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

    static TEST_ROM: Lazy<Vec<u8>> = Lazy::new(|| {
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

        fn write_byte(mut self, address: impl Into<Address>, byte: impl Into<Byte>) -> Self {
            self.writes.push(Write::Byte(address.into(), byte.into()));

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
                    Write::Byte(address, value) => cpu.write_byte(address, value),
                    Write::Word(address, value) => cpu.write_word(address, value),
                }
            }

            let data = data.iter().map(|&byte| Byte::new(byte)).collect::<Vec<_>>();

            cpu.load(&data);
            cpu.reset().expect("Failed to reset");
            cpu.program_counter = PROGRAM_ROM_BEGIN_ADDR;
            loop {
                let code = cpu.read_byte(cpu.program_counter);
                if code == 0x00 {
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
                .write_byte(0x10u16, 0x55)
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
                .write_byte(0xaa_u16, 0x66)
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

            assert_eq!(cpu.read_word(Address::new(0x1234)), 0x75);
            assert!(cpu.status_register.is_empty());
        }

        #[test]
        fn stx_zero_page() {
            // 1. Store 0x12 in memory location 0xee (setup)
            // 2. Store register X value (0) in memory location 0xee
            let data = [0x86, 0xee, 0x00];
            let address = Address::new(0xee);
            let mut cpu = CpuBuilder::new()
                .write_byte(address, 0x12)
                .build_and_run(&data);

            assert_eq!(cpu.read_byte(address), 0x00);
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
                .write_byte(Address::new(0x01), 0x02)
                .write_byte(Address::new(0x03), 0x04)
                .build_and_run(&data);

            assert_eq!(cpu.register_x, 0x02);
            assert_eq!(cpu.register_y, 0x04);
            assert_eq!(cpu.read_byte(Address::new(0x02)), 0x04);
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
                .write_byte(address, 0xf1)
                .build_and_run(&data);

            assert_eq!(cpu.read_byte(address), 0xf0);
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

            assert_eq!(cpu.read_byte(Address::new(0x1234)), 0x02);
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
                .write_byte(Address::new(0xdd), 0b1110_1010)
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
                .write_byte(Address::new(0x1aef), 0b1010_0000)
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
                .write_byte(Address::new(0xcc), 0b0011_1011)
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
                .write_byte(address, 0b0100_1101)
                .build_and_run(&data);

            assert_eq!(cpu.register_x, 1);
            assert_eq!(cpu.read_byte(address), 0b1001_1010);
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
                .write_byte(address, 0b0101_0111)
                .build_and_run(&data);

            assert_eq!(cpu.read_byte(address), 0b0010_1011);
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
                .write_byte(address, 0b1010_1101)
                .build_and_run(&data);

            assert_eq!(cpu.read_byte(address), 0b0101_1011);
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
                .write_byte(address, 0b0100_1101)
                .build_and_run(&data);

            assert_eq!(cpu.read_byte(address), 0b0010_0110);
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
                .write_byte(Address::new(0x11), 0xaa)
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
                .write_byte(Address::new(0x1ede), 0x11)
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
                .write_byte(0xdd_u16, 0xfe)
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
