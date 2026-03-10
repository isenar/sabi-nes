mod config;
mod emulator;
mod frontend;
mod sdl_frontend;

use crate::config::Config;
use crate::emulator::Emulator;
use crate::sdl_frontend::SdlFrontend;
use clap::Parser;
use log::debug;
use sabi_nes::{Result, Rom};

fn main() -> Result<()> {
    env_logger::init();

    debug!("Starting NES Emulator");

    let config = Config::parse();
    let rom = Rom::from_file(&config.rom_path)?;
    let frontend = SdlFrontend::new(&config)?;

    debug!("Initialised with SDL Frontend");

    let mut emulator = Emulator::new(frontend);
    emulator.run(rom)?;

    Ok(())
}
