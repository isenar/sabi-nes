use sabi_nes_core::cartridge::Rom;
use sabi_nes_core::{Address, Bus, Byte, Cpu, Memory};

use anyhow::Result;
use rand::RngExt;
use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};

fn handle_user_input(cpu: &mut Cpu, event_pump: &mut EventPump) -> Result<()> {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => std::process::exit(0),
            Event::KeyDown {
                keycode: Some(Keycode::W),
                ..
            } => cpu.write_byte(Address::new(0xff), 0x77.into())?,
            Event::KeyDown {
                keycode: Some(Keycode::S),
                ..
            } => cpu.write_byte(Address::new(0xff), 0x73.into())?,
            Event::KeyDown {
                keycode: Some(Keycode::A),
                ..
            } => cpu.write_byte(Address::new(0xff), 0x61.into())?,
            Event::KeyDown {
                keycode: Some(Keycode::D),
                ..
            } => cpu.write_byte(Address::new(0xff), 0x64.into())?,
            _ => {}
        }
    }

    Ok(())
}

fn color(byte: Byte) -> Color {
    match byte.value() {
        0 => Color::BLACK,
        1 => Color::WHITE,
        2 | 9 => Color::GREY,
        3 | 10 => Color::RED,
        4 | 11 => Color::GREEN,
        5 | 12 => Color::BLUE,
        6 | 13 => Color::MAGENTA,
        7 | 14 => Color::YELLOW,
        _ => Color::CYAN,
    }
}

fn screen_update_needed(cpu: &mut Cpu, frame: &mut [u8; 32 * 3 * 32]) -> Result<bool> {
    let mut frame_idx = 0;

    for address in 0x0200..0x0600 {
        let address = Address::new(address);
        let color_idx = cpu.read_byte(address)?;
        let (red, green, blue) = color(color_idx).rgb();
        if frame[frame_idx] != red || frame[frame_idx + 1] != green || frame[frame_idx + 2] != blue
        {
            frame[frame_idx] = red;
            frame[frame_idx + 1] = green;
            frame[frame_idx + 2] = blue;

            return Ok(true);
        }

        frame_idx += 3;
    }

    Ok(false)
}

fn main() -> Result<()> {
    let sdl_context = sdl2::init().map_err(anyhow::Error::msg)?;
    let video_subsystem = sdl_context.video().map_err(anyhow::Error::msg)?;
    let window = video_subsystem
        .window("Snake Game", 320, 320)
        .position_centered()
        .build()?;

    let mut canvas = window.into_canvas().present_vsync().build()?;
    let mut event_pump = sdl_context.event_pump().map_err(anyhow::Error::msg)?;

    canvas.set_scale(10.0, 10.0).map_err(anyhow::Error::msg)?;

    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(PixelFormatEnum::RGB24, 32, 32)?;

    let mut screen_state = [0; 32 * 3 * 32];
    let mut rng = rand::rng();

    let rom = Rom::from_file("examples/snake.nes")?;
    let bus = Bus::new(rom);
    let mut cpu = Cpu::new(bus);
    cpu.reset()?;

    loop {
        cpu.step()?;

        handle_user_input(&mut cpu, &mut event_pump)?;

        cpu.write_byte(Address::new(0xfe), rng.random_range(1..16).into())?;

        if screen_update_needed(&mut cpu, &mut screen_state)? {
            texture.update(None, &screen_state, 32 * 3)?;
            canvas
                .copy(&texture, None, None)
                .map_err(anyhow::Error::msg)?;
            canvas.present();
        }

        std::thread::sleep(std::time::Duration::from_micros(50));
    }
}
