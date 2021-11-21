use crate::cartridge::Rom;
use crate::cpu::Address;
use crate::ppu::Ppu;
use crate::{Byte, Memory, Result};
use anyhow::bail;

const VRAM_SIZE: usize = 2048;
const RAM: Address = 0x0000;
const RAM_MIRRORS_END: Address = 0x1fff;
const PPU_REGISTERS_MIRRORS_START: Address = 0x2008;
const PPU_REGISTERS_MIRRORS_END: Address = 0x3fff;
const ROM_START: Address = 0x8000;
const ROM_END: Address = 0xffff;

#[derive(Debug)]
pub struct Bus {
    cpu_vram: [Byte; VRAM_SIZE],
    rom: Rom,
    ppu: Ppu,
    cycles: usize,
}

impl Bus {
    pub fn new(rom: Rom) -> Self {
        let ppu = Ppu::new(&rom.chr_rom, rom.screen_mirroring);

        Self {
            cpu_vram: [0; VRAM_SIZE],
            rom,
            ppu,
            cycles: 0,
        }
    }

    pub fn tick(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
        self.ppu.tick(cycles * 3);
    }

    fn read_prg_rom(&self, mut addr: Address) -> Byte {
        addr -= ROM_START;

        if self.rom.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            //mirror if needed
            addr %= 0x4000;
        }

        self.rom.prg_rom[addr as usize]
    }
}

impl Memory for Bus {
    fn read(&mut self, addr: Address) -> Result<Byte> {
        Ok(match addr {
            RAM..=RAM_MIRRORS_END => {
                // truncate to 11 bits
                let mirror_base_addr = addr & 0b0000_0111_1111_1111;

                self.cpu_vram[mirror_base_addr as usize]
            }
            0x2000 => bail!("Attempted to read from write-only PPU control register"),
            0x2001 => bail!("Attempted to read from write-only PPU mask register"),
            0x2002 => self.ppu.read_status_register(),
            0x2003 => bail!("Attempted to read from write-only PPU OAM address register"),
            0x2004 => self.ppu.read_oam_data(),
            0x2005 => bail!("Attempted to read from write-only PPU scroll register"),
            0x2006 => bail!("Attempted to read from write-only PPU address register"),
            0x2007 => self.ppu.read()?,
            PPU_REGISTERS_MIRRORS_START..=PPU_REGISTERS_MIRRORS_END => {
                let mirror_base_addr = addr & 0b0000_0111_1111_1111;
                self.read(mirror_base_addr)?
            }
            0x4014 => bail!("Attempted to read from write-only PPU OAM DMA register"),
            ROM_START..=ROM_END => self.read_prg_rom(addr),
            _ => {
                println!("Ignoring mem access at {:x?}", addr);
                0
            }
        })
    }

    fn write(&mut self, addr: Address, value: Byte) -> Result<()> {
        match addr {
            RAM..=RAM_MIRRORS_END => {
                let mirror_base_addr = addr & 0b0000_0111_1111_1111;
                self.cpu_vram[mirror_base_addr as usize] = value;
            }
            0x2000 => self.ppu.write_to_control_register(value),
            0x2001 => self.ppu.write_to_mask_register(value),
            0x2002 => bail!("Attempted to write to PPU status register"),
            0x2003 => self.ppu.write_to_oam_address_register(value),
            0x2004 => self.ppu.write_to_oam_data(value),
            0x2005 => self.ppu.write_to_scroll_register(value),
            0x2006 => self.ppu.write_to_addr_register(value),
            0x2007 => self.ppu.write(value)?,
            PPU_REGISTERS_MIRRORS_START..=PPU_REGISTERS_MIRRORS_END => {
                let mirror_base_addr = addr & 0b0000_0111_1111_1111;

                self.write(mirror_base_addr, value)?
            }
            0x4014 => todo!("Write to OAM DMA"),
            ROM_START..=ROM_END => {
                bail!("Attempted to write into cartridge ROM (addr: {:#x})", addr)
            }

            _ => println!("Ignoring mem-write access at {:#x?}", addr),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::MirroringType;
    use assert_matches::assert_matches;

    fn test_rom() -> Rom {
        Rom {
            prg_rom: vec![0x10; 8192],
            chr_rom: vec![0x20; 1024],
            mapper: 1,
            screen_mirroring: MirroringType::Horizontal,
        }
    }

    #[test]
    fn write_to_ram() {
        let mut bus = Bus::new(test_rom());
        bus.write(0x0012, 0xaa).expect("Failed to write to RAM");

        assert_matches!(bus.read(0x0012), Ok(0xaa));
    }

    #[test]
    fn write_to_ram_with_mirroring() {
        let mut bus = Bus::new(test_rom());
        bus.write(0x1eff, 0xaa).expect("Failed to write to RAM");

        assert_matches!(bus.read(0x1eff), Ok(0xaa));
        // 0x1eff truncated to 11 bits == 0x06ff
        assert_matches!(bus.read(0x06ff), Ok(0xaa));
    }

    #[test]
    fn read_from_cartridge_rom() {
        let mut bus = Bus::new(test_rom());

        assert_matches!(bus.read(0x9000), Ok(0x10));
    }

    #[test]
    fn write_to_cartridge_rom_fails() {
        let mut bus = Bus::new(test_rom());

        assert_matches!(bus.write(0x9000, 0xef), Err(_));
    }
}
