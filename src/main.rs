mod config;

use crate::config::Config;
use clap::Parser;
use maplit::hashmap;
use once_cell::sync::Lazy;
use sabi_nes::input::joypad::{Joypad, JoypadButton};
use sabi_nes::ppu::Ppu;
use sabi_nes::render::{render, Frame};
use sabi_nes::{Bus, Cpu, Error, Result, Rom};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::WindowCanvas;
use sdl2::EventPump;
use std::collections::HashMap;

static JOYPAD_BUTTON_MAP: Lazy<HashMap<Keycode, JoypadButton>> = Lazy::new(|| {
    hashmap! {
        Keycode::S => JoypadButton::DOWN,
        Keycode::W =>  JoypadButton::UP,
        Keycode::D =>  JoypadButton::RIGHT,
        Keycode::A => JoypadButton::LEFT,
        Keycode::Space =>  JoypadButton::SELECT,
        Keycode::Return => JoypadButton::START,
        Keycode::O => JoypadButton::BUTTON_A,
        Keycode::P => JoypadButton::BUTTON_B,
    }
});

fn canvas_and_event_pump(config: &Config) -> Result<(WindowCanvas, EventPump)> {
    let sdl_context = sdl2::init().map_err(Error::msg)?;
    let video_subsystem = sdl_context.video().map_err(Error::msg)?;
    let window = video_subsystem
        .window("Sabi NES", config.window_width(), config.window_height())
        .position_centered()
        .build()?;
    let mut canvas = window.into_canvas().present_vsync().build()?;
    let event_pump = sdl_context.event_pump().map_err(Error::msg)?;
    canvas
        .set_scale(config.scale as f32, config.scale as f32)
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
    let emu_config = Config::parse();
    let (mut canvas, mut event_pump) = canvas_and_event_pump(&emu_config)?;

    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(
        PixelFormatEnum::RGB24,
        emu_config.window_width,
        emu_config.window_height,
    )?;

    let game_bytes = std::fs::read(&emu_config.rom_path)?;
    let rom = Rom::new(&game_bytes)?;
    let mut frame = Frame::default();

    let bus = Bus::new_with_callback(rom, move |ppu: &Ppu, joypad: &mut Joypad| -> Result<()> {
        render(ppu, &mut frame)?;

        texture.update(None, &frame.pixel_data, emu_config.window_width() as usize)?;
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
