use crate::apu::apu_flags::ApuFlags;
use crate::apu::channels::dmc::Dmc;
use crate::apu::channels::noise_channel::NoiseChannel;
use crate::apu::channels::square_channel::SquareChannel;
use crate::apu::channels::triangle_channel::TriangleChannel;
use crate::apu::frame_counter::{FrameCounter, FrameSignal};
use crate::bus::DmaOperation;
use crate::{Address, Byte};
use once_cell::sync::Lazy;
use std::mem;

mod apu_flags;
mod channels;
mod frame_counter;

// NES CPU runs at ~1.789773 MHz. We output at 44.1 kHz.
// Every ~40.58 CPU cycles we emit one sample.
const CPU_CLOCK: f32 = 1_789_773.0;
const SAMPLE_RATE: f32 = 44_100.0;
const CYCLES_PER_SAMPLE: f32 = CPU_CLOCK / SAMPLE_RATE;

// First-order IIR filter coefficients derived from the real NES hardware.
// All use the matched-z formula: α = exp(-2π * fc / Fs).
//
// High-pass filters remove DC offset (two cascaded capacitors on the NES board):
//   HP1 ~90 Hz - slow drift removal
//   HP2 ~440 Hz - faster drift removal
// Low-pass filter softens high-frequency aliasing from the square/noise waveforms:
//   LP ~14 kHz
static SLOW_DRIFT_REMOVAL_COEFFICIENT: Lazy<f32> = Lazy::new(|| coefficient(90.0).exp());
static FAST_DRIFT_REMOVAL_COEFFICIENT: Lazy<f32> = Lazy::new(|| coefficient(440.0).exp());
static LOW_PASS_COEFFICIENT: Lazy<f32> = Lazy::new(|| coefficient(14000.0).exp());

const fn coefficient(hz: f32) -> f32 {
    -2.0 * std::f32::consts::PI * hz / SAMPLE_RATE
}

/// Three cascaded first-order IIR filters matching the NES analog output stage.
///
/// Two high-pass filters eliminate the DC offset that arises when channels
/// hold a constant value (e.g. the triangle sequencer freezing on note-off).
/// The low-pass filter rolls off aliasing above 14 kHz.
#[derive(Debug, Default, Clone, Copy)]
struct AudioFilter {
    hp1_prev_in: f32,
    hp1_prev_out: f32,
    hp2_prev_in: f32,
    hp2_prev_out: f32,
    lp_prev_out: f32,
}

impl AudioFilter {
    fn filter(&mut self, input: f32) -> f32 {
        // High-pass 1: y[n] = α * (y[n-1] + x[n] - x[n-1])
        let high_pass1 =
            *SLOW_DRIFT_REMOVAL_COEFFICIENT * (self.hp1_prev_out + input - self.hp1_prev_in);
        self.hp1_prev_in = input;
        self.hp1_prev_out = high_pass1;

        // High-pass 2: same formula chained onto hp1 output
        let high_pass2 =
            *FAST_DRIFT_REMOVAL_COEFFICIENT * (self.hp2_prev_out + high_pass1 - self.hp2_prev_in);
        self.hp2_prev_in = high_pass1;
        self.hp2_prev_out = high_pass2;

        // Low-pass: y[n] = (1 - α) * x[n] + α * y[n-1]
        let low_pass =
            (1.0 - *LOW_PASS_COEFFICIENT) * high_pass2 + *LOW_PASS_COEFFICIENT * self.lp_prev_out;
        self.lp_prev_out = low_pass;

        low_pass
    }
}

#[derive(Debug)]
pub struct Apu {
    pub flags: ApuFlags,
    pub square_channel1: SquareChannel,
    pub square_channel2: SquareChannel,
    pub triangle_channel: TriangleChannel,
    pub noise_channel: NoiseChannel,
    pub dmc: Dmc,
    pub frame_counter: FrameCounter,

    // Audio synthesis state
    cycle_accumulator: f32,
    samples: Vec<f32>,
    filter: AudioFilter,

    // True until $4015 is first written; returns open-bus 0xFF on reads until then.
    status_open_bus: bool,
}

