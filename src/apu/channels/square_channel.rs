use crate::utils::NthBit;
use crate::{Byte, Word};

use super::common::LENGTH_TABLE;
use super::envelope::Envelope;

// 4 duty cycle patterns, each 8 steps. Index is (volume register >> 6).
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0], // 12.5%
    [0, 1, 1, 0, 0, 0, 0, 0], // 25%
    [0, 1, 1, 1, 1, 0, 0, 0], // 50%
    [1, 0, 0, 1, 1, 1, 1, 1], // 75% (inverted 25%)
];

#[derive(Debug, Copy, Clone)]
pub struct SquareChannel {
    pub volume: Byte,
    pub sweep: Byte,
    pub timer_low: Byte,
    pub length_and_timer_high: Byte,

    // Channel identity: pulse 1 uses one's-complement negation in sweep,
    // pulse 2 uses two's-complement. Must be set at construction time.
    is_channel_1: bool,

    enabled: bool,

    timer_counter: Word,
    sequencer_position: u8,
    length_counter: u8,

    envelope: Envelope,

    // Sweep state
    sweep_divider: Byte,
    sweep_reload_flag: bool,
}

impl Default for SquareChannel {
    fn default() -> Self {
        Self {
            // Register bytes start at 0xFF to match NES power-on open-bus state.
            volume: Byte::new(0xFF),
            sweep: Byte::new(0xFF),
            timer_low: Byte::new(0xFF),
            length_and_timer_high: Byte::new(0xFF),
            is_channel_1: false,
            enabled: false,
            timer_counter: Word::default(),
            sequencer_position: 0,
            length_counter: 0,
            envelope: Envelope::default(),
            sweep_divider: Byte::default(),
            sweep_reload_flag: false,
        }
    }
}

impl SquareChannel {
    pub fn channel1() -> Self {
        Self {
            is_channel_1: true,
            ..Self::default()
        }
    }
}

impl SquareChannel {
    /// Returns true if the length counter is non-zero (used for $4015 status read).
    pub fn is_active(&self) -> bool {
        self.length_counter > 0
    }

    /// Called when bit for this channel is written to $4015.
    /// Disabling immediately silences by zeroing the length counter.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    /// Called after the bus writes to $4003 / $4007 (length + timer high byte).
    /// Loads the length counter from the lookup table (only if enabled) and
    /// flags the envelope for restart.
    pub fn on_length_timer_write(&mut self) {
        if self.enabled {
            let index = self.length_counter_load().as_usize();
            self.length_counter = LENGTH_TABLE[index];
        }
        // Restart envelope regardless of enabled state (hardware behaviour).
        self.envelope.restart();
    }

