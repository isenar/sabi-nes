use sabi_nes::{Bus, Cpu, Memory, Rom};

use anyhow::Result;
use rand::Rng;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::EventPump;

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
            } => cpu.write(0xff, 0x77)?,
            Event::KeyDown {
                keycode: Some(Keycode::S),
                ..
            } => cpu.write(0xff, 0x73)?,
            Event::KeyDown {
                keycode: Some(Keycode::A),
                ..
            } => cpu.write(0xff, 0x61)?,
            Event::KeyDown {
                keycode: Some(Keycode::D),
                ..
            } => cpu.write(0xff, 0x64)?,
            _ => {}
        }
    }

    Ok(())
}

fn color(byte: u8) -> Color {
    match byte {
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

fn screen_update_needed(cpu: &Cpu, frame: &mut [u8; 32 * 3 * 32]) -> Result<bool> {
    let mut frame_idx = 0;

    for addr in 0x0200..0x0600 {
        let color_idx = cpu.read(addr)?;
        let (b1, b2, b3) = color(color_idx).rgb();
        if frame[frame_idx] != b1 || frame[frame_idx + 1] != b2 || frame[frame_idx + 2] != b3 {
            frame[frame_idx] = b1;
            frame[frame_idx + 1] = b2;
            frame[frame_idx + 2] = b3;

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
    let mut rng = rand::thread_rng();

    let rom_file = std::fs::read("examples/snake.nes")?;
    let rom = Rom::new(&rom_file)?;
    let bus = Bus::new(rom);
    let mut cpu = Cpu::new(bus);
    cpu.reset()?;
    cpu.run_with_callback(|cpu| {
        // println!("{:?}", cpu);

        handle_user_input(cpu, &mut event_pump)?;
        cpu.write(0xfe, rng.gen_range(1..16))?;

        if screen_update_needed(cpu, &mut screen_state)? {
            texture.update(None, &screen_state, 32 * 3)?;
            canvas
                .copy(&texture, None, None)
                .map_err(anyhow::Error::msg)?;
            canvas.present();
        }

        std::thread::sleep(std::time::Duration::from_micros(50));

        Ok(())
    })?;

    Ok(())
}
