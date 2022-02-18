pub mod mappers;
mod mirroring_type;
mod rom;

pub use mirroring_type::MirroringType;
pub use rom::Rom;

pub const PRG_ROM_BANK_SIZE: usize = 16384;
pub const CHR_ROM_BANK_SIZE: usize = 8192;
