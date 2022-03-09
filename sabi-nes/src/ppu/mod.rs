mod nmi_status;
mod registers;

pub use nmi_status::NmiStatus;

use crate::cartridge::MirroringType;
use crate::ppu::registers::PpuRegisters;
use crate::utils::MirroredAddress;
use crate::{Address, Byte, Result};
use anyhow::bail;

const VRAM_SIZE: usize = 2048;
const PALETTE_TABLE_SIZE: usize = 64;

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

    pub scanline: u16,
    pub cycles: usize,
    pub nmi_interrupt: NmiStatus,

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
            cycles: 0,
            scanline: 0,
            nmi_interrupt: NmiStatus::Inactive,
            internal_data_buffer: Default::default(),
        }
    }

    pub fn tick(&mut self, cycles: u8) -> NmiStatus {
        self.cycles += cycles as usize;

        if self.cycles >= 341 {
            if self.is_sprite_zero_hit() {
                self.registers.set_sprite_zero_hit();
            }

            self.cycles -= 341;
            self.scanline += 1;

            if self.scanline == 241 {
                self.registers.set_vblank().reset_sprite_zero_hit();
                if self.registers.is_generating_nmi() {
                    self.nmi_interrupt = NmiStatus::Active;
                }
            }

            if self.scanline >= 262 {
                self.scanline = 0;
                self.nmi_interrupt = NmiStatus::Inactive;
                self.registers.reset_vblank().reset_sprite_zero_hit();
            }
        }

        self.nmi_interrupt
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

    pub fn read_sprite_pattern_address(&self) -> Address {
        self.registers.read_sprite_pattern_address()
    }

    pub fn write_to_addr_register(&mut self, value: Byte) {
        self.registers.write_address(value);
    }

    pub fn write_to_control_register(&mut self, value: Byte) {
        let before = self.registers.is_generating_nmi();
        self.registers.write_control(value);
        let after = self.registers.is_generating_nmi();

        if !before && after && self.registers.is_in_vblank() {
            self.nmi_interrupt = NmiStatus::Active;
        }
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

    pub fn write_to_oam_dma(&mut self, buffer: &[Byte; 256]) {
        self.registers.write_oam_dma(buffer);
    }

    pub fn write_to_scroll_register(&mut self, value: Byte) {
        self.registers.write_scroll(value);
    }

    pub fn write(&mut self, value: Byte) -> Result<()> {
        let addr = self.registers.read_address();

        match addr {
            0x0000..=0x1fff => println!("Attempted to write to CHR ROM space ({:#?})", addr),
            0x2000..=0x2fff => {
                let mirrored_addr = self.mirror_vram_addr(addr) as usize;
                self.vram[mirrored_addr] = value;
            }
            0x3000..=0x3eff => bail!("Requested invalid address from PPU ({:#x})", addr),
            0x3f00..=0x3fff => {
                let mut addr = addr as usize;
                // "Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C"
                if [0x3f10, 0x3f14, 0x3f18, 0x3f1c].contains(&addr) {
                    addr -= 0x10;
                }

                let offset_addr = addr - 0x3f00;
                self.palette_table[offset_addr] = value;
            }
            0x4000.. => bail!(
                "Unexpected access to mirrored space on PPU write ({:#x})",
                addr
            ),
        }

        self.increment_vram_address();

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
                let mut addr = addr as usize;
                // "Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C"
                if [0x3f10, 0x3f14, 0x3f18, 0x3f1c].contains(&addr) {
                    addr -= 0x10;
                }

                let offset_addr = addr - 0x3f00;
                Ok(self.palette_table[offset_addr])
            }
            0x4000.. => bail!(
                "Unexpected access to mirrored space on PPU read ({:#x})",
                addr
            ),
        }
    }

    pub fn mirror_vram_addr(&self, addr: Address) -> Address {
        let mirrored_vram_addr = addr.mirror_ppu_addr();
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

    fn is_sprite_zero_hit(&self) -> bool {
        let oam_data = self.registers.read_oam_dma();
        let y = oam_data[0].y as u16;
        let x = oam_data[0].x as usize;
        let scanline = self.scanline;

        y == scanline && x <= self.cycles && self.registers.show_sprites()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    impl Ppu {
        fn test_ppu() -> Self {
            Self::new(&[0; 2048], MirroringType::Horizontal)
        }
    }

    #[test]
    fn ppu_vram_writes() {
        let mut ppu = Ppu::test_ppu();
        ppu.write_to_addr_register(0x23);
        ppu.write_to_addr_register(0x05);
        ppu.write(0x66).expect("Failed to write");

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn ppu_vram_reads() {
        let mut ppu = Ppu::test_ppu();
        ppu.write_to_control_register(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_addr_register(0x23);
        ppu.write_to_addr_register(0x05);

        ppu.read().expect("Failed to perform dummy read");

        assert_eq!(ppu.registers.read_address(), 0x2306);
        assert_eq!(ppu.read().unwrap(), 0x66);
    }

    #[test]
    fn ppu_vram_reads_with_step_32() {
        let mut ppu = Ppu::test_ppu();

        ppu.write_to_control_register(0b0100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.registers.write_address(0x21);
        ppu.registers.write_address(0xff);

        ppu.read().expect("Failed to perform dummy read");

        assert_eq!(ppu.read().unwrap(), 0x66);
        assert_eq!(ppu.read().unwrap(), 0x77);
        assert_eq!(ppu.read().unwrap(), 0x88);
    }

    #[test]
    fn vram_horizontal_mirror() {
        let mut ppu = Ppu::test_ppu();

        ppu.registers.write_address(0x24);
        ppu.registers.write_address(0x05);

        ppu.write(0x66).unwrap();

        ppu.registers.write_address(0x28);
        ppu.registers.write_address(0x05);

        ppu.write(0x77).unwrap();

        ppu.registers.write_address(0x20);
        ppu.registers.write_address(0x05);

        ppu.read().unwrap();
        assert_eq!(ppu.read().unwrap(), 0x66);

        ppu.registers.write_address(0x2c);
        ppu.registers.write_address(0x05);

        ppu.read().unwrap();
        assert_eq!(ppu.read().unwrap(), 0x77);
    }

    #[test]
    fn vram_vertical_mirror() {
        let mut ppu = Ppu::test_ppu();
        ppu.mirroring = MirroringType::Vertical;

        ppu.registers.write_address(0x20);
        ppu.registers.write_address(0x05);

        ppu.write(0x66).unwrap();

        ppu.registers.write_address(0x2c);
        ppu.registers.write_address(0x05);

        ppu.write(0x77).unwrap();

        ppu.registers.write_address(0x28);
        ppu.registers.write_address(0x05);

        ppu.read().unwrap();
        assert_eq!(ppu.read().unwrap(), 0x66);

        ppu.registers.write_address(0x24);
        ppu.registers.write_address(0x05);

        ppu.read().unwrap();
        assert_eq!(ppu.read().unwrap(), 0x77);
    }

    #[test]
    fn reading_status_resets_latch() {
        let mut ppu = Ppu::test_ppu();
        ppu.vram[0x0305] = 0x66;

        ppu.registers.write_address(0x21);
        ppu.registers.write_address(0x23);
        ppu.registers.write_address(0x05);

        ppu.read().unwrap();
        assert_ne!(ppu.read().unwrap(), 0x66);

        ppu.read_status_register();

        ppu.registers.write_address(0x23);
        ppu.registers.write_address(0x05);

        ppu.read().unwrap();
        assert_eq!(ppu.read().unwrap(), 0x66);
    }

    #[test]
    fn vram_mirroring() {
        let mut ppu = Ppu::test_ppu();
        ppu.write_to_control_register(0);
        ppu.vram[0x0305] = 0x66;

        ppu.registers.write_address(0x63);
        ppu.registers.write_address(0x05);

        ppu.read().unwrap();
        assert_matches!(ppu.read(), Ok(0x66));
    }

    #[test]
    fn reading_status_resets_vblank() {
        let mut ppu = Ppu::test_ppu();
        ppu.registers.set_vblank();

        let status = ppu.read_status_register();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.registers.read_status() >> 7, 0);
    }

    #[test]
    fn oam_read_write() {
        let mut ppu = Ppu::test_ppu();
        ppu.write_to_oam_address_register(0x10);
        ppu.write_to_oam_data(0x66);
        ppu.write_to_oam_data(0x77);

        ppu.write_to_oam_address_register(0x10);
        assert_eq!(ppu.read_oam_data(), 0x66);

        ppu.write_to_oam_address_register(0x11);
        assert_eq!(ppu.read_oam_data(), 0x77);
    }
}
