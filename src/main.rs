use sabi_nes::ppu::Ppu;
use sabi_nes::render::{render, Frame};
use sabi_nes::{Bus, Cpu, Error, Result, Rom};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

fn main() -> Result<()> {
    println!("Starting the emulator");

    let sdl_context = sdl2::init().map_err(Error::msg)?;
    let video_subsystem = sdl_context.video().map_err(Error::msg)?;
    let window = video_subsystem
        .window("Sabi NES", 256 * 3, 240 * 3)
        .position_centered()
        .build()?;
    let mut canvas = window.into_canvas().present_vsync().build()?;
    let mut event_pump = sdl_context.event_pump().map_err(Error::msg)?;
    canvas.set_scale(3.0, 3.0).map_err(Error::msg)?;

    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(PixelFormatEnum::RGB24, 256, 240)?;

    let game_bytes = std::fs::read("examples/pacman.nes")?;
    let rom = Rom::new(&game_bytes)?;

    let mut frame = Frame::default();

    let bus = Bus::new(rom, move |ppu: &Ppu| {
        render(ppu, &mut frame).expect("Failed to render");

        texture.update(None, &frame.data, 256 * 3).unwrap();

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
