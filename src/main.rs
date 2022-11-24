mod config;
mod emulator;

use crate::config::Config;
use crate::emulator::Emulator;
use clap::Parser;
use sabi_nes::Result;

fn main() -> Result<()> {
    let emu_config = Config::parse();
    let mut emulator = Emulator::create(emu_config)?;

    emulator.run()?;

    Ok(())
}
