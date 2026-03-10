use crate::Config;
use crate::frontend::Frontend;
use anyhow::Error;
use maplit::hashmap;
use once_cell::sync::Lazy;
use sabi_nes::Result;
use sabi_nes::input::joypad::{Joypad, JoypadButton};
use sabi_nes::render::Frame;
use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Texture, WindowCanvas};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const TARGET_FPS: u64 = 60;
const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / TARGET_FPS);

static JOYPAD_BUTTON_MAP: Lazy<HashMap<Keycode, JoypadButton>> = Lazy::new(|| {
    hashmap! {
        Keycode::S => JoypadButton::DOWN,
        Keycode::W => JoypadButton::UP,
        Keycode::D => JoypadButton::RIGHT,
        Keycode::A => JoypadButton::LEFT,
        Keycode::Space => JoypadButton::SELECT,
        Keycode::Return => JoypadButton::START,
        Keycode::O => JoypadButton::BUTTON_A,
        Keycode::P => JoypadButton::BUTTON_B,
    }
});

pub struct SdlFrontend {
    canvas: WindowCanvas,
    event_pump: EventPump,
    last_frame_time: Instant,
}

impl SdlFrontend {
    pub fn new(config: &Config) -> Result<Self> {
        let sdl_context = sdl2::init().map_err(Error::msg)?;
        let video_subsystem = sdl_context.video().map_err(Error::msg)?;

        let window = video_subsystem
            .window("Sabi NES", config.window_width(), config.window_height())
            .position_centered()
            .resizable()
            .build()?;

        let canvas = window.into_canvas().present_vsync().build()?;
        let event_pump = sdl_context.event_pump().map_err(Error::msg)?;

        Ok(Self {
            canvas,
            event_pump,
            last_frame_time: Instant::now(),
        })
    }
}

impl Frontend for SdlFrontend {
    fn render_frame(&mut self, frame: &Frame) -> Result<()> {
        let texture_creator = self.canvas.texture_creator();
        let mut texture = texture_creator.create_texture_streaming(
            PixelFormatEnum::RGB24,
            Frame::WIDTH as u32,
            Frame::HEIGHT as u32,
        )?;

        texture.update(None, &frame.pixel_data, Frame::WIDTH * 3)?;
        self.canvas.copy(&texture, None, None).map_err(Error::msg)?;
        self.canvas.present();
        Ok(())
    }

    fn handle_input(&mut self, joypad: &mut Joypad) -> Result<bool> {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return Ok(false),
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    if let Some(&button) = JOYPAD_BUTTON_MAP.get(&keycode) {
                        joypad.press_button(button);
                    }
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => {
                    if let Some(&button) = JOYPAD_BUTTON_MAP.get(&keycode) {
                        joypad.release_button(button);
                    }
                }
                _ => {}
            }
        }
        Ok(true)
    }

    fn frame_limit(&mut self) {
        let elapsed = self.last_frame_time.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
        self.last_frame_time = Instant::now();
    }
}
