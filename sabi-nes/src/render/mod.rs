mod bg_tile;
mod frame;
pub mod palettes;
mod viewport;

use crate::cartridge::MirroringType;
use crate::ppu::{Ppu, SpriteData};
use crate::render::bg_tile::BgTile;
use crate::render::palettes::SYSTEM_PALETTE;
use crate::render::viewport::Viewport;
use crate::{Address, Byte, Result};

pub use frame::Frame;

const TRANSPARENT_PIXEL: usize = 0b00;

type MetaTile = [Byte; 4];

#[derive(Debug, Clone, Copy)]
pub struct Rgb(Byte, Byte, Byte);

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
    let name_table_address = ppu.registers.read_name_table_address();
    let scroll_x = ppu.registers.read_scroll_x() as usize;
    let scroll_y = ppu.registers.read_scroll_y() as usize;

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

    let viewport = Viewport::new(scroll_x, Frame::WIDTH, scroll_y, Frame::HEIGHT);
    render_name_table(
        ppu,
        frame,
        main_table,
        viewport,
        -(scroll_x as isize),
        -(scroll_y as isize),
    )?;

    if scroll_x > 0 {
        let viewport = Viewport::new(0, scroll_x, 0, Frame::HEIGHT);
        render_name_table(
            ppu,
            frame,
            secondary_table,
            viewport,
            (Frame::WIDTH - scroll_x) as isize,
            0,
        )?;
    } else if scroll_y > 0 {
        render_name_table(
            ppu,
            frame,
            secondary_table,
            Viewport::new(0, Frame::WIDTH, 0, scroll_y),
            0,
            (Frame::HEIGHT - scroll_y) as isize,
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
    let bank = ppu.registers.background_pattern_address() as usize;
    let attribute_table = &name_table[0x03c0..0x0400];

    for (addr, tile_index) in name_table.iter().enumerate().take(0x03c0) {
        let bg_tile = BgTile::new(addr as Address, *tile_index);
        let tile = &ppu.chr_rom[bg_tile.bank_tiles(bank)];
        let bg_palette = bg_palette(ppu, attribute_table, &bg_tile);

        for pixel_y in (0..=7).rev() {
            let mut upper = tile[pixel_y];
            let mut lower = tile[pixel_y + 8];

            for pixel_x in (0..=7).rev() {
                let value = ((1 & lower) << 1 | (1 & upper)) as usize;
                upper >>= 1;
                lower >>= 1;
                let rgb = SYSTEM_PALETTE[bg_palette[value] as usize];
                let pixel_x = bg_tile.column() * 8 + pixel_x;
                let pixel_y = bg_tile.row() * 8 + pixel_y;

                if viewport.contains_pixel(pixel_x, pixel_y) {
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
    for sprite in oam_data {
        render_sprite(sprite, ppu, frame)?;
    }

    Ok(())
}

fn bg_palette(ppu: &Ppu, attribute_table: &[Byte], bg_tile: &BgTile) -> MetaTile {
    let attr_table_idx = bg_tile.attribute_table_idx();
    let attr_byte = attribute_table[attr_table_idx];
    let palette_start = bg_tile.palette_table_idx(attr_byte) as usize;
    [
        ppu.palette_table[0],
        ppu.palette_table[palette_start],
        ppu.palette_table[palette_start + 1],
        ppu.palette_table[palette_start + 2],
    ]
}

fn sprite_palette(ppu: &Ppu, palette_idx: Byte) -> MetaTile {
    let start = palette_idx as usize * 4 + 0x11;
    [
        0,
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}

fn render_sprite(sprite: &SpriteData, ppu: &Ppu, frame: &mut Frame) -> Result<()> {
    let tile_idx = sprite.index as usize;
    let palette_idx = sprite.palette_index();
    let sprite_palette = sprite_palette(ppu, palette_idx);

    let bank = ppu.read_sprite_pattern_address() as usize;

    let tile = &ppu.chr_rom[(bank + tile_idx * 16)..=(bank + tile_idx * 16 + 15)];

    for y_offset in 0..=7 {
        let mut upper = tile[y_offset];
        let mut lower = tile[y_offset + 8];

        for x_offset in (0..=7).rev() {
            let pixel = ((1 & lower) << 1 | (1 & upper)) as usize;
            upper >>= 1;
            lower >>= 1;

            if pixel == TRANSPARENT_PIXEL {
                continue;
            }
            let rgb = SYSTEM_PALETTE[sprite_palette[pixel] as usize];

            frame.set_pixel(sprite.x_pos(x_offset), sprite.y_pos(y_offset), rgb)
        }
    }

    Ok(())
}