    /// Advance the timer by one CPU cycle.
    /// The NES pulse timer runs at half the CPU clock, so we reload with
    /// 2*(period+1)-1 to get the correct sequencer frequency.
    pub fn tick(&mut self) {
        if self.timer_counter > 0 {
            self.timer_counter -= 1;
        } else {
            let period = self.timer().value();
            self.timer_counter = Word::new(period * 2 + 1);
            self.sequencer_position = (self.sequencer_position + 1) % 8;
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
        self.envelope
            .clock(self.volume(), self.is_length_counter_halted());
    }

    /// Called when $4001 / $4005 (sweep register) is written.
    /// Sets the reload flag so the divider reloads on the next half-frame clock.
    pub fn on_sweep_write(&mut self) {
        self.sweep_reload_flag = true;
    }

    /// Clock the sweep unit (called at ~120 Hz, on each half-frame).
    pub fn clock_sweep(&mut self) {
        let target = self.compute_target_period();
        let timer = self.timer();

        // Update the period if the sweep unit is active, the channel is enabled,
        // and the result is in range.
        if self.enabled
            && self.sweep_divider == 0
            && self.is_sweep_enabled()
            && self.sweep_shift() > 0
            && timer >= 8
            && target <= 0x7FF
        {
            self.set_timer_period(target);
        }

        if self.sweep_divider == 0 || self.sweep_reload_flag {
            self.sweep_divider = self.sweep_period();
            self.sweep_reload_flag = false;
        } else {
            self.sweep_divider -= 1;
        }
    }

    /// Current output level in the range 0–15, ready for mixing.
    pub fn output(&self) -> Byte {
        if self.length_counter == 0 {
            return 0x00.into();
        }
        // Mute for very short periods (avoids DC offset on hardware).
        if self.timer().value() < 8 {
            return 0x00.into();
        }
        // Mute if the sweep target would overflow, even when sweep is disabled.
        if self.compute_target_period() > 0x7FF {
            return 0x00.into();
        }
        let duty = DUTY_TABLE[self.duty().as_usize()];
        let duty_bit = duty[self.sequencer_position as usize];
        if duty_bit == 0 {
            return 0x00.into();
        }
        if self.is_constant_volume() {
            self.volume()
        } else {
            self.envelope.decay_level()
        }
    }

    fn duty(&self) -> Byte {
        self.volume >> 6
    }

    fn is_length_counter_halted(&self) -> bool {
        self.volume.nth_bit::<5>()
    }

    fn is_constant_volume(&self) -> bool {
        self.volume.nth_bit::<4>()
    }

    fn volume(&self) -> Byte {
        self.volume & 0b0000_1111
    }

    fn is_sweep_enabled(&self) -> bool {
        self.sweep.nth_bit::<7>()
    }

    fn sweep_period(&self) -> Byte {
        (self.sweep >> 4) & 0b0000_0111
    }

    fn is_sweep_negated(&self) -> bool {
        self.sweep.nth_bit::<3>()
    }

    fn sweep_shift(&self) -> Byte {
        self.sweep & 0b0000_0111
    }

    fn timer(&self) -> Word {
        let timer_high = self.length_and_timer_high & 0b0000_0111;
        Word::from_le_bytes(self.timer_low, timer_high)
    }

    fn length_counter_load(&self) -> Byte {
        self.length_and_timer_high >> 3
    }

    /// Compute the sweep target period without applying it.
    /// If shift == 0 the target equals the current period (no change).
    /// Negate uses one's complement for channel 1, two's complement for channel 2.
    fn compute_target_period(&self) -> u16 {
        let period = self.timer().value();
        let shift = self.sweep_shift().value();
        if shift == 0x00 {
            return period;
        }

        let change = period >> shift;
        if self.is_sweep_negated() {
            let adjustment = if self.is_channel_1 {
                change + 1
            } else {
                change
            };
            period.saturating_sub(adjustment)
        } else {
            period + change
        }
    }

    /// Write an 11-bit period back into the timer register fields.
    fn set_timer_period(&mut self, period: u16) {
        self.timer_low = Byte::new(period as u8);
        let high = Byte::new(((period >> 8) as u8) & 0b0000_0111);
        self.length_and_timer_high = (self.length_and_timer_high & 0b1111_1000) | high;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_data() {
        let channel = SquareChannel {
            volume: Byte::new(0b1011_0101),
            ..SquareChannel::default()
        };

        assert_eq!(channel.duty(), 0b10);
        assert!(channel.is_length_counter_halted());
        assert!(channel.is_constant_volume());
        assert_eq!(channel.volume(), 0b0101);
    }

    #[test]
    fn sweep_data() {
        let channel = SquareChannel {
            sweep: Byte::new(0b1011_1101),
            ..SquareChannel::default()
        };

        assert!(channel.is_sweep_enabled());
        assert_eq!(channel.sweep_period(), 0b011);
        assert!(channel.is_sweep_negated());
        assert_eq!(channel.sweep_shift(), 0b101);
    }

    #[test]
    fn timer_and_length_counter_data() {
        let channel = SquareChannel {
            timer_low: Byte::new(0b1011_1001),
            length_and_timer_high: Byte::new(0b1011_1010),
            ..SquareChannel::default()
        };

        assert_eq!(channel.length_counter_load(), 0b0001_0111);
        assert_eq!(channel.timer(), 0b0010_1011_1001);
    }

    /// Helper: a channel with a valid period (>=8), duty=50%, enabled, with
    /// length counter loaded. The sequencer is advanced to a position where the
    /// duty table outputs 1.
    fn active_channel_at_duty_high() -> SquareChannel {
        // volume: duty=50% (bits 7-6 = 0b10), no halt, no constant-vol, period=5
        let mut channel = SquareChannel {
            volume: Byte::new(0b1000_0101), // duty=10, halt=0, const=0, vol=5
            timer_low: Byte::new(100),      // period well above 8
            length_and_timer_high: Byte::new(0b0000_0000),
            ..SquareChannel::default()
        };
        channel.set_enabled(true);
        channel.on_length_timer_write();
        // Advance sequencer to position 1 where 50% duty table = 1
        // timer_counter starts at 0, so the first tick immediately advances the sequencer
        for _ in 0..=(100u16 * 2 + 1) {
            channel.tick();
        }
        channel
    }

    #[test]
    fn output_uses_envelope_when_not_constant_volume() {
        // volume byte: duty=50%, no halt, constant-volume bit CLEAR, vol=5
        let mut ch = active_channel_at_duty_high();
        // Clock envelope once: start-flag fires, decay → 15.
        ch.clock_envelope();
        assert_eq!(ch.output(), 15, "output should equal envelope decay (15)");

        // Clock envelope enough times to step decay down.
        // period=5, so after 5+1=6 more clocks decay steps to 14.
        for _ in 0..6 {
            ch.clock_envelope();
        }
        assert_eq!(ch.output(), 14);
    }

    #[test]
    fn output_uses_constant_volume() {
        // volume byte: duty=50%, no halt, constant-volume bit SET (bit 4), vol=7
        let mut ch = SquareChannel {
            volume: Byte::new(0b1001_0111), // duty=10, halt=0, const=1, vol=7
            timer_low: Byte::new(100),
            length_and_timer_high: Byte::new(0b0000_0000),
            ..SquareChannel::default()
        };
        ch.set_enabled(true);
        ch.on_length_timer_write();
        for _ in 0..=(100u16 * 2 + 1) {
            ch.tick();
        }
        // Clock envelope many times — output should always be the constant volume, not decay.
        for _ in 0..30 {
            ch.clock_envelope();
        }
        assert_eq!(ch.output(), 7);
    }

    #[test]
    fn output_muted_when_sweep_target_overflows() {
        // period=0x600 (1536), shift=1 (not negated):
        // target = 1536 + 768 = 2304 > 0x7FF → output must be muted.
        // (0x400 would only give 1536 which is still <= 0x7FF)
        let mut channel = SquareChannel {
            // duty=50% (bits 7-6 = 10), no halt, constant-volume, vol=15
            volume: Byte::new(0b1001_1111),
            // sweep: not enabled, period=0, not negated, shift=1
            sweep: Byte::new(0b0000_0001),
            timer_low: Byte::new(0x00),
            // high 3 bits = 6 (0x600 >> 8 = 6)
            length_and_timer_high: Byte::new(0b0000_0110),
            ..SquareChannel::default()
        };
        channel.set_enabled(true);
        channel.on_length_timer_write();
        // timer_counter starts at 0; first tick advances sequencer to pos 1
        // where 50% duty table = 1 (would normally produce output).
        channel.tick();
        assert_eq!(
            channel.output(),
            0,
            "output should be muted when sweep target > 0x7FF"
        );
    }
}
