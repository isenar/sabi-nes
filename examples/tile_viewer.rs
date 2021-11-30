use anyhow::bail;
use sabi_nes::render::palettes::SYSTEM_PALETTE;
use sabi_nes::render::Frame;
use sabi_nes::{Byte, Result, Rom};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

fn show_tiles(chr_rom: &[Byte], bank: usize) -> Result<Frame> {
    assert!(bank <= 1);

    let mut frame = Frame::default();
    let mut tile_y = 0;
    let mut tile_x = 0;
    let bank = (bank * 0x1000) as usize;

    for tile_n in 0..255 {
        if tile_n != 0 && tile_n % 20 == 0 {
            tile_y += 10;
            tile_x = 0;
        }
        let tile = &chr_rom[(bank + tile_n * 16)..=(bank + tile_n * 16 + 15)];

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & upper) << 1 | (1 & lower);
                upper >>= 1;
                lower >>= 1;
                let rgb = match value {
                    0 => SYSTEM_PALETTE[0x02],
                    1 => SYSTEM_PALETTE[0x23],
                    2 => SYSTEM_PALETTE[0x27],
                    3 => SYSTEM_PALETTE[0x30],
                    _ => bail!("RGB color must fit within 2 bits! Got value: {}", value),
                };
                frame.set_pixel(tile_x + x, tile_y + y, rgb)
            }
        }

        tile_x += 10;
    }
    Ok(frame)
}

fn main() -> Result<()> {
    // init sdl2
    let sdl_context = sdl2::init().map_err(anyhow::Error::msg)?;
    let video_subsystem = sdl_context.video().map_err(anyhow::Error::msg)?;
    let window = video_subsystem
        .window("Tile viewer", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build()?;

    let mut canvas = window.into_canvas().present_vsync().build()?;
    let mut event_pump = sdl_context.event_pump().map_err(anyhow::Error::msg)?;
    canvas.set_scale(3.0, 3.0).map_err(anyhow::Error::msg)?;

    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(PixelFormatEnum::RGB24, 256, 240)?;

    let bytes = std::fs::read("examples/pacman.nes")?;
    let rom = Rom::new(&bytes)?;

    let right_bank = show_tiles(&rom.chr_rom, 0)?;

    texture.update(None, &right_bank.pixel_data, 256 * 3)?;
    canvas
        .copy(&texture, None, None)
        .map_err(anyhow::Error::msg)?;
    canvas.present();

    loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return Ok(()),
                _ => {}
            }
        }
    }
}
