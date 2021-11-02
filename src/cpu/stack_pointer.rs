use crate::cpu::{Address, Value};
use std::fmt::{Debug, Formatter};

const STACK_BEGIN_ADDR: Address = 0x0100; // stack is located at page $01 (0x100 - 0x01ff)
const STACK_RESET: u8 = 0xfd;

/// Stack Pointer (or S register) is a byte-wide pointer which stores the stack
/// index into which the next stack element will be inserted
pub struct StackPointer(Value);

impl Debug for StackPointer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl StackPointer {
    pub fn new() -> Self {
        Self(STACK_RESET)
    }

    pub fn value(&self) -> Value {
        self.0
    }

    pub fn address(&self) -> Address {
        STACK_BEGIN_ADDR + self.0 as Address
    }

    pub fn set(&mut self, value: Value) {
        self.0 = value;
    }

    pub fn reset(&mut self) {
        self.0 = STACK_RESET;
    }

    pub fn decrement(&mut self) {
        self.0 = self.0.wrapping_sub(1);
    }

    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
