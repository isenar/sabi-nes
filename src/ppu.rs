mod nmi_status;
mod open_bus;
mod registers;

pub use nmi_status::NmiStatus;
pub use registers::{SpriteData, SpriteSize};

use crate::cartridge::MirroringType;
use crate::cartridge::mappers::Mapper;
use crate::ppu::open_bus::OpenBus;
use crate::ppu::registers::PpuRegisters;
use crate::utils::MirroredAddress;
use crate::{Address, Byte, Result};
use anyhow::bail;

const VRAM_SIZE: usize = 2048;
const PALETTE_TABLE_SIZE: usize = 64;
const MIRRORS: [Address; 4] = [
    Address::new(0x3f10),
    Address::new(0x3f14),
    Address::new(0x3f18),
    Address::new(0x3f1c),
];

#[derive(Debug)]
pub struct Ppu {
    /// Internal memory to keep palette tables used by the screen
    pub palette_table: [Byte; PALETTE_TABLE_SIZE],
    /// 2KiB of space to hold background information
    pub vram: [Byte; VRAM_SIZE],

    /// Mirroring type
    pub mirroring: MirroringType,

    /// PPU registers
    pub registers: PpuRegisters,

    pub scanline: usize,
    pub cycles: usize,
    pub nmi_status: NmiStatus,

    internal_data_buffer: Byte,

    /// Whether the BG shift registers were loaded at the start of the current
    /// scanline (i.e. rendering was active during the h-blank reload window of
    /// the previous scanline, dots 321-336).  If false, the background appears
    /// transparent for sprite-zero-hit purposes even if rendering is later
    /// re-enabled mid-scanline.
    bg_shift_regs_loaded: bool,

    open_bus: OpenBus,
    // Monotonically increasing PPU cycle counter, used for open bus decay.
    // We cannot use `self.cycles` as it resets per scanline.
    total_cycles: usize,
}

impl Ppu {
    pub fn new(mirroring: MirroringType) -> Self {
        Self {
            palette_table: [Byte::default(); PALETTE_TABLE_SIZE],
            vram: [Byte::default(); VRAM_SIZE],
            mirroring,
            registers: PpuRegisters::default(),
            cycles: 0,
            scanline: 0,
            nmi_status: NmiStatus::Inactive,
            internal_data_buffer: Byte::default(),
            bg_shift_regs_loaded: false,
            open_bus: OpenBus::new(),
            total_cycles: 0,
        }
    }

    /// Returns the PPU open bus value, or 0 if it has fully decayed.
    /// Real hardware capacitors discharge over ~600 ms; we model the full
    /// byte as decayed after roughly one second of PPU cycles.
    pub fn open_bus(&self) -> Byte {
        self.open_bus.read(self.total_cycles)
    }

    /// Update the PPU open bus latch and reset its decay timer.
    pub fn write_to_open_bus(&mut self, value: Byte) {
        self.open_bus.write(value, self.total_cycles);
    }

