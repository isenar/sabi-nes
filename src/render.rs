mod frame;
mod palettes;

use crate::cartridge::MirroringType;
use crate::cartridge::mappers::Mapper;
use crate::ppu::{Ppu, SpriteData, SpriteSize};
use crate::{Address, Byte, Result};

pub use frame::Frame;
pub use palettes::{Palette, SystemPalette};

const TRANSPARENT_PIXEL: Byte = Byte::new(0b00);

type MetaTile = [Byte; 4];

#[derive(Debug, Clone, Copy)]
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
        let name_table_address = self.ppu.registers.read_name_table_address();
        let (main_table, secondary_table) = match (self.ppu.mirroring, name_table_address.value()) {
            (MirroringType::Vertical, 0x2000 | 0x2800)
            | (MirroringType::Horizontal, 0x2000 | 0x2400) => {
                (&self.ppu.vram[0..0x0400], &self.ppu.vram[0x0400..0x0800])
            }
            (MirroringType::Vertical, 0x2400 | 0x2c00)
            | (MirroringType::Horizontal, 0x2800 | 0x2c00) => {
                (&self.ppu.vram[0x400..0x800], &self.ppu.vram[0..0x400])
            }
            _ => todo!(),
        };

        let scroll_x = self.ppu.registers.read_scroll_x().as_usize();
        let scroll_y = self.ppu.registers.read_scroll_y().as_usize();

        for screen_y in 0..Frame::HEIGHT {
            let y_in_nametable = (screen_y + scroll_y) % 240;

            // Render main portion
            self.render_scanline(
                main_table,
                screen_y,
                y_in_nametable,
                scroll_x,
                0,
                Frame::WIDTH.saturating_sub(scroll_x),
            )?;

            // Render wrapped portion if scrolling
            if scroll_x > 0 {
                self.render_scanline(
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

    fn render_sprites(&mut self) -> Result<()> {
        let oam_data = self.ppu.registers.read_oam_dma();
        let sprite_size = self.ppu.registers.sprite_size();

        for sprite in oam_data {
            let palette_idx = sprite.palette_index();
            let sprite_palette = self.sprite_palette(palette_idx);

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
                    self.render_sprite_tile(sprite, tile_idx_top, bank, 0, &sprite_palette)?;
                    // Render bottom half
                    self.render_sprite_tile(sprite, tile_idx_bottom, bank, 8, &sprite_palette)?;
                }
                SpriteSize::Small => {
                    // 8x8 mode: render single tile
                    let tile_idx = sprite.index_number.as_usize();
                    let bank = self.ppu.read_sprite_pattern_address();
                    self.render_sprite_tile(sprite, tile_idx, bank, 0, &sprite_palette)?;
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
        sprite_palette: &MetaTile,
    ) -> Result<()> {
        let begin = bank_address.as_usize() + tile_idx * 16;
        let tile: [Byte; 16] =
            std::array::from_fn(|i| self.mapper.read_chr(Address::new((begin + i) as u16)));
        let is_sprite_in_background = sprite.priority();

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

                // Prevent out-of-screen bleeding. Without this, sprites
                // on the right side of the screen might be drawn on the left side.
                if x >= Frame::WIDTH || y >= Frame::HEIGHT {
                    continue;
                }

                // Check sprite priority:
                // - If priority is behind, only draw if no background pixel exists
                // - If priority_behind is not behind, always draw (sprite in front)
                if is_sprite_in_background && self.frame.has_background(x, y) {
                    continue; // Skip this pixel, background takes priority
                }

                let colour = self.palette.colour_by_meta_tile(value, sprite_palette);
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
            let x_in_nametable = (screen_x.saturating_sub(screen_x_start) + scroll_x_offset) % 256;
            let tile_column = x_in_nametable / 8;
            let pixel_x_in_tile = 7 - (x_in_nametable % 8);

            let tile_addr = tile_row * 32 + tile_column;
            if tile_addr >= 0x03c0 {
                continue; // Skip attribute table area
            }

            let tile_index = name_table[tile_addr].as_usize();
            let begin = bank_address.as_usize() + tile_index * 16;
            let tile: [Byte; 16] =
                std::array::from_fn(|i| self.mapper.read_chr(Address::new((begin + i) as u16)));
            let bg_palette = bg_palette(self.ppu, attribute_table, tile_column, tile_row);

            // Get pixel from tile
            let upper = tile[pixel_y_in_tile];
            let lower = tile[pixel_y_in_tile + 8];
            let value = (((lower >> pixel_x_in_tile) & 1) << 1) | (upper >> pixel_x_in_tile) & 1;
            let colour = self.palette.colour_by_meta_tile(value, &bg_palette);

            // Mark as background pixel if non-transparent
            if value != TRANSPARENT_PIXEL {
                self.frame.set_bg_pixel(screen_x, screen_y, colour);
            } else {
                self.frame.set_pixel_colour(screen_x, screen_y, colour);
            }
        }

        Ok(())
    }

    const fn sprite_palette(&self, palette_index: usize) -> MetaTile {
        let start = palette_index * 4 + 0x11;
        [
            Byte::new(0x00),
            self.ppu.palette_table[start],
            self.ppu.palette_table[start + 1],
            self.ppu.palette_table[start + 2],
        ]
    }
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
