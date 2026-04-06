mod chr_tile;
mod frame;
mod palettes;
mod tile_palette;

use crate::cartridge::MirroringType;
use crate::cartridge::mappers::Mapper;
use crate::ppu::{Ppu, SpriteData, SpriteSize};
use crate::{Address, Byte, Result};

pub use frame::Frame;
pub use palettes::{Palette, SystemPalette};

use crate::utils::NthBit;
use chr_tile::ChrTile;
use tile_palette::TilePalette;

const TRANSPARENT_PIXEL: Byte = Byte::new(0b00);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colour(Byte, Byte, Byte);

impl Colour {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self(Byte::new(red), Byte::new(green), Byte::new(blue))
    }
}

pub struct Renderer<'a, P> {
    ppu: &'a Ppu,
    mapper: &'a dyn Mapper,
    frame: &'a mut Frame,
    palette: &'a P,
}

impl<'a, P> Renderer<'a, P>
where
    P: Palette,
{
    pub fn new(ppu: &'a Ppu, mapper: &'a dyn Mapper, frame: &'a mut Frame, palette: &'a P) -> Self {
        Self {
            ppu,
            mapper,
            frame,
            palette,
        }
    }

    pub fn render_frame(&mut self) -> Result<()> {
        self.frame.clear_background_mask();

        if self.ppu.registers.show_background() {
            self.render_background()?;
        }

        if self.ppu.registers.show_sprites() {
            self.render_sprites()?;
        }

        Ok(())
    }

    fn render_background(&mut self) -> Result<()> {
        for screen_y in 0..Frame::HEIGHT {
            let (scroll_x_byte, scroll_y_byte, name_table_address) =
                self.ppu.scanline_scroll()[screen_y];
            let scroll_x = scroll_x_byte.as_usize();
            let scroll_y = scroll_y_byte.as_usize();

            // Determine the four nametable quadrants (top-left, top-right, bottom-left, bottom-right).
            // Vertical mirroring: $2000=$2800, $2400=$2C00 → left/right differ, top/bottom same.
            // Horizontal mirroring: $2000=$2400, $2800=$2C00 → top/bottom differ, left/right same.
            let (top_left, top_right, bot_left, bot_right) =
                match (self.ppu.mirroring, name_table_address.value()) {
                    (MirroringType::Vertical, 0x2000 | 0x2800) => {
                        let a = &self.ppu.vram[0..0x0400];
                        let b = &self.ppu.vram[0x0400..0x0800];
                        (a, b, a, b)
                    }
                    (MirroringType::Vertical, 0x2400 | 0x2c00) => {
                        let a = &self.ppu.vram[0x0400..0x0800];
                        let b = &self.ppu.vram[0..0x0400];
                        (a, b, a, b)
                    }
                    (MirroringType::Horizontal, 0x2000 | 0x2400) => {
                        let a = &self.ppu.vram[0..0x0400];
                        let b = &self.ppu.vram[0x0400..0x0800];
                        (a, b, b, b)
                    }
                    (MirroringType::Horizontal, 0x2800 | 0x2c00) => {
                        let a = &self.ppu.vram[0x0400..0x0800];
                        let b = &self.ppu.vram[0..0x0400];
                        (a, b, b, b)
                    }
                    _ => todo!("Four screen mirroring (used in e.g. Gauntlet"),
                };

            let total_y = screen_y + scroll_y;
            // When total_y >= 240 the visible row is in the nametable below the base.
            let (y_in_nametable, in_lower) = if total_y >= 240 {
                (total_y - 240, true)
            } else {
                (total_y, false)
            };

            let (left_table, right_table) = if in_lower {
                (bot_left, bot_right)
            } else {
                (top_left, top_right)
            };

            // Render main portion (scroll_x pixels into the left nametable, to right edge)
            self.render_scanline(
                left_table,
                screen_y,
                y_in_nametable,
                scroll_x,
                0,
                Frame::WIDTH - scroll_x,
            )?;

            // Render the horizontally-wrapped portion from the right nametable
            if scroll_x > 0 {
                self.render_scanline(
                    right_table,
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

    fn render_sprites(&mut self) -> Result<()> {
        let oam_data = self.ppu.registers.read_oam_dma();
        let sprite_size = self.ppu.registers.sprite_size();

        for sprite in oam_data {
            let palette_idx = sprite.palette_index();
            let sprite_palette = self.sprite_palette(palette_idx);

            match sprite_size {
                SpriteSize::Large => {
                    // 8x16 mode: render two 8x8 tiles vertically.
                    // Bit 0 of tile index selects pattern table; top tile = idx & 0xFE, bottom = top + 1.
                    let tile_idx_top = (sprite.index_number & 0xFE).as_usize();
                    let tile_idx_bottom = tile_idx_top + 1;

                    let bank = if sprite.index_number.nth_bit::<0>() {
                        Address::new(0x1000)
                    } else {
                        Address::new(0)
                    };

                    // Pass sprite_height=16 so vertical flip mirrors over the full 16-pixel range.
                    // The formula `15 - (y_offset + y_base_offset)` naturally swaps tiles when flipped.
                    self.render_sprite_tile(sprite, tile_idx_top, bank, 0, 16, &sprite_palette)?;
                    self.render_sprite_tile(sprite, tile_idx_bottom, bank, 8, 16, &sprite_palette)?;
                }
                SpriteSize::Small => {
                    let tile_idx = sprite.index_number.as_usize();
                    let bank = self.ppu.read_sprite_pattern_address();
                    self.render_sprite_tile(sprite, tile_idx, bank, 0, 8, &sprite_palette)?;
                }
            }
        }

        Ok(())
    }

    fn render_sprite_tile(
        &mut self,
        sprite: &SpriteData,
        tile_idx: usize,
        bank_address: Address,
        y_base_offset: usize,
        sprite_height: usize,
        sprite_palette: &TilePalette,
    ) -> Result<()> {
        let begin = bank_address.as_usize() + tile_idx * 16;
        let tile = ChrTile(std::array::from_fn(|i| {
            self.mapper.read_chr(Address::new((begin + i) as u16))
        }));
        let is_sprite_in_background = sprite.priority();

        for y_offset in 0..=7 {
            for x_offset in 0..=7 {
                let value = tile.pixel(x_offset, y_offset);

                if value == TRANSPARENT_PIXEL {
                    continue;
                }

                let x = sprite.x_pos(x_offset);
                let y = sprite.y_pos(y_offset + y_base_offset, sprite_height);

                // Prevent out-of-screen bleeding. Without this, sprites
                // on the right side of the screen might be drawn on the left side.
                if x >= Frame::WIDTH || y >= Frame::HEIGHT {
                    continue;
                }

                // Check sprite priority:
                // - If priority is behind, only draw if no background pixel exists
                // - If priority is not behind, always draw (sprite in front)
                if is_sprite_in_background && self.frame.has_background(x, y) {
                    continue;
                }

                let colour = sprite_palette.colour(value, self.palette);
                self.frame.set_pixel_colour(x, y, colour);
            }
        }

        Ok(())
    }

    fn render_scanline(
        &mut self,
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

        let bank_address = self.ppu.registers.background_pattern_address();
        let attribute_table = &name_table[0x03c0..0x0400];

        // Calculate which tile row we're in
        let tile_row = nametable_y / 8;
        let pixel_y_in_tile = nametable_y % 8;

        // Render tiles across this scanline
        for screen_x in screen_x_start..(screen_x_start + width) {
            // screen_x >= screen_x_start is guaranteed by the loop range, so plain subtraction is safe
            let x_in_nametable = (screen_x - screen_x_start + scroll_x_offset) % 256;
            let tile_column = x_in_nametable / 8;

            let tile_addr = tile_row * 32 + tile_column;
            if tile_addr >= 0x03c0 {
                continue; // Skip attribute table area
            }

            let tile_index = name_table[tile_addr].as_usize();
            let begin = bank_address.as_usize() + tile_index * 16;
            let tile = ChrTile(std::array::from_fn(|i| {
                self.mapper.read_chr(Address::new((begin + i) as u16))
            }));
            let bg_palette = bg_palette(self.ppu, attribute_table, tile_column, tile_row);

            let value = tile.pixel(x_in_nametable % 8, pixel_y_in_tile);
            let colour = bg_palette.colour(value, self.palette);

            // Mark as background pixel if non-transparent
            if value != TRANSPARENT_PIXEL {
                self.frame.set_bg_pixel(screen_x, screen_y, colour);
            } else {
                self.frame.set_pixel_colour(screen_x, screen_y, colour);
            }
        }

        Ok(())
    }

    fn sprite_palette(&self, palette_index: usize) -> TilePalette {
        debug_assert!(
            palette_index < 4,
            "palette_index must be 0-3, got {palette_index}"
        );
        let start = palette_index * 4 + 0x11;
        TilePalette([
            Byte::new(0x00),
            self.ppu.palette_table[start],
            self.ppu.palette_table[start + 1],
            self.ppu.palette_table[start + 2],
        ])
    }
}

fn bg_palette(
    ppu: &Ppu,
    attribute_table: &[Byte],
    tile_column: usize,
    tile_row: usize,
) -> TilePalette {
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
    TilePalette([
        ppu.palette_table[0],
        ppu.palette_table[palette_start],
        ppu.palette_table[palette_start + 1],
        ppu.palette_table[palette_start + 2],
    ])
}
