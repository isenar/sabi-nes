mod mmc1;
mod nrom;

use crate::{Address, Byte};

pub use mmc1::Mmc1;
pub use nrom::{Nrom128, Nrom256};

pub trait MapperId {
    const ID: u8;

    fn name(&self) -> &'static str;
}

pub trait Mapper {
    /// Maps a CPU address to a PRG ROM offset
    fn map_address(&self, address: Address) -> usize;

    /// Write to mapper registers (for mappers with registers like MMC1)
    fn write(&mut self, address: Address, value: Byte);

    /// Load CHR ROM/RAM data into the mapper
    fn load_chr(&mut self, data: Vec<Byte>);

    /// Read a byte from CHR ROM/RAM at the given PPU address ($0000-$1FFF)
    fn read_chr(&self, address: Address) -> Byte;

    /// Write a byte to CHR RAM (no-op for CHR ROM)
    fn write_chr(&mut self, address: Address, value: Byte);
}
