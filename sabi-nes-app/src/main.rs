use lazy_static::lazy_static;
use sabi_nes::ppu::Ppu;
use sabi_nes::render::{render, Frame};
use sabi_nes::{Bus, Cpu, Error, Result, Rom};

use sabi_nes::input::joypad::{Joypad, JoypadButton};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

use sdl2::render::WindowCanvas;
use sdl2::EventPump;
use std::collections::HashMap;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 240;
const SCALE: u32 = 3;

lazy_static! {
    static ref JOYPAD_BUTTON_MAP: HashMap<Keycode, JoypadButton> = {
        let mut button_map = HashMap::with_capacity(8);

        button_map.insert(Keycode::S, JoypadButton::DOWN);
        button_map.insert(Keycode::W, JoypadButton::UP);
        button_map.insert(Keycode::D, JoypadButton::RIGHT);
        button_map.insert(Keycode::A, JoypadButton::LEFT);
        button_map.insert(Keycode::Space, JoypadButton::SELECT);
        button_map.insert(Keycode::Return, JoypadButton::START);
        button_map.insert(Keycode::O, JoypadButton::BUTTON_A);
        button_map.insert(Keycode::P, JoypadButton::BUTTON_B);

        button_map
    };
}

fn canvas_and_event_pump() -> Result<(WindowCanvas, EventPump)> {
    let sdl_context = sdl2::init().map_err(Error::msg)?;
    let video_subsystem = sdl_context.video().map_err(Error::msg)?;
    let window = video_subsystem
        .window("Sabi NES", WIDTH * SCALE, HEIGHT * SCALE)
        .position_centered()
        .build()?;
    let mut canvas = window.into_canvas().present_vsync().build()?;
    let event_pump = sdl_context.event_pump().map_err(Error::msg)?;
    canvas
        .set_scale(SCALE as f32, SCALE as f32)
        .map_err(Error::msg)?;

    Ok((canvas, event_pump))
}

fn handle_event(event: Event, joypad: &mut Joypad) {
    match event {
        Event::Quit { .. }
        | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        } => std::process::exit(0),
        Event::KeyDown {
            keycode: Some(keycode),
            ..
        } => {
            if let Some(&key) = JOYPAD_BUTTON_MAP.get(&keycode) {
                joypad.set_button_pressed_status(key, true);
            }
        }
        Event::KeyUp {
            keycode: Some(keycode),
            ..
        } => {
            if let Some(&key) = JOYPAD_BUTTON_MAP.get(&keycode) {
                joypad.set_button_pressed_status(key, false);
            }
        }
        _ => {}
    }
}

fn main() -> Result<()> {
    let (mut canvas, mut event_pump) = canvas_and_event_pump()?;

    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(PixelFormatEnum::RGB24, WIDTH, HEIGHT)?;

    let game_bytes = std::fs::read("roms/pacman.nes")?;
    let rom = Rom::new(&game_bytes)?;
    let mut frame = Frame::default();

    let bus = Bus::new_with_callback(rom, move |ppu: &Ppu, joypad: &mut Joypad| -> Result<()> {
        render(ppu, &mut frame)?;

        texture.update(None, &frame.pixel_data, (WIDTH * SCALE) as usize)?;
        canvas.copy(&texture, None, None).map_err(Error::msg)?;
        canvas.present();

        for event in event_pump.poll_iter() {
            handle_event(event, joypad);
        }

        Ok(())
    });

    let mut cpu = Cpu::new(bus);
    cpu.reset()?;
    cpu.run()?;

    Ok(())
}
