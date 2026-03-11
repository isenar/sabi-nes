//! The generic designation NROM refers to the Nintendo cartridge boards NES-NROM-128, NES-NROM-256, their HVC counterparts, and clone boards.
//! The iNES format assigns mapper 0 to NROM.
//! The suffixes 128 and 256 refer to kilobits by Nintendo's own designation;

use crate::cartridge::mappers::{Mapper, MapperId};
use crate::{Address, Byte, Result};

const CHR_RAM_SIZE: usize = 8192;

#[derive(Debug, Default)]
pub struct Nrom<const PRG_ROM_BANKS: usize> {
    chr: Vec<Byte>,
    is_chr_ram: bool,
}

pub type Nrom128 = Nrom<1>;
pub type Nrom256 = Nrom<2>;

impl<const N: usize> Nrom<N> {
    fn load_chr_data(&mut self, data: Vec<Byte>) {
        if data.is_empty() {
            self.chr = vec![Byte::default(); CHR_RAM_SIZE];
            self.is_chr_ram = true;
        } else {
            self.chr = data;
            self.is_chr_ram = false;
        }
    }

    fn read_chr_data(&self, address: Address) -> Byte {
        self.chr
            .get(address.as_usize())
            .copied()
            .unwrap_or_default()
    }

    fn write_char_data(&mut self, address: Address, value: Byte) {
        if self.is_chr_ram
            && let Some(b) = self.chr.get_mut(address.as_usize())
        {
            *b = value;
        }
    }
}

impl<const N: usize> Mapper for Nrom<N> {
    fn map_address(&self, address: Address) -> Result<usize> {
        Ok(address.value() as usize % (N * 0x4000))
    }

    fn write(&mut self, _: Address, _: Byte) {}

    fn load_chr(&mut self, data: Vec<Byte>) {
        self.load_chr_data(data);
    }

    fn read_chr(&self, address: Address) -> Byte {
        self.read_chr_data(address)
    }

    fn write_chr(&mut self, address: Address, value: Byte) {
        self.write_char_data(address, value);
    }
}

impl<const PRG_ROM_BANKS: usize> MapperId for Nrom<PRG_ROM_BANKS> {
    const ID: u32 = 0;
}
