use crate::apu::Apu;
use crate::cartridge::Rom;
use crate::input::joypad::Joypad;
use crate::ppu::{NmiStatus, Ppu};
use crate::utils::MirroredAddress;
use crate::{Address, Byte, Memory, Result};
use anyhow::bail;
use log::debug;

const VRAM_SIZE: usize = 2048;
const PRG_RAM_SIZE: usize = 8192;
const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1fff;
const PPU_REGISTERS_MIRRORS_START: u16 = 0x2008;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3fff;
const PRG_RAM_START: u16 = 0x6000;
const PRG_RAM_END: u16 = 0x7fff;
const ROM_START: u16 = 0x8000;
const ROM_END: u16 = 0xffff;

pub struct Bus {
    cpu_vram: [Byte; VRAM_SIZE],
    prg_ram: [Byte; PRG_RAM_SIZE],
    rom: Rom,
    ppu: Ppu,
    apu: Apu,
    joypad: Joypad,
    cycles: usize,
    frame_ready: bool,
}

impl Bus {
    pub fn new(rom: Rom) -> Bus {
        let ppu = Ppu::new(&rom.chr_rom, rom.screen_mirroring);

        Bus {
            cpu_vram: [Byte::default(); VRAM_SIZE],
            prg_ram: [Byte::default(); PRG_RAM_SIZE],
            rom,
            ppu,
            apu: Apu::default(),
            joypad: Joypad::default(),
            cycles: 0,
            frame_ready: false,
        }
    }

    pub fn tick(&mut self, cycles: usize) -> Result<()> {
        self.cycles += cycles;

        let nmi_before = self.ppu.nmi_interrupt;
        let nmi_after = self.ppu.tick(cycles * 3);

        if NmiStatus::activated(nmi_before, nmi_after) {
            self.frame_ready = true;
        }

        Ok(())
    }

    pub fn poll_nmi_status(&mut self) -> NmiStatus {
        let current = self.ppu.nmi_interrupt;
        self.ppu.nmi_interrupt = NmiStatus::Inactive;

        current
    }

    pub fn is_frame_ready(&self) -> bool {
        self.frame_ready
    }

    pub fn clear_frame_ready(&mut self) {
        self.frame_ready = false;
    }

    pub fn ppu(&self) -> &Ppu {
        &self.ppu
    }

    pub fn joypad_mut(&mut self) -> &mut Joypad {
        &mut self.joypad
    }
}

impl Memory for Bus {
    fn read_byte(&mut self, address: Address) -> Result<Byte> {
        Ok(match address.value() {
            RAM..=RAM_MIRRORS_END => {
                let mirror_base_addr: usize = address.mirror_cpu_vram_addr().into();
                self.cpu_vram[mirror_base_addr]
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
                let mirror_base_addr = address.mirror_ppu_registers_addr();
                self.read_byte(mirror_base_addr)?
            }
            0x4000 => self.apu.square_channel1.volume,
            0x4001 => self.apu.square_channel1.sweep,
            0x4002 => self.apu.square_channel1.timer_low,
            0x4003 => self.apu.square_channel1.length_and_timer_high,
            0x4004 => self.apu.square_channel2.volume,
            0x4005 => self.apu.square_channel2.sweep,
            0x4006 => self.apu.square_channel2.timer_low,
            0x4007 => self.apu.square_channel2.length_and_timer_high,
            0x4008 => self.apu.triangle_channel.linear_counter,
            // 0x4009 is unused
            0x400a => self.apu.triangle_channel.timer_low,
            0x400b => self.apu.triangle_channel.length_and_timer_high,
            0x400c => self.apu.noise_channel.volume,
            // 0x400d is unused
            0x400e => self.apu.noise_channel.mode_and_period,
            0x400f => self.apu.noise_channel.len_counter_and_env_restart,
            0x4014 => bail!("Attempted to read from write-only PPU OAM DMA register"),
            0x4015 => self.apu.flags.bits().into(),
            0x4016 => self.joypad.read(),
            0x4017 => 0.into(), // TODO: Frame Counter impl
            PRG_RAM_START..=PRG_RAM_END => {
                let index = (address - PRG_RAM_START).as_usize();
                self.prg_ram[index]
            }
            ROM_START..=ROM_END => {
                let address = address - ROM_START;
                let mapped_address = self.rom.mapper.map_address(address)?;
                self.rom.prg_rom[mapped_address]
            }
            _ => {
                debug!("Ignored attempt to read address ${address:0X}");
                0.into()
            }
        })
    }

