use crate::Byte;
use bitflags::bitflags;

bitflags! {
    #[derive(Default, Debug)]
    pub struct ApuFlags: Byte {
        const SQUARE_CHANNEL_1_ENABLED = 0b0000_0001;
        const SQUARE_CHANNEL_2_ENABLED = 0b0000_0010;
        const TRIANGLE_CHANNEL_ENABLED = 0b0000_0100;
        const NOISE_CHANNEL_ENABLED    = 0b0000_1000;
        const DMC_ENABLED              = 0b0001_0000;
        const UNUSED1                  = 0b0010_0000;
        const UNUSED2                  = 0b0100_0000;
        const UNUSED3                  = 0b1000_0000;
    }
}

impl From<Byte> for ApuFlags {
    fn from(byte: Byte) -> Self {
        Self::from_bits_truncate(byte)
    }
}
