use crate::{Address, Byte, Result};
use anyhow::anyhow;

mod frame;
pub mod palettes;

use crate::ppu::Ppu;
use crate::render::palettes::SYSTEM_PALLETE;
pub use frame::Frame;

pub type Rgb = (Byte, Byte, Byte);

pub fn render(ppu: &Ppu, frame: &mut Frame) -> Result<()> {
    render_background(ppu, frame)?;

    Ok(())
}

fn render_background(ppu: &Ppu, frame: &mut Frame) -> Result<()> {
    let bank = ppu.registers.background_pattern_address();

    for addr in 0..0x03c0 {
        let tile_addr = *ppu
            .vram
            .get(addr)
            .ok_or_else(|| anyhow!("Failed to fetch address from VRAM ({:#x})", addr))?
            as Address;
        let tile_column = addr % 32;
        let tile_row = addr / 32;
        let tile =
            &ppu.chr_rom[(bank + tile_addr * 16) as usize..=(bank + tile_addr * 16 + 15) as usize];
        let bg_palette = bg_palette(ppu, tile_column, tile_row);

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = ((1 & lower) << 1 | (1 & upper)) as usize;
                upper >>= 1;
                lower >>= 1;
                let rgb = SYSTEM_PALLETE[bg_palette[value] as usize];
                frame.set_pixel(tile_column * 8 + x, tile_row * 8 + y, rgb)
            }
        }
    }

    Ok(())
}

fn bg_palette(ppu: &Ppu, tile_column: usize, tile_row: usize) -> [Byte; 4] {
    let attr_table_idx = tile_row / 4 * 8 + tile_column / 4;
    let attr_byte = ppu.vram[0x3c0 + attr_table_idx];
    let indices = (tile_column % 4 / 2, tile_row % 4 / 2);
    let palette_idx = match indices {
        (0, 0) => attr_byte & 0b11,
        (1, 0) => (attr_byte >> 2) & 0b11,
        (0, 1) => (attr_byte >> 4) & 0b11,
        (1, 1) => (attr_byte >> 6) & 0b11,
        _ => unreachable!("Indices cannot be larger than 1"),
    } as usize;
    let palette_start = 4 * palette_idx + 1;

    [
        ppu.palette_table[0],
        ppu.palette_table[palette_start],
        ppu.palette_table[palette_start + 1],
        ppu.palette_table[palette_start + 2],
    ]
}