impl Default for Apu {
    fn default() -> Self {
        Self {
            flags: ApuFlags::default(),
            square_channel1: SquareChannel::channel1(),
            square_channel2: SquareChannel::default(),
            triangle_channel: TriangleChannel::default(),
            noise_channel: NoiseChannel::default(),
            dmc: Dmc::default(),
            frame_counter: FrameCounter::default(),
            cycle_accumulator: 0.0,
            samples: Vec::new(),
            filter: AudioFilter::default(),
            status_open_bus: true,
        }
    }
}

impl Apu {
    pub fn write_frame_counter(&mut self, value: Byte, dma_operation: DmaOperation) {
        if let Some(signal) = self.frame_counter.write(value, dma_operation) {
            self.dispatch_frame_signal(signal);
        }
    }

    pub fn is_irq_pending(&self) -> bool {
        self.frame_counter.is_irq_pending() || self.dmc.irq_pending
    }

    pub fn set_status_register(&mut self, byte: Byte) {
        self.status_open_bus = false;
        self.flags = ApuFlags::from(byte);
        self.square_channel1
            .set_enabled(self.flags.contains(ApuFlags::SQUARE_CHANNEL_1_ENABLED));
        self.square_channel2
            .set_enabled(self.flags.contains(ApuFlags::SQUARE_CHANNEL_2_ENABLED));
        self.triangle_channel
            .set_enabled(self.flags.contains(ApuFlags::TRIANGLE_CHANNEL_ENABLED));
        self.noise_channel
            .set_enabled(self.flags.contains(ApuFlags::NOISE_CHANNEL_ENABLED));
        let dmc_enabled = self.flags.contains(ApuFlags::DMC_ENABLED);
        if !dmc_enabled {
            self.dmc.irq_pending = false;
        }
        self.dmc.set_enabled(dmc_enabled);
    }

    /// Read $4015: returns length counter and IRQ status, then clears the frame IRQ flag.
    /// Bit 0: square 1 active,
    /// Bit 1: square 2 active,
    /// Bit 2: triangle active,
    /// Bit 3: noise active,
    /// Bit 4: DMC active,
    /// Bit 6: frame counter IRQ pending.
    pub fn read_status_register(&mut self) -> Byte {
        if self.status_open_bus {
            return Byte::new(0xFF);
        }
        let status = self.peek_status_register();
        self.frame_counter.clear_irq();
        self.dmc.irq_pending = false;

        status
    }

    /// Read APU status without side effects (does not clear the frame counter IRQ flag).
    pub fn peek_status_register(&self) -> Byte {
        if self.status_open_bus {
            return Byte::new(0xFF);
        }
        let mut status = Byte::new(0x00);
        if self.square_channel1.is_active() {
            status |= 0x01;
        }
        if self.square_channel2.is_active() {
            status |= 0x02;
        }
        if self.triangle_channel.is_active() {
            status |= 0x04;
        }
        if self.noise_channel.is_active() {
            status |= 0x08;
        }
        if self.dmc.is_active() {
            status |= 0x10;
        }
        if self.frame_counter.is_irq_pending() {
            status |= 0x40;
        }
        if self.dmc.irq_pending {
            status |= 0x80;
        }

        status
    }

    fn dispatch_frame_signal(&mut self, signal: FrameSignal) {
        match signal {
            FrameSignal::QuarterFrame => {
                self.square_channel1.clock_envelope();
                self.square_channel2.clock_envelope();
                self.noise_channel.clock_envelope();
                self.triangle_channel.clock_linear_counter();
            }
            FrameSignal::HalfFrame => {
                self.square_channel1.clock_envelope();
                self.square_channel2.clock_envelope();
                self.noise_channel.clock_envelope();
                self.triangle_channel.clock_linear_counter();
                self.square_channel1.clock_length_counter();
                self.square_channel2.clock_length_counter();
                self.triangle_channel.clock_length_counter();
                self.noise_channel.clock_length_counter();
                self.square_channel1.clock_sweep();
                self.square_channel2.clock_sweep();
            }
        }
    }

    /// Advance the APU by exactly one CPU cycle with known parity.
    pub fn tick_one(&mut self, dma_operation: DmaOperation) -> Option<Address> {
        self.square_channel1.tick();
        self.square_channel2.tick();
        self.triangle_channel.tick();
        self.noise_channel.tick();

        let dma_request = self.dmc.tick();

        if let Some(signal) = self.frame_counter.tick(dma_operation) {
            self.dispatch_frame_signal(signal);
        }

        self.cycle_accumulator += 1.0;
        if self.cycle_accumulator >= CYCLES_PER_SAMPLE {
            self.cycle_accumulator -= CYCLES_PER_SAMPLE;
            let mixed_output = self.mix();
            self.samples.push(self.filter.filter(mixed_output));
        }

        dma_request
    }

