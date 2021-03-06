use crate::cpu::Address;
use crate::{Byte, Result};

pub trait Memory {
    fn read(&mut self, addr: Address) -> Result<Byte>;
    fn write(&mut self, addr: Address, value: Byte) -> Result<()>;

    fn read_u16(&mut self, addr: Address) -> Result<u16> {
        let lo = self.read(addr)?;
        let hi = self.read(addr + 1)?;

        Ok(u16::from_le_bytes([lo, hi]))
    }

    fn write_u16(&mut self, addr: Address, data: u16) -> Result<()> {
        let [lo, hi] = data.to_le_bytes();

        self.write(addr, lo)?;
        self.write(addr + 1, hi)?;

        Ok(())
    }
}
