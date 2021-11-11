use crate::cartridge::Rom;
use crate::cpu::Address;
use crate::{Byte, Memory};

const RAM: Address = 0x0000;
const RAM_MIRRORS_END: Address = 0x1fff;
const PPU_REGISTERS: Address = 0x2000;
const PPU_REGISTERS_MIRRORS_END: Address = 0x3fff;

#[derive(Debug)]
pub struct Bus {
    cpu_vram: [Byte; 2048],
    rom: Rom,
}

impl Bus {
    pub fn new(rom: Rom) -> Self {
        Self {
            cpu_vram: [0; 2048],
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
    fn read(&self, addr: Address) -> Byte {
        match addr {
            RAM..=RAM_MIRRORS_END => {
                // truncate to 11 bits
                let mirror_base_addr = addr & 0b0000_0111_1111_1111;

                self.cpu_vram[mirror_base_addr as usize]
            }
            PPU_REGISTERS..=PPU_REGISTERS_MIRRORS_END => {
                let _mirror_base_addr = addr & 0b0000_0111_1111_1111;

                todo!("Bus read - PPU is not implemented yet")
            }
            0x8000..=0xffff => self.read_prg_rom(addr),
            _ => {
                println!("Ignoring mem access at {:x?}", addr);
                0
            }
        }
    }

    fn write(&mut self, addr: Address, value: Byte) {
        match addr {
            RAM..=RAM_MIRRORS_END => {
                let mirror_base_addr = addr & 0b0000_0111_1111_1111;
                self.cpu_vram[mirror_base_addr as usize] = value;
            }

            PPU_REGISTERS..=PPU_REGISTERS_MIRRORS_END => {
                let _mirror_base_addr = addr & 0b0000_0111_1111_1111;

                todo!("Bus write - PPU is not implemented yet")
            }
            0x8000..=0xffff => panic!("Attempted to write into cartridge ROM (addr: {:#x})", addr),

            _ => println!("Ignoring mem-write access at {:#x?}", addr),
        }
    }
}
