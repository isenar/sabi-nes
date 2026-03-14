use crate::Byte;
use crate::apu::apu_flags::ApuFlags;
use crate::apu::channels::dmc::Dmc;
use crate::apu::channels::noise_channel::NoiseChannel;
use crate::apu::channels::square_channel::SquareChannel;
use crate::apu::channels::triangle_channel::TriangleChannel;
use crate::apu::frame_counter::{FrameCounter, FrameSignal};

mod apu_flags;
mod channels;
mod frame_counter;

// NES CPU runs at ~1.789773 MHz. We output at 44.1 kHz.
// Every ~40.58 CPU cycles we emit one sample.
const CPU_CLOCK: f64 = 1_789_773.0;
const SAMPLE_RATE: f64 = 44_100.0;
const CYCLES_PER_SAMPLE: f64 = CPU_CLOCK / SAMPLE_RATE;

#[derive(Debug, Default)]
pub struct Apu {
    pub flags: ApuFlags,
    pub square_channel1: SquareChannel,
    pub square_channel2: SquareChannel,
    pub triangle_channel: TriangleChannel,
    pub noise_channel: NoiseChannel,
    pub dmc: Dmc,
    pub frame_counter: FrameCounter,

    // Audio synthesis state
    cycle_accumulator: f64,
    samples: Vec<f32>,
}

impl Apu {
    pub fn set_status_register(&mut self, byte: Byte) {
        self.flags = ApuFlags::from(byte);
        self.square_channel1
            .set_enabled(self.flags.contains(ApuFlags::SQUARE_CHANNEL_1_ENABLED));
        self.square_channel2
            .set_enabled(self.flags.contains(ApuFlags::SQUARE_CHANNEL_2_ENABLED));
    }

    pub fn status_register(&self) -> Byte {
        self.flags.bits().into()
    }

    /// Advance the APU by `cycles` CPU cycles, accumulating audio samples.
    pub fn tick(&mut self, cycles: usize) {
        for _ in 0..cycles {
            self.square_channel1.tick();
            self.square_channel2.tick();

            match self.frame_counter.tick() {
                FrameSignal::QuarterFrame => {
                    self.square_channel1.clock_envelope();
                    self.square_channel2.clock_envelope();
                }
                FrameSignal::HalfFrame => {
                    self.square_channel1.clock_envelope();
                    self.square_channel2.clock_envelope();
                    self.square_channel1.clock_length_counter();
                    self.square_channel2.clock_length_counter();
                }
                FrameSignal::None => {}
            }

            self.cycle_accumulator += 1.0;
            if self.cycle_accumulator >= CYCLES_PER_SAMPLE {
                self.cycle_accumulator -= CYCLES_PER_SAMPLE;
                self.samples.push(self.mix());
            }
        }
    }

    /// Take all accumulated samples since the last call, leaving the buffer empty.
    pub fn drain_samples(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.samples)
    }

    // NES square-channel mixing approximation from the nesdev wiki.
    // Output is in roughly -1.0..1.0.
    fn mix(&self) -> f32 {
        let sq1 = f32::from(self.square_channel1.output());
        let sq2 = f32::from(self.square_channel2.output());
        let sq_sum = sq1 + sq2;
        if sq_sum == 0.0 {
            return 0.0;
        }
        95.88 / (8128.0 / sq_sum + 100.0)
    }
}
