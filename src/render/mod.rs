mod frame;
pub mod palettes;

use crate::ppu::Ppu;
use crate::render::palettes::SYSTEM_PALLETE;
use crate::{Address, Byte, Result};
use anyhow::anyhow;

pub use frame::Frame;

pub type Rgb = (Byte, Byte, Byte);

pub fn render(ppu: &Ppu, frame: &mut Frame) -> Result<()> {
    render_background(ppu, frame)?;
    render_sprites(ppu, frame)?;

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

fn render_sprites(ppu: &Ppu, frame: &mut Frame) -> Result<()> {
    let oam_data = ppu.registers.read_all_oam_data();
    for i in (0..oam_data.len()).step_by(4).rev() {
        let tile_idx = oam_data[i + 1] as usize;
        let tile_x = oam_data[i + 3] as usize;
        let tile_y = oam_data[i] as usize;

        let flip_vertical = oam_data[i + 2] >> 7 & 1 == 1;
        let flip_horizontal = oam_data[i + 2] >> 6 & 1 == 1;
        let palette_idx = oam_data[i + 2] & 0b11;
        let sprite_palette = sprite_palette(ppu, palette_idx.into());

        let bank = ppu.read_sprite_pattern_address() as usize;

        let tile = &ppu.chr_rom[(bank + tile_idx * 16)..=(bank + tile_idx * 16 + 15)];

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            'ololo: for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => continue 'ololo, // skip coloring the pixel
                    1 => SYSTEM_PALLETE[sprite_palette[1] as usize],
                    2 => SYSTEM_PALLETE[sprite_palette[2] as usize],
                    3 => SYSTEM_PALLETE[sprite_palette[3] as usize],
                    _ => panic!("can't be"),
                };
                match (flip_horizontal, flip_vertical) {
                    (false, false) => frame.set_pixel(tile_x + x, tile_y + y, rgb),
                    (true, false) => frame.set_pixel(tile_x + 7 - x, tile_y + y, rgb),
                    (false, true) => frame.set_pixel(tile_x + x, tile_y + 7 - y, rgb),
                    (true, true) => frame.set_pixel(tile_x + 7 - x, tile_y + 7 - y, rgb),
                }
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

fn sprite_palette(ppu: &Ppu, palette_idx: usize) -> [Byte; 4] {
    let start = palette_idx * 4 + 0x11;
    [
        0,
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}
