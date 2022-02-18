//! The generic designation NROM refers to the Nintendo cartridge boards NES-NROM-128, NES-NROM-256, their HVC counterparts, and clone boards.
//! The iNES format assigns mapper 0 to NROM.
//! The suffixes 128 and 256 refer to kilobits by Nintendo's own designation;

use crate::cartridge::mappers::{Mapper, MapperId};
use crate::{Address, Result};

#[derive(Debug)]
pub struct Nrom<const PRG_ROM_BANKS: usize>;

pub type Nrom128 = Nrom<1>;
pub type Nrom256 = Nrom<2>;

impl Mapper for Nrom128 {
    fn map_address(&self, address: Address) -> Result<Address> {
        Ok(if address >= 0x4000 {
            address % 0x4000
        } else {
            address
        })
    }
}

impl Mapper for Nrom256 {
    fn map_address(&self, address: Address) -> Result<Address> {
        Ok(address)
    }
}

impl<const PRG_ROM_BANKS: usize> MapperId for Nrom<PRG_ROM_BANKS> {
    const ID: u32 = 0;
}
