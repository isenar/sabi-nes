use crate::{Address, Byte};

use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub struct BgTile {
    pub address: Address,
    pub tile_index: usize,
}

impl BgTile {
    pub fn new(address: Address, tile_index: impl Into<usize>) -> Self {
        Self {
            address,
            tile_index: tile_index.into(),
        }
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

    pub const fn bank_tiles(&self, bank: usize) -> RangeInclusive<usize> {
        let start = bank + self.tile_index * 16;

        start..=(start + 15)
    }

    pub fn column(&self) -> usize {
        (self.address % 32).into()
    }

    pub fn row(&self) -> usize {
        (self.address / 32).into()
    }
}
