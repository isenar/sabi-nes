mod bg_tile;
mod frame;
pub mod palettes;
mod viewport;

use crate::ppu::Ppu;
use crate::render::bg_tile::BgTile;
use crate::render::palettes::SYSTEM_PALETTE;
use crate::{Address, Byte, Result};

use crate::cartridge::MirroringType;
use crate::render::viewport::Viewport;
pub use frame::Frame;

pub type Rgb = (Byte, Byte, Byte);
type MetaTile = [Byte; 4];

pub fn render(ppu: &Ppu, frame: &mut Frame) -> Result<()> {
    if ppu.registers.show_background() {
        render_background(ppu, frame)?;
    }

    if ppu.registers.show_sprites() {
        render_sprites(ppu, frame)?;
    }

    Ok(())
}

fn render_background(ppu: &Ppu, frame: &mut Frame) -> Result<()> {
    let name_table_address = ppu.registers.control.name_table_address();
    let scroll_x = ppu.registers.scroll.scroll_x as usize;
    let scroll_y = ppu.registers.scroll.scroll_y as usize;

    let (main_table, secondary_table) = match (ppu.mirroring, name_table_address) {
        (MirroringType::Vertical, 0x2000 | 0x2800)
        | (MirroringType::Horizontal, 0x2000 | 0x2400) => {
            (&ppu.vram[0..0x0400], &ppu.vram[0x0400..0x0800])
        }
        (MirroringType::Vertical, 0x2400 | 0x2c00)
        | (MirroringType::Horizontal, 0x2800 | 0x2c00) => {
            (&ppu.vram[0x400..0x800], &ppu.vram[0..0x400])
        }
        _ => todo!(),
    };

    let viewport = Viewport::new(scroll_x, 256, scroll_y, 240);
    render_name_table(
        ppu,
        frame,
        main_table,
        viewport,
        -(scroll_x as isize),
        -(scroll_y as isize),
    )?;

    if scroll_x > 0 {
        let viewport = Viewport::new(0, scroll_x, 0, 240);
        render_name_table(
            ppu,
            frame,
            secondary_table,
            viewport,
            (256 - scroll_x) as isize,
            0,
        )?;
    } else if scroll_y > 0 {
        render_name_table(
            ppu,
            frame,
            secondary_table,
            Viewport::new(0, 256, 0, scroll_y),
            0,
            (240 - scroll_y) as isize,
        )?;
    }

    Ok(())
}

fn render_name_table(
    ppu: &Ppu,
    frame: &mut Frame,
    name_table: &[Byte],
    viewport: Viewport,
    shift_x: isize,
    shift_y: isize,
) -> Result<()> {
    let bank = ppu.registers.background_pattern_address();
    let attribute_table = &name_table[0x03c0..0x0400];

    for (addr, tile_index) in name_table.iter().enumerate().take(0x03c0) {
        let tile_new = BgTile::new(addr as Address, ppu)?;
        let tile_column = addr % 32;
        let tile_row = addr / 32;
        let tile_idx = *tile_index as Address;
        let tile =
            &ppu.chr_rom[(bank + tile_idx * 16) as usize..=(bank + tile_idx * 16 + 15) as usize];
        let bg_palette = bg_palette(ppu, attribute_table, tile_column, tile_row);

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = ((1 & lower) << 1 | (1 & upper)) as usize;
                upper >>= 1;
                lower >>= 1;
                let rgb = SYSTEM_PALETTE[bg_palette[value] as usize];
                let pixel_x = tile_new.column() * 8 + x;
                let pixel_y = tile_new.row() * 8 + y;

                if pixel_x >= viewport.x1
                    && pixel_x < viewport.x2
                    && pixel_y >= viewport.y1
                    && pixel_y < viewport.y2
                {
                    frame.set_pixel(
                        (shift_x + pixel_x as isize) as usize,
                        (shift_y + pixel_y as isize) as usize,
                        rgb,
                    );
                }
            }
        }
    }

    Ok(())
}

fn render_sprites(ppu: &Ppu, frame: &mut Frame) -> Result<()> {
    let oam_data = ppu.registers.read_oam_dma();
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
            'inner_loop: for x in (0..=7).rev() {
                let value = ((1 & lower) << 1 | (1 & upper)) as usize;
                upper >>= 1;
                lower >>= 1;
                let rgb = match value {
                    0 => continue 'inner_loop, // skip coloring the pixel
                    _ => SYSTEM_PALETTE[sprite_palette[value] as usize],
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

fn bg_palette(
    ppu: &Ppu,
    attribute_table: &[Byte],
    tile_column: usize,
    tile_row: usize,
) -> [Byte; 4] {
    let attr_table_idx = tile_row / 4 * 8 + tile_column / 4;
    let attr_byte = attribute_table[attr_table_idx];
    let palette_idx = match (tile_column % 4 / 2, tile_row % 4 / 2) {
        (0, 0) => attr_byte & 0b11,
        (1, 0) => (attr_byte >> 2) & 0b11,
        (0, 1) => (attr_byte >> 4) & 0b11,
        (1, 1) => (attr_byte >> 6) & 0b11,
        (_, _) => panic!("should not happen"),
    };

    let palette_start = 1 + (palette_idx as usize) * 4;
    [
        ppu.palette_table[0],
        ppu.palette_table[palette_start],
        ppu.palette_table[palette_start + 1],
        ppu.palette_table[palette_start + 2],
    ]
}

fn sprite_palette(ppu: &Ppu, palette_idx: usize) -> MetaTile {
    let start = palette_idx * 4 + 0x11;
    [
        0,
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}
