use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Config {
    #[arg(long = "rom-path")]
    pub rom_path: PathBuf,
    #[arg(default_value = "256", long = "width")]
    pub window_width: u32,
    #[arg(default_value = "240", long = "height")]
    pub window_height: u32,
    #[arg(default_value = "3", long = "scale")]
    pub scale: u32,
}

impl Config {
    pub fn window_width(&self) -> u32 {
        self.window_width * self.scale
    }

    pub fn window_height(&self) -> u32 {
        self.window_height * self.scale
    }
}
