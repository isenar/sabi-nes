//! MMC1 (Mapper 1) - One of the most common NES mappers
//!
//! The MMC1 uses a serial shift register for configuration:
//! - Writes to $8000-$FFFF are shifted into a 5-bit shift register
//! - On the 5th write, the register value is written to an internal register
//! - Writing a value with bit 7 set resets the shift register
//!
//! Memory Map:
//! - CPU $6000-$7FFF: 8KB PRG RAM (optional, battery-backed)
//! - CPU $8000-$BFFF: 16KB PRG ROM bank (switchable or fixed to the first bank)
//! - CPU $C000-$FFFF: 16KB PRG ROM bank (switchable or fixed to the last bank)
//! - PPU $0000-$0FFF: 4KB CHR bank (switchable)
//! - PPU $1000-$1FFF: 4KB CHR bank (switchable)

use crate::cartridge::mappers::{Mapper, MapperId};
use crate::{Address, Byte, Result};

#[derive(Debug)]
pub struct Mmc1 {
    /// 5-bit shift register, bit 0 = next empty slot
    shift_register: Byte,
    /// Number of writes to shift register (0-4)
    shift_count: u8,

    /// Control register ($8000-$9FFF)
    /// Bits:
    /// 0-1: Mirroring (0=one-screen lower, 1=one-screen upper, 2=vertical, 3=horizontal)
    /// 2-3: PRG ROM bank mode
    /// 4:   CHR ROM bank mode
    control: Byte,

    /// CHR bank 0 register ($A000-$BFFF)
    chr_bank_0: Byte,

    /// CHR bank 1 register ($C000-$DFFF)
    chr_bank_1: Byte,

    /// PRG bank register ($E000-$FFFF)
    /// Bits 0-3: PRG ROM bank
    /// Bit 4: PRG RAM enabled (0=enabled)
    prg_bank: Byte,

    /// Number of PRG ROM banks (16KB each)
    prg_rom_banks: usize,

    /// Number of CHR ROM banks (4KB each for MMC1, since it switches 4KB at a time)
    #[allow(dead_code)]
    chr_rom_banks: usize,
}

impl Mmc1 {
    pub fn new(prg_rom_banks: usize, chr_rom_banks: usize) -> Self {
        Self {
            shift_register: Byte::new(0x10), // Bit 5 set indicates empty
            shift_count: 0,
            control: Byte::new(0x0C), // Default: last bank fixed, 8KB CHR mode
            chr_bank_0: Byte::default(),
            chr_bank_1: Byte::default(),
            prg_bank: Byte::default(),
            prg_rom_banks,
            chr_rom_banks,
        }
    }

    /// Write to the MMC1 registers via shift register
    /// Called when CPU writes to $8000-$FFFF
    fn write_register(&mut self, address: Address, value: Byte) {
        // Reset if bit 7 is set
        if value & 0x80 != 0 {
            self.shift_register = 0x10.into();
            self.shift_count = 0;
            self.control |= 0x0C; // Set to default mode
            return;
        }

        // Shift in the bit
        self.shift_register >>= 1;
        self.shift_register |= (value & 1) << 4;
        self.shift_count += 1;

        // After 5 writes, write to the internal register
        if self.shift_count == 5 {
            let register_value = self.shift_register;

            // Determine which register based on address
            match address.value() {
                0x8000..=0x9FFF => {
                    self.control = register_value;
                }
                0xA000..=0xBFFF => self.chr_bank_0 = register_value,
                0xC000..=0xDFFF => self.chr_bank_1 = register_value,
                0xE000..=0xFFFF => {
                    self.prg_bank = register_value;
                }
                _ => {}
            }

            // Reset shift register
            self.shift_register = Byte::new(0x10);
            self.shift_count = 0;
        }
    }

    /// Map PRG ROM address (0-based offset 0x0000-0x7FFF from CPU $8000-$FFFF)
    fn map_prg_address(&self, address: Address) -> usize {
        let bank_mode = (self.control >> 2) & 0b11;
        let prg_bank_num = (self.prg_bank & 0x0F).as_usize();

        match bank_mode.value() {
            0 | 1 => {
                // 32KB mode: ignore low bit of bank number
                let bank = (prg_bank_num >> 1) % (self.prg_rom_banks / 2);
                bank * 0x8000 + address.as_usize()
            }
            2 => {
                // Fix first bank at $8000, switch $C000
                if address < 0x4000 {
                    address.as_usize() // First 16KB bank
                } else {
                    let bank = prg_bank_num % self.prg_rom_banks;
                    bank * 0x4000 + (address.as_usize() & 0x3FFF)
                }
            }
            3 => {
                // Switch $8000, fix last bank at $C000
                if address < 0x4000 {
                    let bank = prg_bank_num % self.prg_rom_banks;
                    bank * 0x4000 + address.as_usize()
                } else {
                    let last_bank = self.prg_rom_banks - 1;
                    last_bank * 0x4000 + (address.as_usize() & 0x3FFF)
                }
            }
            _ => unreachable!(),
        }
    }

    /// Map CHR ROM/RAM address (PPU $0000-$1FFF)
    #[allow(dead_code)]
    fn map_chr_address(&self, address: Address) -> usize {
        // If no CHR banks (CHR-RAM), just pass through the address
        if self.chr_rom_banks == 0 {
            return address.as_usize();
        }

        let chr_mode = (self.control >> 4) & 1;

        match chr_mode.value() {
            0 => {
                // 8KB mode: use CHR bank 0, ignore low bit
                let bank = ((self.chr_bank_0 >> 1).as_usize()) % (self.chr_rom_banks / 2);
                bank * 0x2000 + address.as_usize() // TODO
            }
            1 => {
                // 4KB mode: two separate 4KB banks
                if address < 0x1000 {
                    let bank = self.chr_bank_0.as_usize() % self.chr_rom_banks;
                    bank * 0x1000 + (address.as_usize() & 0x0FFF) // TODO
                } else {
                    let bank = self.chr_bank_1.as_usize() % self.chr_rom_banks;
                    bank * 0x1000 + (address.as_usize() & 0x0FFF) // TODO
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Mapper for Mmc1 {
    fn map_address(&self, address: Address) -> Result<usize> {
        // Only PRG ROM addresses come through here (CPU reads from $8000-$FFFF)
        // PPU reads CHR ROM directly without going through the mapper
        Ok(self.map_prg_address(address))
    }

    fn write(&mut self, address: Address, value: Byte) {
        if address >= 0x8000 {
            self.write_register(address, value);
        }
    }
}

impl MapperId for Mmc1 {
    const ID: u32 = 1;
}