    pub fn tick(&mut self, cycles: usize, mapper: &dyn Mapper) -> NmiStatus {
        self.cycles += cycles;
        self.total_cycles += cycles;

        // Sprite zero hit fires at the specific PPU cycle within the scanline (X+1),
        // not at the end of the scanline, so we check continuously here.
        if !self.registers.is_sprite_zero_hit_set() && self.is_sprite_zero_hit(mapper) {
            self.registers.set_sprite_zero_hit();
        }

        if self.cycles >= 341 {
            if self.is_sprite_overflow() {
                self.registers.set_sprite_overflow();
            }

            // Record whether rendering was active during the h-blank reload
            // window (end of this scanline).  The next scanline's BG shift
            // registers are only populated if rendering is active here.
            self.bg_shift_regs_loaded = self.registers.is_rendering_active();

            self.cycles -= 341;
            self.scanline += 1;

            // On real hardware, OAMADDR is reset to 0 during dots 257-320 of
            // every visible scanline (and the pre-render line) when rendering
            // is enabled.  Approximate this by resetting it at each visible
            // scanline boundary so that OAM DMA performed in vblank always
            // writes to OAM starting at address 0, matching hardware behaviour.
            if self.scanline <= 240 && self.registers.is_rendering_active() {
                self.registers.reset_oam_address();
            }

            if self.scanline == 241 {
                self.registers.set_vblank().reset_sprite_overflow();
                if self.registers.is_generating_nmi() {
                    self.nmi_status = NmiStatus::Active;
                }
            }

            if self.scanline >= 262 {
                self.scanline = 0;
                self.nmi_status = NmiStatus::Inactive;
                self.registers
                    .reset_vblank()
                    .reset_sprite_zero_hit()
                    .reset_sprite_overflow();
            }
        }

        self.nmi_status
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
            self.nmi_status = NmiStatus::Active;
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

    pub fn write(&mut self, value: Byte, mapper: &mut dyn Mapper) -> Result<()> {
        let addr = self.registers.read_address();

        match addr.value() {
            0x0000..=0x1fff => {
                mapper.write_chr(addr, value);
            }
            0x2000..=0x2fff => {
                let mirrorred = self.mirror_vram_addr(addr);
                self.vram[mirrorred.as_usize()] = value;
            }
            0x3000..=0x3eff => bail!("Requested invalid address from PPU ({addr:#x})"),
            0x3f00..=0x3fff => {
                let mut addr = addr;
                // "Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C"
                if MIRRORS.contains(&addr) {
                    addr = addr - 0x10;
                }

                let offset_address = (addr - 0x3F00) & 0x1F;
                // Palette RAM is 6-bit; the upper 2 bits are not stored.
                self.palette_table[offset_address.as_usize()] = value & 0x3F;
            }
            0x4000.. => bail!("Unexpected access to mirrored space on PPU write ({addr:#x})"),
        }

        self.increment_vram_address();

        Ok(())
    }

    pub fn read(&mut self, mapper: &dyn Mapper) -> Result<Byte> {
        let addr = self.registers.read_address();
        self.increment_vram_address();

        match addr.value() {
            0x0000..=0x1fff => {
                let result = self.internal_data_buffer;
                self.internal_data_buffer = mapper.read_chr(addr);

                Ok(result)
            }
            0x2000..=0x2fff => {
                let result = self.internal_data_buffer;
                let mirrored = self.mirror_vram_addr(addr);
                self.internal_data_buffer = self.vram[mirrored.as_usize()];

                Ok(result)
            }
            0x3000..=0x3eff => bail!("Requested invalid address from PPU ({addr:#x})"),
            0x3f00..=0x3fff => {
                let mut addr = addr;
                // "Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C"
                if MIRRORS.contains(&addr) {
                    addr = addr - 0x10;
                }

                // Palette reads return data directly (no buffering), but the read
                // buffer is loaded with nametable data from the mirrored address
                // at $2F00–$2FFF (addr - $1000).
                let nametable_addr = addr - 0x1000;
                let mirrored = self.mirror_vram_addr(nametable_addr);
                self.internal_data_buffer = self.vram[mirrored.as_usize()];

                let offset_address = (addr - 0x3f00).value() & 0x1F;
                // Palette RAM is 6-bit; upper 2 bits come from the PPU open bus.
                // In greyscale mode the lower 4 bits are forced to zero.
                let palette_data = self.palette_table[offset_address as usize];
                let colour_bits = if self.registers.is_greyscale() {
                    palette_data & 0x30
                } else {
                    palette_data & 0x3F
                };
                Ok((self.open_bus() & 0xC0) | colour_bits)
            }
            0x4000.. => bail!("Unexpected access to mirrored space on PPU read ({addr:#x})"),
        }
    }

    pub fn mirror_vram_addr(&self, addr: Address) -> Address {
        let mirrored_vram_addr = addr.mirror_ppu_addr();
        let vram_index = mirrored_vram_addr - 0x2000;
        let name_table = vram_index / 0x0400;

        let offset = match (self.mirroring, name_table.value()) {
            (MirroringType::Vertical, 2 | 3) | (MirroringType::Horizontal, 3) => 0x800,
            (MirroringType::Horizontal, 1 | 2) => 0x400,
            _ => 0x000,
        };

        vram_index - offset
    }

    fn is_sprite_zero_hit(&self, mapper: &dyn Mapper) -> bool {
        let oam_data = self.registers.read_oam_dma();
        let sprite = oam_data[0];
        let y = sprite.y.as_usize();
        let x = sprite.x.as_usize();

        let sprite_height = self.registers.sprite_size().height();
        if self.scanline >= 240 {
            return false;
        }

        // NES OAM Y = screen_Y − 1: sprite appears on scanlines y+1 through y+sprite_height
        if !(self.scanline > y && self.scanline <= y + sprite_height) {
            return false;
        }

        if !(self.cycles > x
            && x != 255
            && self.registers.show_sprites()
            && self.registers.show_background()
            && self.bg_shift_regs_loaded)
        {
            return false;
        }

        // No hit at X=0 if either left-column mask hides pixels there
        if x == 0
            && (!self.registers.show_sprites_left_column()
                || !self.registers.show_background_left_column())
        {
            return false;
        }

        // Per-pixel overlap check on this scanline row
        let sprite_row = self.scanline - (y + 1);

        let row_in_tile = if sprite.flip_vertically() {
            (sprite_height - 1) - sprite_row
        } else {
            sprite_row
        };

        let sprite_pattern_base = self.registers.read_sprite_pattern_address().value() as usize;
        let tile_base = sprite_pattern_base + sprite.index_number.as_usize() * 16;

        let sprite_plane1 = mapper
            .read_chr(Address::new((tile_base + row_in_tile) as u16))
            .value();
        let sprite_plane2 = mapper
            .read_chr(Address::new((tile_base + row_in_tile + 8) as u16))
            .value();

        for sprite_col in 0..8usize {
            let screen_x = x + sprite_col;
            if screen_x >= 256 {
                break;
            }

            // Per-pixel left column mask
            if screen_x < 8
                && (!self.registers.show_sprites_left_column()
                    || !self.registers.show_background_left_column())
            {
                continue;
            }

            // Sprite pixel opacity (MSB = leftmost pixel; flip reverses column order)
            let bit = if sprite.flip_horizontally() {
                sprite_col
            } else {
                7 - sprite_col
            };
            let sprite_opaque = ((sprite_plane1 >> bit) | (sprite_plane2 >> bit)) & 1 != 0;
            if !sprite_opaque {
                continue;
            }

            if self.is_background_pixel_opaque(screen_x, self.scanline, mapper) {
                return true;
            }
        }

        false
    }

    fn is_background_pixel_opaque(
        &self,
        screen_x: usize,
        scanline: usize,
        mapper: &dyn Mapper,
    ) -> bool {
        let scroll_x = self.registers.read_scroll_x().as_usize();
        let scroll_y = self.registers.read_scroll_y().as_usize();

        let eff_x = (screen_x + scroll_x) % 512;
        let eff_y = (scanline + scroll_y) % 480;

        // Select one of the four nametables based on which 256x240 quadrant we land in
        let nt_id = (eff_y / 240) * 2 + (eff_x / 256);
        let nt_base_addr = 0x2000u16 + (nt_id as u16) * 0x400;

        let local_x = eff_x % 256;
        let local_y = eff_y % 240;
        let tile_idx = (local_y / 8) * 32 + (local_x / 8);

        let nt_addr = Address::new(nt_base_addr + tile_idx as u16);
        let mirrored = self.mirror_vram_addr(nt_addr);
        let tile_index = self.vram[mirrored.as_usize()].as_usize();

        let bg_pattern_base = self.registers.background_pattern_address().value() as usize;
        let tile_base = bg_pattern_base + tile_index * 16;
        let pixel_row = eff_y % 8;
        let pixel_col = eff_x % 8;

        let plane1 = mapper
            .read_chr(Address::new((tile_base + pixel_row) as u16))
            .value();
        let plane2 = mapper
            .read_chr(Address::new((tile_base + pixel_row + 8) as u16))
            .value();

        let bit = 7 - pixel_col;
        ((plane1 >> bit) | (plane2 >> bit)) & 1 != 0
    }

    fn is_sprite_overflow(&self) -> bool {
        if !self.registers.is_rendering_active() {
            return false;
        }
        let sprite_height: usize = match self.registers.sprite_size() {
            SpriteSize::Small => 8,
            SpriteSize::Large => 16,
        };
        self.registers
            .read_oam_dma()
            .iter()
            .filter(|sprite| {
                let y = sprite.y.as_usize();
                self.scanline > y && self.scanline <= y + sprite_height
            })
            .count()
            > 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::mappers::Mapper;

    struct NullMapper;

    impl Mapper for NullMapper {
        fn map_address(&self, _: Address) -> Result<usize> {
            Ok(0)
        }
        fn write(&mut self, _: Address, _: Byte) {}
        fn load_chr(&mut self, _: Vec<Byte>) {}
        fn read_chr(&self, _: Address) -> Byte {
            Byte::default()
        }
        fn write_chr(&mut self, _: Address, _: Byte) {}
    }

    impl Ppu {
        fn test_ppu() -> Self {
            Self::new(MirroringType::Horizontal)
        }
    }

    #[test]
    fn ppu_vram_writes() {
        let mut ppu = Ppu::test_ppu();
        ppu.write_to_addr_register(0x23.into());
        ppu.write_to_addr_register(0x05.into());
        ppu.write(0x66.into(), &mut NullMapper)
            .expect("Failed to write");

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn ppu_vram_reads() {
        let mut ppu = Ppu::test_ppu();
        ppu.write_to_control_register(0x00.into());
        ppu.vram[0x0305] = 0x66.into();

        ppu.write_to_addr_register(0x23.into());
        ppu.write_to_addr_register(0x05.into());

        ppu.read(&NullMapper).expect("Failed to perform dummy read");

        assert_eq!(ppu.registers.read_address(), 0x2306);
        assert_eq!(ppu.read(&NullMapper).unwrap(), 0x66);
    }

    #[test]
    fn ppu_vram_reads_with_step_32() {
        let mut ppu = Ppu::test_ppu();

        ppu.write_to_control_register(0b0100.into());
        ppu.vram[0x01ff] = 0x66.into();
        ppu.vram[0x01ff + 32] = 0x77.into();
        ppu.vram[0x01ff + 64] = 0x88.into();

        ppu.registers.write_address(0x21.into());
        ppu.registers.write_address(0xff.into());

        ppu.read(&NullMapper).expect("Failed to perform dummy read");

        assert_eq!(ppu.read(&NullMapper).unwrap(), 0x66);
        assert_eq!(ppu.read(&NullMapper).unwrap(), 0x77);
        assert_eq!(ppu.read(&NullMapper).unwrap(), 0x88);
    }

    #[test]
    fn vram_horizontal_mirror() {
        let mut ppu = Ppu::test_ppu();

        ppu.registers.write_address(0x24.into());
        ppu.registers.write_address(0x05.into());

        ppu.write(0x66.into(), &mut NullMapper).unwrap();

        ppu.registers.write_address(0x28.into());
        ppu.registers.write_address(0x05.into());

        ppu.write(0x77.into(), &mut NullMapper).unwrap();

        ppu.registers.write_address(0x20.into());
        ppu.registers.write_address(0x05.into());

        ppu.read(&NullMapper).unwrap();
        assert_eq!(ppu.read(&NullMapper).unwrap(), 0x66);

        ppu.registers.write_address(0x2c.into());
        ppu.registers.write_address(0x05.into());

        ppu.read(&NullMapper).unwrap();
        assert_eq!(ppu.read(&NullMapper).unwrap(), 0x77);
    }

    #[test]
    fn vram_vertical_mirror() {
        let mut ppu = Ppu::test_ppu();
        ppu.mirroring = MirroringType::Vertical;

        ppu.registers.write_address(0x20.into());
        ppu.registers.write_address(0x05.into());

        ppu.write(0x66.into(), &mut NullMapper).unwrap();

        ppu.registers.write_address(0x2c.into());
        ppu.registers.write_address(0x05.into());

        ppu.write(0x77.into(), &mut NullMapper).unwrap();

        ppu.registers.write_address(0x28.into());
        ppu.registers.write_address(0x05.into());

        ppu.read(&NullMapper).unwrap();
        assert_eq!(ppu.read(&NullMapper).unwrap(), 0x66);

        ppu.registers.write_address(0x24.into());
        ppu.registers.write_address(0x05.into());

        ppu.read(&NullMapper).unwrap();
        assert_eq!(ppu.read(&NullMapper).unwrap(), 0x77);
    }

    #[test]
    fn reading_status_resets_latch() {
        let mut ppu = Ppu::test_ppu();
        ppu.vram[0x0305] = 0x66.into();

        ppu.registers.write_address(0x21.into());
        ppu.registers.write_address(0x23.into());
        ppu.registers.write_address(0x05.into());

        ppu.read(&NullMapper).unwrap();
        assert_ne!(ppu.read(&NullMapper).unwrap(), 0x66);

        ppu.read_status_register();

        ppu.registers.write_address(0x23.into());
        ppu.registers.write_address(0x05.into());

        ppu.read(&NullMapper).unwrap();
        assert_eq!(ppu.read(&NullMapper).unwrap(), 0x66);
    }

    #[test]
    fn vram_mirroring() {
        let mut ppu = Ppu::test_ppu();
        ppu.write_to_control_register(0x00.into());
        ppu.vram[0x0305] = 0x66.into();

        ppu.registers.write_address(0x63.into());
        ppu.registers.write_address(0x05.into());

        ppu.read(&NullMapper).unwrap();
        assert_eq!(ppu.read(&NullMapper).unwrap(), 0x66);
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
        ppu.write_to_oam_address_register(0x10.into());
        ppu.write_to_oam_data(0x66.into());
        ppu.write_to_oam_data(0x77.into());

        ppu.write_to_oam_address_register(0x10.into());
        assert_eq!(ppu.read_oam_data(), 0x66);

        ppu.write_to_oam_address_register(0x11.into());
        assert_eq!(ppu.read_oam_data(), 0x77);
    }
}