    /// Advance the APU by `cycles` CPU cycles, accumulating audio samples.
    /// DMA requests from the DMC are ignored here; use `tick_one` directly for DMA handling.
    pub fn tick(&mut self, cycles: usize) {
        for _ in 0..cycles {
            self.tick_one(DmaOperation::Get);
        }
    }

    pub fn drain_samples(&mut self) -> Vec<f32> {
        mem::take(&mut self.samples)
    }

    /// NES mixer approximation based on the Lookup Table solution
    /// in [NESDev wiki page][nes_dev].
    ///
    /// [nes_dev]: https://www.nesdev.org/wiki/APU_Mixer
    fn mix(&self) -> f32 {
        let square1_output = self.square_channel1.output().as_float();
        let square2_output = self.square_channel2.output().as_float();
        let square_sum = square1_output + square2_output;
        let square_out = if square_sum == 0.0 {
            0.0
        } else {
            95.88 / (8128.0 / square_sum + 100.0)
        };

        let triangle_output = self.triangle_channel.output().as_float();
        let noise_output = self.noise_channel.output().as_float();
        let dmc_output = self.dmc.output().as_float();
        let tnd_sum = triangle_output / 8227.0 + noise_output / 12241.0 + dmc_output / 22638.0;
        let tnd_out = if tnd_sum == 0.0 {
            0.0
        } else {
            159.79 / (1.0 / tnd_sum + 100.0)
        };

        square_out + tnd_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::DmaOperation;

    fn make_apu() -> Apu {
        let mut apu = Apu::default();
        // Write $4015 to exit open-bus mode
        apu.set_status_register(Byte::new(0x00));
        apu
    }

    #[test]
    fn dmc_bit4_in_status_when_active() {
        let mut apu = make_apu();
        apu.dmc.write_sample_address(Byte::new(0x10));
        apu.dmc.write_sample_length(Byte::new(0x01));
        apu.set_status_register(Byte::new(0x10)); // enable DMC
        let status = apu.peek_status_register();
        assert_eq!(status & 0x10, 0x10, "bit 4 should be set when DMC active");
    }

    #[test]
    fn dmc_bit4_cleared_when_disabled() {
        let mut apu = make_apu();
        apu.dmc.write_sample_address(Byte::new(0x10));
        apu.dmc.write_sample_length(Byte::new(0x01));
        apu.set_status_register(Byte::new(0x10));
        apu.set_status_register(Byte::new(0x00)); // disable DMC
        let status = apu.peek_status_register();
        assert_eq!(
            status & 0x10,
            0x00,
            "bit 4 should be clear when DMC inactive"
        );
    }

    #[test]
    fn dmc_irq_appears_in_status_bit7() {
        let mut apu = make_apu();
        apu.dmc.irq_pending = true;
        let status = apu.peek_status_register();
        assert_eq!(
            status & 0x80,
            0x80,
            "bit 7 should be set when DMC IRQ pending"
        );
    }

    #[test]
    fn tick_one_returns_dma_request_when_dmc_needs_sample() {
        let mut apu = make_apu();
        apu.dmc.write_sample_address(Byte::new(0x00));
        apu.dmc.write_sample_length(Byte::new(0x01));
        apu.dmc.write_flags_and_rate(Byte::new(0x0F)); // rate index 15, period=54
        apu.set_status_register(Byte::new(0x10)); // enable DMC
        // Tick until we get a DMA request (should come on first output unit clock = ~54 ticks)
        let mut dma_addr = None;
        for _ in 0..200 {
            if let Some(addr) = apu.tick_one(DmaOperation::Get) {
                dma_addr = Some(addr);
                break;
            }
        }
        assert!(dma_addr.is_some(), "APU should request DMA for DMC sample");
    }

    #[test]
    fn irq_status_includes_dmc_irq() {
        let mut apu = make_apu();
        apu.dmc.irq_pending = true;
        assert!(apu.is_irq_pending(), "irq_status should reflect DMC IRQ");
    }
}
