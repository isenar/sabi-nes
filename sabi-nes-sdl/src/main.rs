mod config;
mod frontend;

use crate::config::Config;
use crate::frontend::SdlFrontend;
use clap::Parser;
use log::info;
use sabi_nes_core::{Emulator, Result, Rom};

fn main() -> Result<()> {
    env_logger::init();

    info!("Starting NES Emulator");

    let config = Config::parse();
    let rom = Rom::from_file(&config.rom_path)?;
    info!(
        "Loaded ROM: `{}`",
        config.rom_path.file_name().unwrap().display()
    );

    let mut frontend = SdlFrontend::new(&config)?;
    info!("Initialised with SDL Frontend");

    let mut emulator = Emulator::new(rom)?;
    while let Some(buttons) = frontend.poll_events() {
        emulator.set_joypad(buttons);
        emulator.step_frame()?;
        frontend.render(emulator.frame())?;
        frontend.push_audio(&emulator.drain_audio());
        frontend.frame_limit();
    }

    Ok(())
}
