use crate::cpu::{Address, Value};

const STACK_BEGIN_ADDR: Address = 0x0100; // stack is located at page $01 (0x100 - 0x01ff)
const STACK_RESET: u8 = 0xfd;

#[derive(Default, Debug)]
pub struct StackPointer(Value);

impl StackPointer {
    pub fn new(value: Value) -> Self {
        Self(value)
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
}
