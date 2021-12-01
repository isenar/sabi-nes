use structopt::StructOpt;

use std::path::PathBuf;

#[derive(Debug, StructOpt)]
pub struct Config {
    #[structopt(parse(from_os_str), long = "rom-path")]
    pub rom_path: PathBuf,
    #[structopt(default_value = "256", long = "width")]
    pub window_width: u32,
    #[structopt(default_value = "240", long = "height")]
    pub window_height: u32,
    #[structopt(default_value = "3", long = "scale")]
    pub scale: u32,
}
