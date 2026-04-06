mod config;
mod emulator;
mod frontend;

use crate::config::Config;
use crate::emulator::Emulator;
use crate::frontend::SdlFrontend;
use clap::Parser;
use log::info;
use sabi_nes_core::{Result, Rom};

fn main() -> Result<()> {
    env_logger::init();

    info!("Starting NES Emulator");

    let config = Config::parse();
    let rom = Rom::from_file(&config.rom_path)?;
    info!(
        "Loaded ROM: `{}`",
        config.rom_path.file_name().unwrap().display()
    );

    let frontend = SdlFrontend::new(&config)?;
    info!("Initialised with SDL Frontend");

    let mut emulator = Emulator::new(frontend);
    emulator.run(rom)?;

    Ok(())
}
