mod registers;

use crate::cartridge::MirroringType;
use crate::ppu::registers::{MaskRegister, ScrollRegister, StatusRegister};
use crate::{Address, Byte, Result};
use anyhow::bail;
use registers::AddressRegister;
use registers::ControlRegister;

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
    pub mask_register: MaskRegister,
    pub status_register: StatusRegister,
    pub scroll_register: ScrollRegister,

    pub oam_address: Byte,

    internal_data_buffer: Byte,
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
            mask_register: Default::default(),
            status_register: Default::default(),
            scroll_register: Default::default(),
            oam_address: 0,
        }
    }

    pub fn increment_vram_address(&mut self) {
        self.address_register
            .increment(self.control_register.vram_addr_increment());
    }

    pub fn read_status_register(&self) -> Byte {
        self.status_register.bits()
    }

    pub fn read_oam_data(&self) -> Byte {
        self.oam_data[self.oam_address as usize]
    }

    pub fn write_to_addr_register(&mut self, value: Byte) {
        self.address_register.update(value);
    }

    pub fn write_to_control_register(&mut self, value: Byte) {
        self.control_register.update(value);
    }

    pub fn write_to_mask_register(&mut self, value: Byte) {
        self.mask_register.update(value);
    }

    pub fn write_to_oam_address_register(&mut self, value: Byte) {
        self.oam_address = value;
    }

    pub fn write_to_oam_data(&mut self, value: Byte) {
        self.oam_data[self.oam_address as usize] = value;
        self.oam_address = self.oam_address.wrapping_add(1);
    }

    pub fn write_to_scroll_register(&mut self, value: Byte) {
        self.scroll_register.write(value);
    }

    pub fn write(&mut self, value: Byte) -> Result<()> {
        let addr = self.address_register.get();
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
        let addr = self.address_register.get();
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
