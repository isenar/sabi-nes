use crate::cartridge::Rom;
use crate::cpu::Address;
use crate::{Byte, Memory, Result};
use anyhow::bail;

const VRAM_SIZE: usize = 2048;
const RAM: Address = 0x0000;
const RAM_MIRRORS_END: Address = 0x1fff;
const PPU_REGISTERS: Address = 0x2000;
const PPU_REGISTERS_MIRRORS_END: Address = 0x3fff;
const ROM_START: Address = 0x8000;
const ROM_END: Address = 0xffff;

#[derive(Debug)]
pub struct Bus {
    cpu_vram: [Byte; VRAM_SIZE],
    rom: Rom,
}

impl Bus {
    pub fn new(rom: Rom) -> Self {
        Self {
            cpu_vram: [0; VRAM_SIZE],
            rom,
        }
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
    fn read(&self, addr: Address) -> Result<Byte> {
        Ok(match addr {
            RAM..=RAM_MIRRORS_END => {
                // truncate to 11 bits
                let mirror_base_addr = addr & 0b0000_0111_1111_1111;

                self.cpu_vram[mirror_base_addr as usize]
            }
            PPU_REGISTERS..=PPU_REGISTERS_MIRRORS_END => {
                let _mirror_base_addr = addr & 0b0000_0111_1111_1111;

                bail!("Bus read - PPU is not implemented yet")
            }
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

            PPU_REGISTERS..=PPU_REGISTERS_MIRRORS_END => {
                let _mirror_base_addr = addr & 0b0000_0111_1111_1111;

                bail!("Bus write - PPU is not implemented yet")
            }
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
        let bus = Bus::new(test_rom());

        assert_matches!(bus.read(0x9000), Ok(0x10));
    }

    #[test]
    fn write_to_cartridge_rom_fails() {
        let mut bus = Bus::new(test_rom());

        assert_matches!(bus.write(0x9000, 0xef), Err(_));
    }
}
