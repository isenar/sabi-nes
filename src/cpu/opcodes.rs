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
    Opcode::new(0xe8, "INX", 1, 2, AddressingMode::Implied),
    Opcode::new(0xc8, "INY", 1, 2, AddressingMode::Implied),
    // -- flag clear/set instructions
    Opcode::new(0x18, "CLC", 1, 2, AddressingMode::Implied),
    Opcode::new(0xd8, "CLD", 1, 2, AddressingMode::Implied),
    Opcode::new(0x58, "CLI", 1, 2, AddressingMode::Implied),
    Opcode::new(0xb8, "CLV", 1, 2, AddressingMode::Implied),
    Opcode::new(0x38, "SEC", 1, 2, AddressingMode::Implied),
    Opcode::new(0xf8, "SED", 1, 2, AddressingMode::Implied),
    Opcode::new(0x78, "SEI", 1, 2, AddressingMode::Implied),
    // AND
    Opcode::new(0x29, "AND", 2, 2, AddressingMode::Immediate),
    Opcode::new(0x25, "AND", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0x35, "AND", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0x2d, "AND", 3, 4, AddressingMode::Absolute),
    Opcode::new(0x3d, "AND", 3, 4, AddressingMode::AbsoluteX), // +1 cycle if page boundary crossed
    Opcode::new(0x39, "AND", 3, 4, AddressingMode::AbsoluteY), // +1 cycle if page boundary crossed
    Opcode::new(0x21, "AND", 2, 6, AddressingMode::IndirectX),
    Opcode::new(0x31, "AND", 2, 5, AddressingMode::IndirectY), // +1 cycle if page boundary crossed
    // -- load/set instructions --
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
