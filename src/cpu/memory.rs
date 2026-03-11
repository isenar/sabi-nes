use crate::{Address, Byte, Result};

pub trait Memory {
    fn read_byte(&mut self, addr: Address) -> Result<Byte>;
    fn write_byte(&mut self, addr: Address, value: Byte) -> Result<()>;

    fn read_word(&mut self, addr: Address) -> Result<u16> {
        let lo = self.read_byte(addr)?;
        let hi = self.read_byte(addr + 1u16)?;

        Ok(u16::from_le_bytes([lo.value(), hi.value()]))
    }

    fn write_word(&mut self, addr: Address, data: u16) -> Result<()> {
        let [lo, hi] = data.to_le_bytes();

        self.write_byte(addr, lo.into())?;
        self.write_byte(addr + 1u16, hi.into())?;

        Ok(())
    }
}
