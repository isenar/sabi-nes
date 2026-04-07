use crate::apu::Apu;
use crate::cartridge::Rom;
use crate::cartridge::mappers::Mapper;
use crate::input::joypad::Joypad;
use crate::ppu::{NmiStatus, Ppu};
use crate::utils::MirroredAddress;
use crate::{Address, Byte, Memory, Result};
use derive_more::IsVariant;
use log::{debug, trace, warn};
use std::mem;
use std::ops::{Deref, DerefMut, Not};

#[derive(Debug, Default, PartialEq, Copy, Clone, IsVariant)]
pub enum DmaOperation {
    #[default]
    Get,
    Put,
}

impl Not for DmaOperation {
    type Output = DmaOperation;

    fn not(self) -> Self::Output {
        match self {
            Self::Get => Self::Put,
            Self::Put => Self::Get,
        }
    }
}

impl DmaOperation {
    pub const fn cycles(&self) -> usize {
        match self {
            Self::Get => 513,
            Self::Put => 514,
        }
    }

    pub const fn reset_delay(&self) -> Byte {
        match self {
            Self::Get => Byte::new(5),
            Self::Put => Byte::new(4),
        }
    }
}

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
    cycles: u64,
    frame_ready: bool,
    // Extra cycles to consume on the next tick, used for OAM DMA stall.
    pending_cycles: usize,
    // The CPU data bus retains the last value driven on it. Reads from unmapped
    // addresses return this value instead of driving the bus to zero.
    cpu_open_bus: Byte,
    dma_operation: DmaOperation,
}

impl Bus {
    pub fn new(rom: Rom) -> Bus {
        let ppu = Ppu::new(rom.screen_mirroring);

        Bus {
            cpu_vram: [Byte::default(); VRAM_SIZE],
            prg_ram: [Byte::default(); PRG_RAM_SIZE],
            rom,
            ppu,
            apu: Apu::default(),
            joypad: Joypad::default(),
            cycles: 0,
            frame_ready: false,
            pending_cycles: 0,
            cpu_open_bus: Byte::default(),
            dma_operation: DmaOperation::default(),
        }
    }

    /// Advance the emulator by exactly one CPU cycle and toggle cycle parity.
    pub fn tick_one(&mut self) -> Result<()> {
        let dma_operation = self.dma_operation;
        self.dma_operation = !self.dma_operation;
        self.cycles += 1;

        let nmi_before = self.ppu.nmi_status;
        let mapper = self.rom.mapper.deref();
        let nmi_after = self.ppu.tick(3, mapper);
        if let Some(dma_addr) = self.apu.tick_one(dma_operation) {
            debug_assert!(
                dma_addr >= 0x8000,
                "DMC sample address must be in PRG ROM range ($8000–$FFFF)"
            );
            let saved_open_bus = self.cpu_open_bus;
            let sample = self.read_byte(dma_addr)?;
            self.cpu_open_bus = saved_open_bus; // DMC DMA is not a CPU cycle; don't pollute the open-bus latch
            self.apu.dmc.deliver_sample(sample);
            // 4-cycle stall approximation; real hardware uses 3–4 cycles depending
            // on whether the CPU is in a read or write cycle.
            self.pending_cycles += 4;
        }

        if NmiStatus::activated(nmi_before, nmi_after) {
            self.frame_ready = true;
        }

        Ok(())
    }

    pub fn tick(&mut self, cycles: usize) -> Result<()> {
        let total = cycles + mem::take(&mut self.pending_cycles);
        for _ in 0..total {
            self.tick_one()?;
        }
        Ok(())
    }

    pub fn drain_audio_samples(&mut self) -> Vec<f32> {
        self.apu.drain_samples()
    }

    pub fn poll_irq_status(&self) -> bool {
        self.apu.is_irq_pending()
    }

