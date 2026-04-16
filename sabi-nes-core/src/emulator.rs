use crate::frontend::Frontend;
use crate::render::{Frame, Renderer, SystemPalette};
use crate::{Bus, Cpu, Result, Rom};

pub struct Emulator<F> {
    frontend: F,
    frame: Frame,
    cpu: Cpu,
    palette: SystemPalette,
}

impl<F> Emulator<F>
where
    F: Frontend,
{
    pub fn new(frontend: F, rom: Rom) -> Result<Self> {
        let bus = Bus::new(rom);
        let mut cpu = Cpu::new(bus);
        cpu.reset()?;
        Ok(Self {
            frontend,
            frame: Frame::new(),
            cpu,
            palette: SystemPalette::new(),
        })
    }

    /// Advances emulation until one frame is complete.
    /// Returns `Ok(true)` to continue, `Ok(false)` to quit.
    pub fn step_frame(&mut self) -> Result<bool> {
        loop {
            self.cpu.step()?;

            if self.cpu.bus().is_frame_ready() {
                let samples = self.cpu.bus_mut().drain_audio_samples();
                let mut renderer = Renderer::new(
                    self.cpu.bus().ppu(),
                    self.cpu.bus().mapper(),
                    &mut self.frame,
                    &self.palette,
                );
                renderer.render_frame();

                self.frontend.render_frame(&self.frame)?;
                self.frontend.queue_audio(&samples);

                let should_continue = self
                    .frontend
                    .handle_input(self.cpu.bus_mut().joypad_mut())?;

                self.frontend.frame_limit();
                self.cpu.bus_mut().clear_frame_ready();

                return Ok(should_continue);
            }
        }
    }
}
