use sabi_nes::ppu::Ppu;
use sabi_nes::render::{render, Frame};
use sabi_nes::{Bus, Cpu, Error, Result, Rom};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 240;
const SCALE: u32 = 3;

fn main() -> Result<()> {
    let sdl_context = sdl2::init().map_err(Error::msg)?;
    let video_subsystem = sdl_context.video().map_err(Error::msg)?;
    let window = video_subsystem
        .window("Sabi NES", WIDTH * SCALE, HEIGHT * SCALE)
        .position_centered()
        .build()?;
    let mut canvas = window.into_canvas().present_vsync().build()?;
    let mut event_pump = sdl_context.event_pump().map_err(Error::msg)?;
    canvas
        .set_scale(SCALE as f32, SCALE as f32)
        .map_err(Error::msg)?;

    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(PixelFormatEnum::RGB24, WIDTH, HEIGHT)?;

    let game_bytes = std::fs::read("examples/pacman.nes")?;
    let rom = Rom::new(&game_bytes)?;

    let mut frame = Frame::default();

    let bus = Bus::new(rom, move |ppu: &Ppu| {
        render(ppu, &mut frame).expect("Failed to render");

        texture
            .update(None, &frame.pixel_data, (WIDTH * SCALE) as usize)
            .unwrap();

        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),
                _ => {}
            }
        }
    });

    let mut cpu = Cpu::new(bus);
    cpu.reset()?;
    cpu.run()?;

    Ok(())
}
