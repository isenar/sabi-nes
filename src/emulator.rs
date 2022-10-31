use crate::Config;
use anyhow::Error;
use maplit::hashmap;
use once_cell::sync::Lazy;
use sabi_nes::input::joypad::{Joypad, JoypadButton};
use sabi_nes::ppu::Ppu;
use sabi_nes::render::{render, Frame};
use sabi_nes::Result;
use sabi_nes::{Bus, Cpu, Rom};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Texture, WindowCanvas};
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
                joypad.press_button(key);
            }
        }
        Event::KeyUp {
            keycode: Some(keycode),
            ..
        } => {
            if let Some(&key) = JOYPAD_BUTTON_MAP.get(&keycode) {
                joypad.release_button(key);
            }
        }
        _ => {}
    }
}

pub struct Emulator {
    config: Config,
    canvas: WindowCanvas,
    event_pump: EventPump,
    frame: Frame,
}

impl Emulator {
    pub fn create(config: Config) -> Result<Self> {
        let sdl_context = sdl2::init().map_err(Error::msg)?;
        let video_subsystem = sdl_context.video().map_err(Error::msg)?;
        let window = video_subsystem
            .window("Sabi NES", config.window_width(), config.window_height())
            .position_centered()
            .resizable()
            .build()?;
        let canvas = window.into_canvas().present_vsync().build()?;
        let event_pump = sdl_context.event_pump().map_err(Error::msg)?;
        let frame = Frame::default();

        Ok(Self {
            config,
            canvas,
            event_pump,
            frame,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.canvas
            .set_scale(self.config.scale as f32, self.config.scale as f32)
            .map_err(Error::msg)?;

        let creator = self.canvas.texture_creator();
        let mut texture = creator.create_texture_target(
            PixelFormatEnum::RGB24,
            self.config.window_width,
            self.config.window_height,
        )?;

        let game_bytes = std::fs::read(&self.config.rom_path)?;
        let rom = Rom::new(&game_bytes)?;

        let bus =
            Bus::new_with_callback(rom, move |ppu: &Ppu, joypad: &mut Joypad| -> Result<()> {
                self.callback(ppu, &mut texture, joypad)
            });

        let mut cpu = Cpu::new(bus);
        cpu.reset()?;

        cpu.run()
    }

    fn callback(&mut self, ppu: &Ppu, texture: &mut Texture, joypad: &mut Joypad) -> Result<()> {
        render(ppu, &mut self.frame)?;

        texture.update(
            None,
            &self.frame.pixel_data,
            self.config.window_width() as usize,
        )?;
        self.canvas.copy(&texture, None, None).map_err(Error::msg)?;
        self.canvas.present();

        for event in self.event_pump.poll_iter() {
            handle_event(event, joypad);
        }

        Ok(())
    }
}
