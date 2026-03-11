mod frame;
mod palettes;

use crate::cartridge::MirroringType;
use crate::ppu::{Ppu, SpriteData, SpriteSize};
use crate::{Address, Byte, Result};

pub use frame::Frame;
pub use palettes::SYSTEM_PALETTE;

const TRANSPARENT_PIXEL: Byte = Byte::new(0b00);

type MetaTile = [Byte; 4];

#[derive(Debug, Clone, Copy)]
pub struct Colour(Byte, Byte, Byte);

impl Colour {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self(Byte::new(red), Byte::new(green), Byte::new(blue))
    }
}

pub fn render(ppu: &Ppu, frame: &mut Frame) -> Result<()> {
    frame.clear_background_mask();

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

    let (main_table, secondary_table) = match (ppu.mirroring, name_table_address.value()) {
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

    for screen_y in 0..Frame::HEIGHT {
        // Use scroll values captured for this specific scanline
        let (scroll_x, scroll_y) = ppu.scanline_scroll[screen_y];
        let scroll_x = scroll_x as usize;
        let scroll_y = scroll_y as usize;

        let y_in_nametable = (screen_y + scroll_y) % 240;

        // Render main portion
        render_scanline(
            ppu,
            frame,
            main_table,
            screen_y,
            y_in_nametable,
            scroll_x,
            0,
            Frame::WIDTH.saturating_sub(scroll_x),
        )?;

        // Render wrapped portion if scrolling
        if scroll_x > 0 {
            render_scanline(
                ppu,
                frame,
                secondary_table,
                screen_y,
                y_in_nametable,
                0,
                Frame::WIDTH - scroll_x,
                scroll_x,
            )?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_scanline(
    ppu: &Ppu,
    frame: &mut Frame,
    name_table: &[Byte],
    screen_y: usize,
    nametable_y: usize,
    scroll_x_offset: usize,
    screen_x_start: usize,
    width: usize,
) -> Result<()> {
    if width == 0 {
        return Ok(());
    }

    let bank_address = ppu.registers.background_pattern_address();
    let attribute_table = &name_table[0x03c0..0x0400];

    // Calculate which tile row we're in
    let tile_row = nametable_y / 8;
    let pixel_y_in_tile = nametable_y % 8;

    // Render tiles across this scanline
    for screen_x in screen_x_start..(screen_x_start + width) {
        let x_in_nametable = (screen_x.saturating_sub(screen_x_start) + scroll_x_offset) % 256;
        let tile_column = x_in_nametable / 8;
        let pixel_x_in_tile = 7 - (x_in_nametable % 8);

        let tile_addr = tile_row * 32 + tile_column;
        if tile_addr >= 0x03c0 {
            continue; // Skip attribute table area
        }

        let tile_index = name_table[tile_addr].as_usize();
        let begin = bank_address.as_usize() + tile_index * 16; // TODO
        let end = bank_address.as_usize() + tile_index * 16 + 15; // TODO
        let tile = &ppu.chr_rom[begin..=end];
        let bg_palette = bg_palette(ppu, attribute_table, tile_column, tile_row);

        // Get pixel from tile
        let upper = tile[pixel_y_in_tile];
        let lower = tile[pixel_y_in_tile + 8];
        let value = (((lower >> pixel_x_in_tile) & 1) << 1) | (upper >> pixel_x_in_tile) & 1;
        let colour = SYSTEM_PALETTE[bg_palette[value.as_usize()].as_usize()]; // TODO: helper fn?

        // Mark as background pixel if non-transparent (value != 0)
        if value != TRANSPARENT_PIXEL {
            frame.set_bg_pixel(screen_x, screen_y, colour);
        } else {
            frame.set_pixel_colour(screen_x, screen_y, colour);
        }
    }

    Ok(())
}

fn render_sprites(ppu: &Ppu, frame: &mut Frame) -> Result<()> {
    let oam_data = ppu.registers.read_oam_dma();
    let sprite_size = ppu.registers.sprite_size();

    for sprite in oam_data {
        let palette_idx = sprite.palette_index();
        let sprite_palette = sprite_palette(ppu, palette_idx);

        match sprite_size {
            SpriteSize::Large => {
                // 8x16 mode: render two 8x8 tiles vertically
                // Bit 0 of tile index determines which pattern table (ignored)
                // Top tile: tile_idx & 0xFE
                // Bottom tile: (tile_idx & 0xFE) + 1
                let tile_idx_top = (sprite.index_number & 0xFE).as_usize();
                let tile_idx_bottom = tile_idx_top + 1;

                // In 8x16 mode, bit 0 of tile index selects pattern table
                let bank = if sprite.index_number & 1 == 0 {
                    Address::new(0)
                } else {
                    Address::new(0x1000)
                };

                // Render top half
                render_sprite_tile(ppu, frame, sprite, tile_idx_top, bank, &sprite_palette, 0)?;
                // Render bottom half
                render_sprite_tile(
                    ppu,
                    frame,
                    sprite,
                    tile_idx_bottom,
                    bank,
                    &sprite_palette,
                    8,
                )?;
            }
            SpriteSize::Small => {
                // 8x8 mode: render single tile
                let tile_idx = sprite.index_number.as_usize();
                let bank = ppu.read_sprite_pattern_address();
                render_sprite_tile(ppu, frame, sprite, tile_idx, bank, &sprite_palette, 0)?;
            }
        }
    }

    Ok(())
}

fn render_sprite_tile(
    ppu: &Ppu,
    frame: &mut Frame,
    sprite: &SpriteData,
    tile_idx: usize,
    bank_address: Address,
    sprite_palette: &MetaTile,
    y_base_offset: usize,
) -> Result<()> {
    let bank_address = bank_address.as_usize();
    let tile = &ppu.chr_rom[(bank_address + tile_idx * 16)..=(bank_address + tile_idx * 16 + 15)];
    let priority_behind = sprite.priority();

    for y_offset in 0..=7 {
        let mut upper = tile[y_offset];
        let mut lower = tile[y_offset + 8];
        for x_offset in (0..=7).rev() {
            let value = ((lower & 1) << 1) | (upper & 1);
            upper >>= 1;
            lower >>= 1;

            if value == TRANSPARENT_PIXEL {
                continue;
            }

            let x = sprite.x_pos(x_offset);
            let y = sprite.y_pos(y_offset + y_base_offset);

            // Check sprite priority:
            // - If priority is behind, only draw if no background pixel exists
            // - If priority_behind is not behind, always draw (sprite in front)
            if priority_behind && frame.has_bg(x, y) {
                continue; // Skip this pixel, background takes priority
            }

            let colour = SYSTEM_PALETTE[sprite_palette[value.as_usize()].as_usize()]; // TODO: again - helper fn
            frame.set_pixel_colour(x, y, colour);
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
        (0, 0) => attr_byte,
        (1, 0) => attr_byte >> 2,
        (0, 1) => attr_byte >> 4,
        (1, 1) => attr_byte >> 6,
        (_, _) => unreachable!("should not happen, we've already covered all cases"),
    };
    let palette_idx = palette_idx & 0b11;
    let palette_start = 1 + palette_idx.as_usize() * 4;
    [
        ppu.palette_table[0],
        ppu.palette_table[palette_start],
        ppu.palette_table[palette_start + 1],
        ppu.palette_table[palette_start + 2],
    ]
}

fn sprite_palette(ppu: &Ppu, palette_idx: Byte) -> MetaTile {
    let start = palette_idx.as_usize() * 4 + 0x11;
    [
        Byte::default(),
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}
