use crate::cpu::addressing_mode::AddressingMode;
use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Opcode {
    pub code: u8,
    pub name: &'static str,
    pub bytes: u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}

impl Opcode {
    pub const fn new(
        code: u8,
        name: &'static str,
        bytes: u8,
        cycles: u8,
        mode: AddressingMode,
    ) -> Self {
        Self {
            code,
            name,
            bytes,
            cycles,
            mode,
        }
    }

    pub const fn len(&self) -> u16 {
        (self.bytes - 1) as u16
    }
}

const OPCODES: &[Opcode] = &[
    Opcode::new(0x00, "BRK", 1, 7, AddressingMode::Implied),
    Opcode::new(0xea, "NOP", 1, 2, AddressingMode::Implied),
    // -- flag clear/set instructions
    Opcode::new(0x18, "CLC", 1, 2, AddressingMode::Implied),
    Opcode::new(0xd8, "CLD", 1, 2, AddressingMode::Implied),
    Opcode::new(0x58, "CLI", 1, 2, AddressingMode::Implied),
    Opcode::new(0xb8, "CLV", 1, 2, AddressingMode::Implied),
    Opcode::new(0x38, "SEC", 1, 2, AddressingMode::Implied),
    Opcode::new(0xf8, "SED", 1, 2, AddressingMode::Implied),
    Opcode::new(0x78, "SEI", 1, 2, AddressingMode::Implied),
    // -- logical instructions --
    // AND
    Opcode::new(0x29, "AND", 2, 2, AddressingMode::Immediate),
    Opcode::new(0x25, "AND", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0x35, "AND", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0x2d, "AND", 3, 4, AddressingMode::Absolute),
    Opcode::new(0x3d, "AND", 3, 4, AddressingMode::AbsoluteX), // +1 cycle if page boundary crossed
    Opcode::new(0x39, "AND", 3, 4, AddressingMode::AbsoluteY), // +1 cycle if page boundary crossed
    Opcode::new(0x21, "AND", 2, 6, AddressingMode::IndirectX),
    Opcode::new(0x31, "AND", 2, 5, AddressingMode::IndirectY), // +1 cycle if page boundary crossed
    // BIT
    Opcode::new(0x2c, "BIT", 3, 4, AddressingMode::Absolute),
    Opcode::new(0x24, "BIT", 2, 3, AddressingMode::ZeroPage),
    // EOR
    Opcode::new(0x49, "EOR", 2, 2, AddressingMode::Immediate),
    Opcode::new(0x45, "EOR", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0x55, "EOR", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0x4d, "EOR", 3, 4, AddressingMode::Absolute),
    Opcode::new(0x5d, "EOR", 3, 4, AddressingMode::AbsoluteX), // +1 cycle if page boundary crossed
    Opcode::new(0x59, "EOR", 3, 4, AddressingMode::AbsoluteY), // +1 cycle if page boundary crossed
    Opcode::new(0x41, "EOR", 2, 6, AddressingMode::IndirectX),
    Opcode::new(0x51, "EOR", 2, 5, AddressingMode::IndirectY), // +1 cycle if page boundary crossed
    // ORA
    Opcode::new(0x09, "ORA", 2, 2, AddressingMode::Immediate),
    Opcode::new(0x05, "ORA", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0x15, "ORA", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0x0d, "ORA", 3, 4, AddressingMode::Absolute),
    Opcode::new(0x1d, "ORA", 3, 4, AddressingMode::AbsoluteX), // +1 cycle if page boundary crossed
    Opcode::new(0x19, "ORA", 3, 4, AddressingMode::AbsoluteY), // +1 cycle if page boundary crossed
    Opcode::new(0x01, "ORA", 2, 6, AddressingMode::IndirectX),
    Opcode::new(0x11, "ORA", 2, 5, AddressingMode::IndirectY), // +1 cycle if page boundary crossed
    // -- load/store instructions --
    // LDA
    Opcode::new(0xa9, "LDA", 2, 2, AddressingMode::Immediate),
    Opcode::new(0xa5, "LDA", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0xb5, "LDA", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0xad, "LDA", 3, 4, AddressingMode::Absolute),
    Opcode::new(0xbd, "LDA", 3, 4, AddressingMode::AbsoluteX), // +1 cycle if page boundary crossed
    Opcode::new(0xb9, "LDA", 3, 4, AddressingMode::AbsoluteY), // +1 cycle if page boundary crossed
    Opcode::new(0xa1, "LDA", 2, 6, AddressingMode::IndirectX),
    Opcode::new(0xb1, "LDA", 2, 5, AddressingMode::IndirectY), // +1 cycle if page boundary crossed
    // LDX
    Opcode::new(0xa2, "LDX", 2, 2, AddressingMode::Immediate),
    Opcode::new(0xae, "LDX", 3, 4, AddressingMode::Absolute),
    Opcode::new(0xbe, "LDX", 3, 4, AddressingMode::AbsoluteY), // +1 cycle if page boundary crossed
    Opcode::new(0xa6, "LDX", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0xb6, "LDX", 2, 3, AddressingMode::ZeroPageY),
    // LDY
    Opcode::new(0xa0, "LDY", 2, 2, AddressingMode::Immediate),
    Opcode::new(0xac, "LDY", 3, 4, AddressingMode::Absolute),
    Opcode::new(0xbc, "LDY", 3, 4, AddressingMode::AbsoluteX), // +1 cycle if page boundary crossed
    Opcode::new(0xa4, "LDY", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0xb4, "LDY", 2, 4, AddressingMode::ZeroPageX),
    // STA
    Opcode::new(0x85, "STA", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0x95, "STA", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0x8d, "STA", 3, 4, AddressingMode::Absolute),
    Opcode::new(0x9d, "STA", 3, 5, AddressingMode::AbsoluteX),
    Opcode::new(0x99, "STA", 3, 5, AddressingMode::AbsoluteY),
    Opcode::new(0x81, "STA", 2, 6, AddressingMode::IndirectX),
    Opcode::new(0x91, "STA", 2, 6, AddressingMode::IndirectY),
    // STX
    Opcode::new(0x8e, "STX", 3, 4, AddressingMode::Absolute),
    Opcode::new(0x86, "STX", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0x96, "STX", 2, 4, AddressingMode::ZeroPageY),
    // STY
    Opcode::new(0x8c, "STY", 3, 4, AddressingMode::Absolute),
    Opcode::new(0x84, "STY", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0x94, "STY", 2, 4, AddressingMode::ZeroPageX),
    // -- transfer instructions --
    Opcode::new(0xaa, "TAX", 1, 2, AddressingMode::Implied),
    Opcode::new(0xa8, "TAY", 1, 2, AddressingMode::Implied),
    Opcode::new(0xba, "TSX", 1, 2, AddressingMode::Implied),
    Opcode::new(0x8a, "TXA", 1, 2, AddressingMode::Implied),
    Opcode::new(0x9a, "TXS", 1, 2, AddressingMode::Implied),
    Opcode::new(0x98, "TYA", 1, 2, AddressingMode::Implied),
    // -- stack instructions --
    Opcode::new(0x48, "PHA", 1, 3, AddressingMode::Implied),
    Opcode::new(0x08, "PHP", 1, 3, AddressingMode::Implied),
    Opcode::new(0x68, "PLA", 1, 4, AddressingMode::Implied),
    Opcode::new(0x28, "PLP", 1, 4, AddressingMode::Implied),
    // -- increment/decrement instructions --
    Opcode::new(0xce, "DEC", 3, 6, AddressingMode::Absolute),
    Opcode::new(0xde, "DEC", 3, 7, AddressingMode::AbsoluteX),
    Opcode::new(0xc6, "DEC", 2, 5, AddressingMode::ZeroPage),
    Opcode::new(0xd6, "DEC", 2, 6, AddressingMode::ZeroPageX),
    Opcode::new(0xca, "DEX", 1, 2, AddressingMode::Implied),
    Opcode::new(0x88, "DEY", 1, 2, AddressingMode::Implied),
    Opcode::new(0xee, "INC", 3, 6, AddressingMode::Absolute),
    Opcode::new(0xfe, "INC", 3, 7, AddressingMode::AbsoluteX),
    Opcode::new(0xe6, "INC", 2, 5, AddressingMode::ZeroPage),
    Opcode::new(0xf6, "INC", 2, 6, AddressingMode::ZeroPageX),
    Opcode::new(0xe8, "INX", 1, 2, AddressingMode::Implied),
    Opcode::new(0xc8, "INY", 1, 2, AddressingMode::Implied),
    // -- shift instructions --
    // ASL
    Opcode::new(0x0a, "ASL", 1, 2, AddressingMode::Accumulator),
    Opcode::new(0x0e, "ASL", 3, 6, AddressingMode::Absolute),
    Opcode::new(0x1e, "ASL", 3, 7, AddressingMode::AbsoluteX),
    Opcode::new(0x06, "ASL", 2, 5, AddressingMode::ZeroPage),
    Opcode::new(0x16, "ASL", 2, 6, AddressingMode::ZeroPageX),
    // LSR
    Opcode::new(0x4a, "LSR", 1, 2, AddressingMode::Accumulator),
    Opcode::new(0x4e, "LSR", 3, 6, AddressingMode::Absolute),
    Opcode::new(0x5e, "LSR", 3, 7, AddressingMode::AbsoluteX),
    Opcode::new(0x46, "LSR", 2, 5, AddressingMode::ZeroPage),
    Opcode::new(0x56, "LSR", 2, 6, AddressingMode::ZeroPageX),
    // ROL
    Opcode::new(0x2a, "ROL", 1, 2, AddressingMode::Accumulator),
    Opcode::new(0x2e, "ROL", 3, 6, AddressingMode::Absolute),
    Opcode::new(0x3e, "ROL", 3, 7, AddressingMode::AbsoluteX),
    Opcode::new(0x26, "ROL", 2, 5, AddressingMode::ZeroPage),
    Opcode::new(0x36, "ROL", 2, 6, AddressingMode::ZeroPageX),
    // ROR
    Opcode::new(0x6a, "ROR", 1, 2, AddressingMode::Accumulator),
    Opcode::new(0x6e, "ROR", 3, 6, AddressingMode::Absolute),
    Opcode::new(0x7e, "ROR", 3, 7, AddressingMode::AbsoluteX),
    Opcode::new(0x66, "ROR", 2, 5, AddressingMode::ZeroPage),
    Opcode::new(0x76, "ROR", 2, 6, AddressingMode::ZeroPageX),
    // -- branch instructions --
    Opcode::new(0x90, "BCC", 2, 2, AddressingMode::Relative), // +1 if page is crossed, +1 if branch is taken
    Opcode::new(0xb0, "BCS", 2, 2, AddressingMode::Relative), // +1 if page is crossed, +1 if branch is taken
    Opcode::new(0xf0, "BEQ", 2, 2, AddressingMode::Relative), // +1 if page is crossed, +1 if branch is taken
    Opcode::new(0x30, "BMI", 2, 2, AddressingMode::Relative), // +1 if page is crossed, +1 if branch is taken
    Opcode::new(0xd0, "BNE", 2, 2, AddressingMode::Relative), // +1 if page is crossed, +1 if branch is taken
    Opcode::new(0x10, "BPL", 2, 2, AddressingMode::Relative), // +1 if page is crossed, +1 if branch is taken
    Opcode::new(0x50, "BVC", 2, 2, AddressingMode::Relative), // +1 if page is crossed, +1 if branch is taken
    Opcode::new(0x70, "BVS", 2, 2, AddressingMode::Relative), // +1 if page is crossed, +1 if branch is taken
    // -- arithmetic instructions --
    //ADC
    Opcode::new(0x69, "ADC", 2, 2, AddressingMode::Immediate),
    Opcode::new(0x6d, "ADC", 3, 4, AddressingMode::Absolute),
    Opcode::new(0x7d, "ADC", 3, 4, AddressingMode::AbsoluteX), // +1 cycle if page is crossed
    Opcode::new(0x79, "ADC", 3, 4, AddressingMode::AbsoluteY), // +1 cycle if page is crossed
    Opcode::new(0x65, "ADC", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0x75, "ADC", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0x61, "ADC", 2, 6, AddressingMode::IndirectX),
    Opcode::new(0x71, "ADC", 2, 5, AddressingMode::IndirectX), // +1 cycle if page boundary crossed
];

lazy_static! {
    pub static ref OPCODES_MAPPING: HashMap<u8, &'static Opcode> = {
        let mut mapping = HashMap::with_capacity(OPCODES.len());

        for opcode in OPCODES {
            mapping.insert(opcode.code, opcode);
        }

        mapping
    };
}
