mod mmc1;
mod nrom;

use crate::{Address, Byte, Result};

pub use mmc1::Mmc1;
pub use nrom::{Nrom128, Nrom256};

pub trait MapperId {
    const ID: u32;
}

pub trait Mapper {
    /// Maps a CPU/PPU address to a ROM offset (usize for large ROMs)
    fn map_address(&self, address: Address) -> Result<usize>;

    /// Write to mapper registers (for mappers that support writes)
    /// Default implementation does nothing (for read-only mappers like NROM)
    fn write(&mut self, address: Address, value: Byte);
}
