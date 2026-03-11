use crate::frontend::Frontend;
use sabi_nes::render::{Frame, render};
use sabi_nes::{Bus, Cpu, Result, Rom};

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

        loop {
            if cpu.step()? {
                break;
            }

            if cpu.bus().is_frame_ready() {
                render(cpu.bus().ppu(), cpu.bus().mapper(), &mut self.frame)?;

                // Render frame via frontend
                self.frontend.render_frame(&self.frame)?;

                // Handle input events
                if !self.frontend.handle_input(cpu.bus_mut().joypad_mut())? {
                    break; // Frontend requested exit
                }

                // Frame rate limiting
                self.frontend.frame_limit();

                cpu.bus_mut().clear_frame_ready();
            }
        }

        Ok(())
    }
}