    fn write_byte(&mut self, address: Address, value: Byte) -> Result<()> {
        match address.value() {
            RAM..=RAM_MIRRORS_END => {
                let mirror_base_addr: usize = address.mirror_cpu_vram_addr().into();
                self.cpu_vram[mirror_base_addr] = value;
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
                let mirror_base_addr = address.mirror_ppu_registers_addr();

                self.write_byte(mirror_base_addr, value)?;
            }
            0x4000 => self.apu.square_channel1.volume = value,
            0x4001 => self.apu.square_channel1.sweep = value,
            0x4002 => self.apu.square_channel1.timer_low = value,
            0x4003 => self.apu.square_channel1.length_and_timer_high = value,
            0x4004 => self.apu.square_channel2.volume = value,
            0x4005 => self.apu.square_channel2.sweep = value,
            0x4006 => self.apu.square_channel2.timer_low = value,
            0x4007 => self.apu.square_channel2.length_and_timer_high = value,
            0x4008 => self.apu.triangle_channel.linear_counter = value,
            // 0x4009 is unused
            0x400a => self.apu.triangle_channel.timer_low = value,
            0x400b => self.apu.triangle_channel.length_and_timer_high = value,
            0x400c => self.apu.noise_channel.volume = value,
            // 0x400d is unused
            0x400e => self.apu.noise_channel.mode_and_period = value,
            0x400f => self.apu.noise_channel.len_counter_and_env_restart = value,
            0x4010 => self.apu.dmc.flags_and_rate = value,
            0x4011 => self.apu.dmc.direct_load = value,
            0x4012 => self.apu.dmc.sample_address = value,
            0x4013 => self.apu.dmc.sample_length = value,
            0x4014 => {
                let mut buffer = [Byte::default(); 256];
                let high = value.as_word() << 8;
                // We could use std::array::try_from_fn to create the buffer once it gets stabilised,
                // for now we'll use the good old for loop
                for (offset, byte) in buffer.iter_mut().enumerate() {
                    let address = (high + offset as u16).as_address();
                    *byte = self.read_byte(address)?;
                }

                self.ppu.write_to_oam_dma(&buffer);
            }
            0x4015 => self.apu.set_status_register(value),
            0x4016 => self.joypad.write(value),
            0x4017 => {} // TODO: Frame Counter impl
            PRG_RAM_START..=PRG_RAM_END => {
                let index = (address - PRG_RAM_START).as_usize();
                self.prg_ram[index] = value;
            }
            ROM_START..=ROM_END => {
                // Allow mapper to handle writes (for mappers with registers like MMC1)
                self.rom.mapper.write(address, value);
            }
            _ => {
                debug!("Ignored attempt to write to address ${address:0X}");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::mappers::Nrom128;
    use crate::cartridge::{CHR_ROM_BANK_SIZE, MirroringType, PRG_ROM_BANK_SIZE};
    use assert_matches::assert_matches;

    fn test_bus() -> Bus {
        Bus::new(test_rom())
    }

    fn test_rom() -> Rom {
        Rom {
            prg_rom: vec![0x10.into(); PRG_ROM_BANK_SIZE],
            chr_rom: vec![0x20.into(); CHR_ROM_BANK_SIZE],
            mapper: Box::new(Nrom128 {}),
            screen_mirroring: MirroringType::Horizontal,
        }
    }

    #[test]
    fn write_to_ram() {
        let mut bus = test_bus();
        let address = Address::new(0x0012);
        let byte = 0xaa.into();
        bus.write_byte(address, byte)
            .expect("Failed to write to RAM");

        assert_eq!(bus.read_byte(address).unwrap(), byte);
    }

    #[test]
    fn write_to_ram_with_mirroring() {
        let mut bus = test_bus();
        let address = Address::new(0x1eff);
        let byte = 0xaa.into();
        bus.write_byte(address, byte)
            .expect("Failed to write to RAM");

        assert_eq!(bus.read_byte(address).unwrap(), byte);
        // 0x1eff truncated to 11 bits == 0x06ff
        assert_eq!(bus.read_byte(Address::new(0x06ff)).unwrap(), byte);
    }

    #[test]
    fn read_from_cartridge_rom() {
        let mut bus = test_bus();

        assert_eq!(bus.read_byte(Address::new(0x9000)).unwrap(), 0x10);
    }

    #[test]
    fn write_to_cartridge_rom_passes_to_mapper() {
        let mut bus = test_bus();

        // NROM doesn't have writable registers, but the write should succeed
        // (mapper's default write implementation does nothing)
        assert_matches!(bus.write_byte(Address::new(0x9000), 0xef.into()), Ok(()));
    }
}
