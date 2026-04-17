use crate::input::joypad::JoypadButton;
use crate::render::{Frame, Renderer, SystemPalette};
use crate::{Bus, Cpu, Result, Rom};

pub struct Emulator {
    frame: Frame,
    cpu: Cpu,
    palette: SystemPalette,
    audio_buffer: Vec<f32>,
    joypad_state: JoypadButton,
}

impl Emulator {
    pub fn new(rom: Rom) -> Result<Self> {
        let bus = Bus::new(rom);
        let mut cpu = Cpu::new(bus);
        cpu.reset()?;
        Ok(Self {
            frame: Frame::new(),
            cpu,
            palette: SystemPalette::new(),
            audio_buffer: Vec::new(),
            joypad_state: JoypadButton::empty(),
        })
    }

    pub fn set_joypad(&mut self, buttons: JoypadButton) {
        self.joypad_state = buttons;
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn drain_audio(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.audio_buffer)
    }

    pub fn step_frame(&mut self) -> Result<()> {
        self.cpu
            .bus_mut()
            .joypad_mut()
            .set_all_buttons(self.joypad_state);
        loop {
            self.cpu.step()?;
            if self.cpu.bus().is_frame_ready() {
                let samples = self.cpu.bus_mut().drain_audio_samples();
                self.audio_buffer.extend_from_slice(&samples);
                let mut renderer = Renderer::new(
                    self.cpu.bus().ppu(),
                    self.cpu.bus().mapper(),
                    &mut self.frame,
                    &self.palette,
                );
                renderer.render_frame();
                self.cpu.bus_mut().clear_frame_ready();
                return Ok(());
            }
        }
    }
}
