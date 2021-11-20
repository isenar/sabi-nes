mod address;
mod control;
mod mask;
mod scroll;
mod status;

use crate::{Address, Byte};
pub use address::AddressRegister;
pub use control::ControlRegister;
pub use mask::MaskRegister;
pub use scroll::ScrollRegister;
pub use status::StatusRegister;

const OAM_DATA_SIZE: usize = 256;

#[derive(Debug)]
pub struct PpuRegisters {
    address: AddressRegister,
    control: ControlRegister,
    mask: MaskRegister,
    scroll: ScrollRegister,
    status: StatusRegister,
    oam_address: Byte,
    /// Internal memory to keep state of sprites (Object Attribute Memory)
    oam_data: [Byte; OAM_DATA_SIZE],
}

impl Default for PpuRegisters {
    fn default() -> Self {
        Self {
            address: AddressRegister::default(),
            control: ControlRegister::default(),
            mask: MaskRegister::default(),
            scroll: ScrollRegister::default(),
            status: StatusRegister::default(),
            oam_address: Byte::default(),
            oam_data: [0; OAM_DATA_SIZE],
        }
    }
}

impl PpuRegisters {
    pub fn read_address(&self) -> Address {
        self.address.get()
    }

    pub fn read_oam_data(&self) -> Byte {
        self.oam_data[self.oam_address as usize]
    }

    pub fn read_status(&self) -> Byte {
        self.status.bits()
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
        self.oam_address = value;
    }

    pub fn write_oam_data(&mut self, value: Byte) {
        self.oam_data[self.oam_address as usize] = value;
        self.oam_address = self.oam_address.wrapping_add(1);
    }

    pub fn write_scroll(&mut self, value: Byte) {
        self.scroll.write(value);
    }

    pub fn increment_vram_address(&mut self) {
        self.address.increment(self.control.vram_addr_increment())
    }
}
