use crate::Config;
use crate::frontend::Frontend;
use anyhow::Error;
use maplit::hashmap;
use once_cell::sync::Lazy;
use sabi_nes::Result;
use sabi_nes::input::joypad::{Joypad, JoypadButton};
use sabi_nes::render::Frame;
use sdl2::EventPump;
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::WindowCanvas;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const TARGET_FPS: u64 = 60;
const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / TARGET_FPS);

// Allow ~4 frames of audio in the SDL queue before we stop pushing.
// At 44100 Hz, 1 frame ≈ 735 samples × 4 bytes = 2940 bytes.
const MAX_AUDIO_QUEUE_BYTES: u32 = 2940 * 4;

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
    audio_queue: AudioQueue<f32>,
    fps_counter: u32,
    fps_timer: Instant,
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

        let canvas = window.into_canvas().build()?;
        let event_pump = sdl_context.event_pump().map_err(Error::msg)?;

        let audio_subsystem = sdl_context.audio().map_err(Error::msg)?;
        let spec = AudioSpecDesired {
            freq: Some(44_100),
            channels: Some(1),
            samples: Some(1024),
        };
        let audio_queue = audio_subsystem
            .open_queue::<f32, _>(None, &spec)
            .map_err(Error::msg)?;
        audio_queue.resume();

        Ok(Self {
            canvas,
            event_pump,
            last_frame_time: Instant::now(),
            audio_queue,
            fps_counter: 0,
            fps_timer: Instant::now(),
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

        texture.update(None, frame.pixel_data(), Frame::WIDTH * 3)?;
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
            let remaining = FRAME_DURATION - elapsed;
            // Sleep for most of the time (OS sleep is coarse-grained, typically
            // 1–10 ms on macOS/Linux). Leave the last 2 ms for a spin-wait so
            // we don't overshoot the target by a full scheduler quantum.
            if remaining > Duration::from_millis(2) {
                std::thread::sleep(remaining - Duration::from_millis(2));
            }
            while self.last_frame_time.elapsed() < FRAME_DURATION {
                std::hint::spin_loop();
            }
        }
        self.last_frame_time = Instant::now();

        // Update window title with measured FPS once per second.
        self.fps_counter += 1;
        let fps_elapsed = self.fps_timer.elapsed();
        if fps_elapsed >= Duration::from_secs(1) {
            let fps = self.fps_counter as f64 / fps_elapsed.as_secs_f64();
            let _ = self
                .canvas
                .window_mut()
                .set_title(&format!("Sabi NES — {fps:.1} fps"));
            self.fps_counter = 0;
            self.fps_timer = Instant::now();
        }
    }

    fn queue_audio(&mut self, samples: &[f32]) {
        // Don't let the queue grow unboundedly if the emulator runs faster than
        // the audio device drains it.
        if self.audio_queue.size() < MAX_AUDIO_QUEUE_BYTES {
            let _ = self.audio_queue.queue_audio(samples);
        }
    }
}
