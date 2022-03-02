mod address;
mod control;
mod mask;
mod oam;
mod scroll;
mod status;

use crate::ppu::registers::oam::Oam;
use crate::{Address, Byte};
pub use address::AddressRegister;
pub use control::ControlRegister;
pub use mask::MaskRegister;
pub use scroll::ScrollRegister;
pub use status::StatusRegister;

#[derive(Debug, Default)]
pub struct PpuRegisters {
    address: AddressRegister,
    pub control: ControlRegister,
    mask: MaskRegister,
    pub scroll: ScrollRegister,
    status: StatusRegister,
    oam: Oam,
}

impl PpuRegisters {
    pub fn read_address(&self) -> Address {
        self.address.get()
    }

    pub fn read_oam_data(&self) -> Byte {
        self.oam.read()
    }

    pub fn read_oam_dma(&self) -> &[Byte] {
        self.oam.read_all()
    }

    pub fn read_status(&mut self) -> Byte {
        let status = self.status.bits();

        self.status.reset_vblank();
        self.address.reset_latch();
        self.scroll.reset_latch();

        status
    }

    pub fn read_sprite_pattern_address(&self) -> Address {
        self.control.sprite_pattern_address()
    }

    pub fn write_address(&mut self, value: Byte) {
        self.address.update(value);
    }

    pub fn write_control(&mut self, value: Byte) {
        self.control.update(value);
    }

    pub fn write_mask(&mut self, value: Byte) {
        self.mask.update(value);
    }

    pub fn write_oam_address(&mut self, value: Byte) {
        self.oam.write_address(value);
    }

    pub fn write_oam_data(&mut self, value: Byte) {
        self.oam.write(value);
    }

    pub fn write_oam_dma(&mut self, buffer: &[Byte]) {
        self.oam.write_all(buffer);
    }

    pub fn write_scroll(&mut self, value: Byte) {
        self.scroll.write(value);
    }

    pub fn set_vblank(&mut self) -> &mut Self {
        self.status.vblank_started();
        self
    }

    pub fn reset_vblank(&mut self) -> &mut Self {
        self.status.remove(StatusRegister::VBLANK_STARTED);
        self
    }

    pub fn set_sprite_zero_hit(&mut self) -> &mut Self {
        self.status.insert(StatusRegister::SPRITE_ZERO_HIT);
        self
    }

    pub fn reset_sprite_zero_hit(&mut self) -> &mut Self {
        self.status.remove(StatusRegister::SPRITE_ZERO_HIT);
        self
    }

    pub fn is_generating_nmi(&self) -> bool {
        self.control.contains(ControlRegister::GENERATE_NMI)
    }

    pub fn is_in_vblank(&self) -> bool {
        self.status.contains(StatusRegister::VBLANK_STARTED)
    }

    pub fn background_pattern_address(&self) -> Address {
        self.control
            .contains(ControlRegister::BACKROUND_PATTERN_ADDR) as Address
            * 0x1000
    }

    pub fn increment_vram_address(&mut self) {
        self.address.increment(self.control.vram_addr_increment())
    }

    pub fn show_sprites(&self) -> bool {
        self.mask.show_sprites()
    }

    pub fn show_background(&self) -> bool {
        self.mask.show_background()
    }
}
