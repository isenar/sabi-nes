mod address_register;
mod control_register;

use crate::cartridge::MirroringType;
use crate::ppu::control_register::ControlRegister;
use crate::{Address, Byte, Result};
use address_register::AddressRegister;
use anyhow::bail;

use std::cell::Cell;

#[derive(Debug)]
pub struct Ppu {
    /// Visuals of game stored on cartridge
    pub chr_rom: Vec<Byte>,
    /// Internal memory to keep palette tables used by the screen
    pub palette_table: [Byte; 32],
    /// 2KiB of space to hold background information
    pub vram: [Byte; 2048],
    /// Internal memory to keep state of sprites (Object Attribute Memory)
    pub oam_data: [Byte; 256],
    /// Mirroring type
    pub mirroring: MirroringType,

    pub address_register: AddressRegister,
    pub control_register: ControlRegister,

    internal_data_buffer: Cell<Byte>,
}

impl Ppu {
    pub fn new(chr_rom: &[Byte], mirroring: MirroringType) -> Self {
        Self {
            chr_rom: chr_rom.into(),
            palette_table: [0; 32],
            vram: [0; 2048],
            oam_data: [0; 256],
            mirroring,
            address_register: Default::default(),
            control_register: Default::default(),
            internal_data_buffer: Default::default(),
        }
    }

    #[allow(unused)]
    pub fn increment_vram_address(&mut self) {
        self.address_register
            .increment(self.control_register.vram_addr_increment());
    }

    pub fn write_to_addr_register(&mut self, value: Byte) {
        self.address_register.update(value);
    }

    pub fn write_to_control_register(&mut self, value: Byte) {
        self.control_register.update(value);
    }

    pub fn read(&self) -> Result<Byte> {
        let addr = self.address_register.get();
        // TODO:
        // self.increment_vram_address();

        match addr {
            0x0000..=0x1fff => {
                let result = self.internal_data_buffer.get();
                self.internal_data_buffer.set(self.chr_rom[addr as usize]);

                Ok(result)
            }
            0x2000..=0x2fff => {
                let result = self.internal_data_buffer.get();
                let mirrored_addr = self.mirror_vram_addr(addr);
                self.internal_data_buffer
                    .set(self.vram[mirrored_addr as usize]);

                Ok(result)
            }
            0x3000..=0x3eff => bail!("Requested invalid address from PPU ({:#x})", addr),
            0x3f00..=0x3fff => {
                let offset_addr = addr - 0x3f00;

                Ok(self.palette_table[offset_addr as usize])
            }
            0x4000.. => bail!("Unexpected access to mirrored space {:#x}", addr),
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
