use sabi_nes_core::frontend::Frontend;
use sabi_nes_core::render::{Frame, Renderer, SystemPalette};
use sabi_nes_core::{Bus, Cpu, Result, Rom};

pub struct Emulator<F> {
    frontend: F,
    frame: Frame,
}

impl<F> Emulator<F>
where
    F: Frontend,
{
    pub fn new(frontend: F) -> Self {
        Self {
            frontend,
            frame: Frame::new(),
        }
    }

    pub fn run(&mut self, rom: Rom) -> Result<()> {
        let bus = Bus::new(rom);
        let mut cpu = Cpu::new(bus);
        cpu.reset()?;
        let palette = SystemPalette::new();

        loop {
            cpu.step()?;

            if cpu.bus().is_frame_ready() {
                let samples = cpu.bus_mut().drain_audio_samples();
                let mut renderer = Renderer::new(
                    cpu.bus().ppu(),
                    cpu.bus().mapper(),
                    &mut self.frame,
                    &palette,
                );
                renderer.render_frame()?;

                self.frontend.render_frame(&self.frame)?;
                self.frontend.queue_audio(&samples);

                if !self.frontend.handle_input(cpu.bus_mut().joypad_mut())? {
                    break;
                }

                self.frontend.frame_limit();

                cpu.bus_mut().clear_frame_ready();
            }
        }

        Ok(())
    }
}
