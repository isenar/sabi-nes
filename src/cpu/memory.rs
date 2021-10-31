use crate::cpu::{Address, Value};

pub trait Memory {
    fn read(&self, addr: Address) -> Value;
    fn write(&mut self, addr: Address, value: Value);

    fn read_u16(&self, addr: Address) -> u16 {
        let lo = self.read(addr);
        let hi = self.read(addr + 1);

        u16::from_le_bytes([lo, hi])
    }

    fn write_u16(&mut self, addr: Address, data: u16) {
        let [lo, hi] = data.to_le_bytes();

        self.write(addr, lo);
        self.write(addr + 1, hi);
    }
}
