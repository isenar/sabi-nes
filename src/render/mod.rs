use crate::{Address, Byte};

mod frame;
pub mod palettes;

use crate::ppu::Ppu;
use crate::render::palettes::SYSTEM_PALLETE;
pub use frame::Frame;

pub type Rgb = (Byte, Byte, Byte);

pub fn render(ppu: &Ppu, frame: &mut Frame) {
    let bank = ppu.registers.background_pattern_address();

    for addr in 0..0x03c0 {
        let tile_addr = ppu.vram.get(addr).unwrap().clone() as Address;
        let tile_x = addr % 32;
        let tile_y = addr / 32;
        let tile =
            &ppu.chr_rom[(bank + tile_addr * 16) as usize..=(bank + tile_addr * 16 + 15) as usize];

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & upper) << 1 | (1 & lower);
                upper >>= 1;
                lower >>= 1;
                let rgb = match value {
                    0 => SYSTEM_PALLETE[0x01],
                    1 => SYSTEM_PALLETE[0x23],
                    2 => SYSTEM_PALLETE[0x27],
                    3 => SYSTEM_PALLETE[0x30],
                    _ => panic!("can't be"),
                };
                frame.set_pixel(tile_x * 8 + x, tile_y * 8 + y, rgb)
            }
        }
    }
}
