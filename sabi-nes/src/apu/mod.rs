use crate::apu::apu_flags::ApuFlags;
use crate::apu::channels::dmc::Dmc;
use crate::apu::channels::noise_channel::NoiseChannel;
use crate::apu::channels::square_channel::SquareChannel;
use crate::apu::channels::triangle_channel::TriangleChannel;
use crate::apu::frame_counter::FrameCounter;
use crate::Byte;

mod apu_flags;
mod channels;
mod frame_counter;

#[derive(Debug, Default)]
pub struct Apu {
    pub flags: ApuFlags,
    pub square_channel1: SquareChannel,
    pub square_channel2: SquareChannel,
    pub triangle_channel: TriangleChannel,
    pub noise_channel: NoiseChannel,
    pub dmc: Dmc,
    pub frame_counter: FrameCounter,
}

impl Apu {
    pub fn set_status_register(&mut self, byte: Byte) {
        self.flags = ApuFlags::from(byte);
    }

    pub fn status_register(&self) -> Byte {
        self.flags.bits()
    }
}
