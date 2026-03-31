use anyhow::bail;
use sabi_nes::cartridge::mappers::Mapper;
use sabi_nes::render::{Frame, SYSTEM_PALETTE};
use sabi_nes::{Address, Result, Rom};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::ops::Deref;

fn show_tiles(mapper: &dyn Mapper, bank: usize) -> Result<Frame> {
    assert!(bank <= 1);

    let mut frame = Frame::new();
    let mut tile_y = 0;
    let mut tile_x = 0;
    let bank = bank * 0x1000;

    for tile_n in 0..255 {
        if tile_n != 0 && tile_n % 20 == 0 {
            tile_y += 10;
            tile_x = 0;
        }
        let start = bank + tile_n * 16;
        let tile: [_; 16] =
            std::array::from_fn(|i| mapper.read_chr(Address::new((start + i) as u16)));

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = ((upper & 1) << 1) | (lower & 1);
                upper >>= 1;
                lower >>= 1;
                let colour = match value.value() {
                    0 => SYSTEM_PALETTE[0x02],
                    1 => SYSTEM_PALETTE[0x23],
                    2 => SYSTEM_PALETTE[0x27],
                    3 => SYSTEM_PALETTE[0x30],
                    _ => bail!("RGB color must fit within 2 bits! Got value: {value}"),
                };
                frame.set_pixel_colour(tile_x + x, tile_y + y, colour);
            }
        }

        tile_x += 10;
    }
    Ok(frame)
}

fn main() -> Result<()> {
    let scale = 3;
    // init sdl2
    let sdl_context = sdl2::init().map_err(anyhow::Error::msg)?;
    let video_subsystem = sdl_context.video().map_err(anyhow::Error::msg)?;
    let window = video_subsystem
        .window(
            "Tile viewer",
            (Frame::WIDTH * scale) as _,
            (Frame::HEIGHT * scale) as _,
        )
        .position_centered()
        .build()?;

    let mut canvas = window.into_canvas().present_vsync().build()?;
    let mut event_pump = sdl_context.event_pump().map_err(anyhow::Error::msg)?;
    canvas
        .set_scale(scale as _, scale as _)
        .map_err(anyhow::Error::msg)?;

    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(
        PixelFormatEnum::RGB24,
        Frame::WIDTH as _,
        Frame::HEIGHT as _,
    )?;

    let rom = Rom::from_file("pacman.nes")?;

    let right_bank = show_tiles(rom.mapper.deref(), 0)?;

    texture.update(None, right_bank.pixel_data(), Frame::WIDTH * scale)?;
    canvas
        .copy(&texture, None, None)
        .map_err(anyhow::Error::msg)?;
    canvas.present();

    loop {
        for event in event_pump.poll_iter() {
            if let Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } = event
            {
                return Ok(());
            }
        }
    }
}
