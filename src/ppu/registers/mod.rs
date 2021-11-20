mod address;
mod control;
mod mask;
mod scroll;
mod status;

use crate::Byte;
pub use address::AddressRegister;
pub use control::ControlRegister;
pub use mask::MaskRegister;
pub use scroll::ScrollRegister;
pub use status::StatusRegister;

const OAM_DATA_SIZE: usize = 256;

#[derive(Debug)]
pub struct PpuRegisters {
    pub address: AddressRegister,
    pub control: ControlRegister,
    pub mask: MaskRegister,
    pub scroll: ScrollRegister,
    pub status: StatusRegister,
    pub oam_address: Byte,
    /// Internal memory to keep state of sprites (Object Attribute Memory)
    pub oam_data: [Byte; OAM_DATA_SIZE],
}

impl Default for PpuRegisters {
    fn default() -> Self {
        Self {
            address: Default::default(),
            control: Default::default(),
            mask: Default::default(),
            scroll: Default::default(),
            status: Default::default(),
            oam_address: Default::default(),
            oam_data: [0; OAM_DATA_SIZE],
        }
    }
}
