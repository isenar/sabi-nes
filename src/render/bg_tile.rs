use crate::ppu::Ppu;
use crate::{Address, Byte, Result};
use anyhow::anyhow;
use std::ops::RangeInclusive;

#[derive(Debug, Copy, Clone)]
pub struct BgTile {
    pub address: Address,
    pub address_in_attr_table: Address,
}

impl BgTile {
    pub fn new(address: Address, ppu: &Ppu) -> Result<Self> {
        let address_in_attr_table = ppu
            .vram
            .get(address as usize)
            .ok_or_else(|| anyhow!("Failed to fetch address from VRAM ({:#x})", address))?
            .to_owned()
            .into();

        Ok(Self {
            address,
            address_in_attr_table,
        })
    }

    pub fn attribute_table_idx(&self) -> usize {
        self.row() / 4 * 8 + self.column() / 4
    }

    pub fn palette_table_idx(&self, attribute_byte: Byte) -> Byte {
        let indices = (self.column() % 4 / 2, self.row() % 4 / 2);
        let shift_by = indices.0 * 2 + indices.1 * 4;

        let palette_idx = (attribute_byte >> shift_by) & 0b11;

        4 * palette_idx + 1
    }

    pub fn range(&self, bank: Address) -> RangeInclusive<usize> {
        let tile_addr = self.address_in_attr_table as usize;
        let bank = bank as usize;

        (bank + tile_addr * 16)..=(bank + tile_addr * 16 + 15)
    }

    pub fn column(&self) -> usize {
        (self.address % 32).into()
    }

    pub fn row(&self) -> usize {
        (self.address / 32).into()
    }
}
