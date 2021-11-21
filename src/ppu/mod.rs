mod registers;

use crate::cartridge::MirroringType;
use crate::ppu::registers::PpuRegisters;
use crate::{Address, Byte, Result};
use anyhow::bail;

const VRAM_SIZE: usize = 2048;
const PALETTE_TABLE_SIZE: usize = 32;

#[derive(Debug)]
pub struct Ppu {
    /// Visuals of game stored on cartridge
    pub chr_rom: Vec<Byte>,
    /// Internal memory to keep palette tables used by the screen
    pub palette_table: [Byte; PALETTE_TABLE_SIZE],
    /// 2KiB of space to hold background information
    pub vram: [Byte; VRAM_SIZE],

    /// Mirroring type
    pub mirroring: MirroringType,

    /// PPU registers
    pub registers: PpuRegisters,

    internal_data_buffer: Byte,
}

impl Ppu {
    pub fn new(chr_rom: &[Byte], mirroring: MirroringType) -> Self {
        Self {
            chr_rom: chr_rom.into(),
            palette_table: [0; PALETTE_TABLE_SIZE],
            vram: [0; VRAM_SIZE],
            mirroring,
            registers: Default::default(),
            internal_data_buffer: Default::default(),
        }
    }

    pub fn increment_vram_address(&mut self) {
        self.registers.increment_vram_address();
    }

    pub fn read_status_register(&mut self) -> Byte {
        self.registers.read_status()
    }

    pub fn read_oam_data(&self) -> Byte {
        self.registers.read_oam_data()
    }

    pub fn write_to_addr_register(&mut self, value: Byte) {
        self.registers.write_address(value);
    }

    pub fn write_to_control_register(&mut self, value: Byte) {
        self.registers.write_control(value);
    }

    pub fn write_to_mask_register(&mut self, value: Byte) {
        self.registers.write_mask(value);
    }

    pub fn write_to_oam_address_register(&mut self, value: Byte) {
        self.registers.write_oam_address(value);
    }

    pub fn write_to_oam_data(&mut self, value: Byte) {
        self.registers.write_oam_data(value);
    }

    pub fn write_to_scroll_register(&mut self, value: Byte) {
        self.registers.write_scroll(value);
    }

    pub fn write(&mut self, value: Byte) -> Result<()> {
        let addr = self.registers.read_address();
        self.increment_vram_address();

        match addr {
            0x0000..=0x1fff => bail!("Attempted to write to CHR ROM space ({:#?})", addr),
            0x2000..=0x2fff => {
                let mirrored_addr = self.mirror_vram_addr(addr);
                self.vram[mirrored_addr as usize] = value;
            }
            0x3000..=0x3eff => bail!("Requested invalid address from PPU ({:#x})", addr),
            0x3f00..=0x3fff => {
                let offset_addr = addr - 0x3f00;
                self.palette_table[offset_addr as usize] = value;
            }
            0x4000.. => bail!(
                "Unexpected access to mirrored space on PPU write ({:#x})",
                addr
            ),
        }

        Ok(())
    }

    pub fn read(&mut self) -> Result<Byte> {
        let addr = self.registers.read_address();
        self.increment_vram_address();

        match addr {
            0x0000..=0x1fff => {
                let result = self.internal_data_buffer;
                self.internal_data_buffer = self.chr_rom[addr as usize];

                Ok(result)
            }
            0x2000..=0x2fff => {
                let result = self.internal_data_buffer;
                let mirrored_addr = self.mirror_vram_addr(addr);
                self.internal_data_buffer = self.vram[mirrored_addr as usize];

                Ok(result)
            }
            0x3000..=0x3eff => bail!("Requested invalid address from PPU ({:#x})", addr),
            0x3f00..=0x3fff => {
                let offset_addr = addr - 0x3f00;
                Ok(self.palette_table[offset_addr as usize])
            }
            0x4000.. => bail!(
                "Unexpected access to mirrored space on PPU read ({:#x})",
                addr
            ),
        }
    }

    pub fn mirror_vram_addr(&self, addr: Address) -> Address {
        let mirrored_vram_addr = addr & 0b0010_1111_1111_1111;
        let vram_index = mirrored_vram_addr - 0x2000;
        let name_table = vram_index / 0x0400;

        let offset = match (self.mirroring, name_table) {
            (MirroringType::Vertical, 2 | 3) => 0x800,
            (MirroringType::Horizontal, 1 | 2) => 0x400,
            (MirroringType::Horizontal, 3) => 0x800,
            _ => 0x000,
        };

        vram_index - offset
    }
}
