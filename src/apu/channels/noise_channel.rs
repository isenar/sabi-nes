use crate::Byte;
use crate::utils::NthBit;

use super::common::LENGTH_TABLE;
use super::envelope::Envelope;

// Maps the 4-bit period index (bits 3-0 of $400E) to a CPU-clock timer period (NTSC).
const TIMER_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

#[derive(Debug, Clone, Copy)]
pub struct NoiseChannel {
    pub volume: Byte,
    pub mode_and_period: Byte,
    pub len_counter_and_env_restart: Byte,

    enabled: bool,
    timer_counter: u16,
    // 15-bit LFSR. Starts at 1 (bit 0 set → channel starts muted until it cycles).
    lfsr: u16,
    length_counter: u8,

    envelope: Envelope,
}

impl Default for NoiseChannel {
    fn default() -> Self {
        Self {
            volume: Byte::default(),
            mode_and_period: Byte::default(),
            len_counter_and_env_restart: Byte::default(),
            enabled: false,
            timer_counter: 0,
            lfsr: 1, // hardware power-on state
            length_counter: 0,
            envelope: Envelope::default(),
        }
    }
}

impl NoiseChannel {
    /// Returns true if the length counter is non-zero (used for $4015 status read).
    pub fn is_active(&self) -> bool {
        self.length_counter > 0
    }

    /// Called when bit 3 of $4015 is written.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    /// Called when $400F (length counter + envelope restart) is written.
    pub fn on_length_timer_write(&mut self) {
        if self.enabled {
            let index = (self.len_counter_and_env_restart >> 3).value() as usize;
            self.length_counter = LENGTH_TABLE[index];
        }
        self.envelope.restart();
    }

    /// Advance the timer by one CPU cycle; clock the LFSR when it expires.
    pub fn tick(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = TIMER_TABLE[self.timer_period().value() as usize];
            self.clock_lfsr();
        } else {
            self.timer_counter -= 1;
        }
    }

    /// Clock the length counter (called at ~120 Hz, on each half-frame).
    pub fn clock_length_counter(&mut self) {
        if !self.is_length_counter_halted() && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    /// Clock the envelope generator (called at ~240 Hz, on each quarter-frame).
    pub fn clock_envelope(&mut self) {
        self.envelope.clock(
            self.volume_divider_period(),
            self.is_length_counter_halted(),
        );
    }

    /// Current output level in the range 0–15, ready for mixing.
    /// Silenced if disabled, length counter is zero, or LFSR bit 0 is set.
    pub fn output(&self) -> Byte {
        if !self.enabled || self.length_counter == 0 || self.lfsr & 1 == 1 {
            return 0x00.into();
        }
        if self.is_constant_volume() {
            self.volume_divider_period()
        } else {
            self.envelope.decay_level()
        }
    }

    fn is_length_counter_halted(&self) -> bool {
        self.volume.nth_bit::<5>()
    }

    fn is_constant_volume(&self) -> bool {
        self.volume.nth_bit::<4>()
    }

    fn volume_divider_period(&self) -> Byte {
        self.volume & 0b0000_1111
    }

    fn mode(&self) -> NoiseMode {
        if self.mode_and_period.nth_bit::<7>() {
            NoiseMode::Short
        } else {
            NoiseMode::Long
        }
    }

    fn timer_period(&self) -> Byte {
        self.mode_and_period & 0b0000_1111
    }

    /// Advance the LFSR by one step.
    /// Long mode: feedback = bit 0 XOR bit 1  (produces 32,767-step white noise)
    /// Short mode: feedback = bit 0 XOR bit 6  (produces 93-step periodic noise)
    fn clock_lfsr(&mut self) {
        let other_bit = match self.mode() {
            NoiseMode::Long => (self.lfsr >> 1) & 1,
            NoiseMode::Short => (self.lfsr >> 6) & 1,
        };
        let feedback = (self.lfsr & 1) ^ other_bit;
        self.lfsr = (self.lfsr >> 1) | (feedback << 14);
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NoiseMode {
    Short,
    Long,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_data() {
        let channel = NoiseChannel {
            volume: Byte::new(0b1011_1010),
            ..NoiseChannel::default()
        };

        assert!(channel.is_length_counter_halted());
        assert!(channel.is_constant_volume());
        assert_eq!(channel.volume_divider_period(), 0b1010);
    }

    #[test]
    fn mode_and_period_data() {
        let channel = NoiseChannel {
            mode_and_period: Byte::new(0b1010_0011),
            ..NoiseChannel::default()
        };

        assert_eq!(NoiseMode::Short, channel.mode());
        assert_eq!(channel.timer_period(), 0b0011);
    }

    #[test]
    fn lfsr_long_mode_produces_noise() {
        let mut channel = NoiseChannel::default();
        channel.set_enabled(true);
        channel.len_counter_and_env_restart = Byte::new(0b1111_1000);
        channel.on_length_timer_write();
        channel.volume = Byte::new(0b0001_1111); // constant volume = 15

        // Collect 32 output values after clocking the LFSR directly
        let mut seen_nonzero = false;
        for _ in 0..100 {
            channel.clock_lfsr();
            if channel.output() > 0 {
                seen_nonzero = true;
                break;
            }
        }
        assert!(
            seen_nonzero,
            "LFSR should produce non-zero output within 100 clocks"
        );
    }

    #[test]
    fn lfsr_short_mode_has_short_period() {
        let mut channel = NoiseChannel {
            mode_and_period: Byte::new(0b1000_0000),
            ..NoiseChannel::default()
        };

        // Short mode uses a 93-step sequence; the LFSR should return to 1 after 93 steps
        for _ in 0..93 {
            channel.clock_lfsr();
        }
        assert_eq!(
            channel.lfsr, 1,
            "Short-mode LFSR should complete its 93-step cycle"
        );
    }

    #[test]
    fn output_muted_when_length_counter_zero() {
        let mut channel = NoiseChannel::default();
        channel.set_enabled(true);
        channel.volume = Byte::new(0b0001_1111); // constant volume = 15
        // Don't write $400F, so length_counter stays 0
        assert_eq!(channel.output(), 0);
    }

    #[test]
    fn output_uses_envelope_decay() {
        // volume: no halt, constant-volume bit CLEAR, period=3
        let mut channel = NoiseChannel {
            volume: Byte::new(0b0000_0011),
            ..NoiseChannel::default()
        };
        channel.set_enabled(true);
        channel.len_counter_and_env_restart = Byte::new(0b1111_1000);
        channel.on_length_timer_write();

        // Advance LFSR until bit 0 is 0 (channel not muted by LFSR)
        for _ in 0..100 {
            if channel.lfsr & 1 == 0 {
                break;
            }
            channel.clock_lfsr();
        }
        assert_eq!(
            channel.lfsr & 1,
            0,
            "LFSR bit 0 should be 0 for audible output"
        );

        // First envelope clock: start-flag fires, decay → 15.
        channel.clock_envelope();
        assert_eq!(channel.output(), 15);

        // After period+1 more clocks, decay steps to 14.
        for _ in 0..=3 {
            channel.clock_envelope();
        }
        assert_eq!(channel.output(), 14);
    }
}
