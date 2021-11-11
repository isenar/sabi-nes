use crate::cartridge::Rom;
use crate::cpu::Address;
use crate::{Byte, Memory, Result};
use anyhow::bail;

const VRAM_SIZE: usize = 2048;
const RAM: Address = 0x0000;
const RAM_MIRRORS_END: Address = 0x1fff;
const PPU_REGISTERS: Address = 0x2000;
const PPU_REGISTERS_MIRRORS_END: Address = 0x3fff;

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

    fn read_prg_rom(&self, addr: Address) -> Byte {
        let mut addr = addr - 0x8000;

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
            0x8000..=0xffff => self.read_prg_rom(addr),
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

                println!("Truncated to {:x?}", mirror_base_addr);

                self.cpu_vram[mirror_base_addr as usize] = value;
            }

            PPU_REGISTERS..=PPU_REGISTERS_MIRRORS_END => {
                let _mirror_base_addr = addr & 0b0000_0111_1111_1111;

                bail!("Bus write - PPU is not implemented yet")
            }
            0x8000..=0xffff => bail!("Attempted to write into cartridge ROM (addr: {:#x})", addr),

            _ => println!("Ignoring mem-write access at {:#x?}", addr),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::MirroringType;

    fn test_rom() -> Rom {
        Rom {
            prg_rom: vec![0; 1024],
            chr_rom: vec![0; 1024],
            mapper: 1,
            screen_mirroring: MirroringType::Horizontal,
        }
    }

    #[test]
    fn write_to_ram() -> Result<()> {
        let mut bus = Bus::new(test_rom());
        bus.write(0x0012, 0xaa)?;

        assert_eq!(bus.read(0x0012)?, (0xaa));

        Ok(())
    }

    #[test]
    fn write_to_ram_with_mirroring() -> Result<()> {
        let mut bus = Bus::new(test_rom());
        bus.write(0x1eff, 0xaa)?;

        assert_eq!(bus.read(0x1eff)?, 0xaa);
        // 0x1eff truncated to 11 bits == 0x06ff
        assert_eq!(bus.read(0x06ff)?, 0xaa);

        Ok(())
    }
}