    pub fn poll_nmi_status(&mut self) -> NmiStatus {
        let current = self.ppu.nmi_status;
        self.ppu.nmi_status = NmiStatus::Inactive;

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

    pub fn mapper(&self) -> &dyn Mapper {
        self.rom.mapper.deref()
    }

    pub fn joypad_mut(&mut self) -> &mut Joypad {
        &mut self.joypad
    }

    /// Read a byte without triggering any side effects. Used by the trace/debugger
    /// This method is mostly intended for tests and in the future, for debugger.
    pub fn peek_byte(&self, address: Address) -> Byte {
        match address.value() {
            RAM..=RAM_MIRRORS_END => {
                let mirror_base_addr = address.mirror_cpu_vram_addr().as_usize();
                self.cpu_vram[mirror_base_addr]
            }
            0x2000..=0x2007 => self.ppu.open_bus(),
            PPU_REGISTERS_MIRRORS_START..=PPU_REGISTERS_MIRRORS_END => {
                let mirror_base_addr = address.mirror_ppu_registers_addr();
                self.peek_byte(mirror_base_addr)
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
            0x400a => self.apu.triangle_channel.timer_low,
            0x400b => self.apu.triangle_channel.length_and_timer_high,
            0x400c => self.apu.noise_channel.volume,
            0x400e => self.apu.noise_channel.mode_and_period,
            0x400f => self.apu.noise_channel.len_counter_and_env_restart,
            0x4015 => self.apu.peek_status_register(),
            0x4016 | 0x4017 => Byte::new(0x00),
            PRG_RAM_START..=PRG_RAM_END => {
                let index = (address - PRG_RAM_START).as_usize();
                self.prg_ram[index]
            }
            ROM_START..=ROM_END => match self.rom.mapper.map_address(address - ROM_START) {
                Ok(mapped) => self.rom.prg_rom[mapped],
                Err(error) => {
                    warn!(
                        "Failed to map address {:04X}: `{error}`",
                        address - ROM_START
                    );
                    self.cpu_open_bus
                }
            },
            _ => self.cpu_open_bus,
        }
    }
}

impl Memory for Bus {
    fn read_byte(&mut self, address: Address) -> Result<Byte> {
        let value = match address.value() {
            RAM..=RAM_MIRRORS_END => {
                let mirror_base_addr = address.mirror_cpu_vram_addr().as_usize();
                self.cpu_vram[mirror_base_addr]
            }
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 => {
                // Write-only registers — nothing drives the bus, so the latch value decays back.
                debug!("Read from write-only PPU register ${address:04X}");
                self.ppu.open_bus()
            }
            0x2002 => {
                // Bits 7–5 come from the PPU status register; bits 4–0 come from the open bus.
                let status = self.ppu.read_status_register();
                let value = (status & 0xE0) | (self.ppu.open_bus() & 0x1F);
                self.ppu.write_to_open_bus(value);
                value
            }
            0x2004 => {
                let value = self.ppu.read_oam_data();
                self.ppu.write_to_open_bus(value);
                value
            }
            0x2007 => {
                let value = self.ppu.read(self.rom.mapper.deref())?;
                self.ppu.write_to_open_bus(value);
                value
            }
            PPU_REGISTERS_MIRRORS_START..=PPU_REGISTERS_MIRRORS_END => {
                let mirror_base_addr = address.mirror_ppu_registers_addr();
                self.read_byte(mirror_base_addr)?
            }
            // $4000–$400F are write-only APU registers; reads return open bus.
            // $4014 is write-only; the real 2A03 does not drive the data bus on reads,
            // so the open-bus value is returned (same as unmapped addresses).
            0x4014 => return Ok(self.cpu_open_bus),
            // $4015 is internal to the 2A03; its value doesn't drive the external data bus.
            // Bit 5 is not driven by the 2A03 at all, so it remains whatever was on the bus.
            0x4015 => return Ok(self.apu.read_status_register() | (self.cpu_open_bus & 0x20)),
            0x4016 => (self.joypad.read() & 0x1F) | (self.cpu_open_bus & 0xE0),
            // TODO: For reads, this is actually Player 2's controller, not frame counter!
            0x4017 => self.cpu_open_bus & 0xE0,
            PRG_RAM_START..=PRG_RAM_END => {
                let index = (address - PRG_RAM_START).as_usize();
                self.prg_ram[index]
            }
            ROM_START..=ROM_END => {
                let mapped_address = self.rom.mapper.map_address(address - ROM_START)?;
                self.rom.prg_rom[mapped_address]
            }
            _ => {
                trace!("Ignored attempt to read address ${address:0X}");
                return Ok(self.cpu_open_bus);
            }
        };
        self.cpu_open_bus = value;
        Ok(value)
    }

    fn write_byte(&mut self, address: Address, value: Byte) -> Result<()> {
        self.cpu_open_bus = value;
        match address.value() {
            RAM..=RAM_MIRRORS_END => {
                let mirror_base_addr: usize = address.mirror_cpu_vram_addr().into();
                self.cpu_vram[mirror_base_addr] = value;
            }
            0x2000 => {
                self.ppu.write_to_open_bus(value);
                self.ppu.write_to_control_register(value);
            }
            0x2001 => {
                self.ppu.write_to_open_bus(value);
                self.ppu.write_to_mask_register(value);
            }
            0x2002 => {
                self.ppu.write_to_open_bus(value);
                warn!("Attempted to write to PPU status register");
            }
            0x2003 => {
                self.ppu.write_to_open_bus(value);
                self.ppu.write_to_oam_address_register(value);
            }
            0x2004 => {
                self.ppu.write_to_open_bus(value);
                self.ppu.write_to_oam_data(value);
            }
            0x2005 => {
                self.ppu.write_to_open_bus(value);
                self.ppu.write_to_scroll_register(value);
            }
            0x2006 => {
                self.ppu.write_to_open_bus(value);
                self.ppu.write_to_addr_register(value);
            }
            0x2007 => {
                self.ppu.write_to_open_bus(value);
                self.ppu.write(value, self.rom.mapper.deref_mut())?;
            }
            PPU_REGISTERS_MIRRORS_START..=PPU_REGISTERS_MIRRORS_END => {
                let mirror_base_addr = address.mirror_ppu_registers_addr();
                self.write_byte(mirror_base_addr, value)?;
            }
            0x4000 => {
                self.apu.square_channel1.volume = value;
            }
            0x4001 => {
                self.apu.square_channel1.sweep = value;
                self.apu.square_channel1.on_sweep_write();
            }
            0x4002 => {
                self.apu.square_channel1.timer_low = value;
            }
            0x4003 => {
                self.apu.square_channel1.length_and_timer_high = value;
                self.apu.square_channel1.on_length_timer_write();
            }
            0x4004 => self.apu.square_channel2.volume = value,
            0x4005 => {
                self.apu.square_channel2.sweep = value;
                self.apu.square_channel2.on_sweep_write();
            }
            0x4006 => {
                self.apu.square_channel2.timer_low = value;
            }
            0x4007 => {
                self.apu.square_channel2.length_and_timer_high = value;
                self.apu.square_channel2.on_length_timer_write();
            }
            0x4008 => {
                self.apu.triangle_channel.linear_counter = value;
            }
            // 0x4009 is unused
            0x400a => {
                self.apu.triangle_channel.timer_low = value;
            }
            0x400b => {
                self.apu.triangle_channel.length_and_timer_high = value;
                self.apu.triangle_channel.on_length_timer_write();
            }
            0x400c => {
                self.apu.noise_channel.volume = value;
            }
            // 0x400d is unused
            0x400e => {
                self.apu.noise_channel.mode_and_period = value;
            }
            0x400f => {
                self.apu.noise_channel.len_counter_and_env_restart = value;
                self.apu.noise_channel.on_length_timer_write();
            }
            0x4010 => self.apu.dmc.write_flags_and_rate(value),
            0x4011 => self.apu.dmc.write_direct_load(value),
            0x4012 => self.apu.dmc.write_sample_address(value),
            0x4013 => self.apu.dmc.write_sample_length(value),
            0x4014 => {
                let mut buffer = [Byte::default(); 256];
                let high = value.as_address() << 8;

                for (offset, byte) in buffer.iter_mut().enumerate() {
                    let address = high + u16::try_from(offset)?;
                    *byte = self.read_byte(address)?;
                }

                self.ppu.write_to_oam_dma(&buffer);
                self.pending_cycles += self.dma_operation.cycles();
            }
            0x4015 => self.apu.set_status_register(value),
            0x4016 => self.joypad.write(value),
            0x4017 => self.apu.write_frame_counter(value, self.dma_operation),
            // 0x6000-0x7fff
            PRG_RAM_START..=PRG_RAM_END => {
                let index = (address - PRG_RAM_START).as_usize();
                self.prg_ram[index] = value;
            }
            // 0x8000-0xffff
            ROM_START..=ROM_END => {
                self.rom.mapper.write(address, value);
            }
            _ => {
                trace!("Ignored attempt to write to address ${address:0X}");
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
        Rom::new(
            vec![0x10.into(); PRG_ROM_BANK_SIZE],
            vec![0x20.into(); CHR_ROM_BANK_SIZE],
            Box::new(Nrom128::default()),
            MirroringType::Horizontal,
        )
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

    #[test]
    fn dmc_dma_stalls_cpu_by_4_cycles() {
        let mut bus = test_bus();
        // Exit APU open-bus mode and configure DMC:
        // Rate index 15 = period 54 (fastest trigger), sample at $C000 (default, maps to PRG ROM = 0x10)
        bus.write_byte(Address::new(0x4015), Byte::new(0x00))
            .unwrap(); // exit open-bus
        bus.write_byte(Address::new(0x4010), Byte::new(0x0F))
            .unwrap(); // rate 15
        bus.write_byte(Address::new(0x4012), Byte::new(0x00))
            .unwrap(); // sample addr = $C000
        bus.write_byte(Address::new(0x4013), Byte::new(0x00))
            .unwrap(); // length = 1 byte
        bus.write_byte(Address::new(0x4015), Byte::new(0x10))
            .unwrap(); // enable DMC

        let mut got_stall = false;
        for _ in 0..200 {
            let pending_before = bus.pending_cycles;
            bus.tick_one().unwrap();
            if bus.pending_cycles > pending_before {
                got_stall = true;
                assert_eq!(
                    bus.pending_cycles - pending_before,
                    4,
                    "DMC DMA should add exactly 4 pending cycles"
                );
                // Verify sample was actually delivered to the DMC
                assert_matches!(
                    bus.apu.dmc.sample_buffer(),
                    Some(_),
                    "sample should be in DMC buffer after DMA"
                );
                assert_eq!(
                    bus.apu.dmc.sample_buffer(),
                    Some(Byte::new(0x10)),
                    "sample should contain PRG ROM value (0x10)"
                );
                break;
            }
        }
        assert!(got_stall, "DMC DMA should have fired within 200 cycles");
    }
}
