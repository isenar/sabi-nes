use crate::utils::NthBit;
use crate::{Byte, Word};

use super::common::LENGTH_TABLE;

// 32-step triangle sequence: counts 15 down to 0, then 0 up to 15.
const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

/// The triangle channel produces a quantised triangle wave.
/// It has no volume control, but it has a length counter
/// as well as a higher resolution linear counter control (called "linear"
/// since it uses the 7-bit value written to $4008 directly instead of a
/// lookup table like the length counter).
#[derive(Debug, Default, Copy, Clone)]
pub struct TriangleChannel {
    pub linear_counter: Byte,
    pub timer_low: Byte,
    pub length_and_timer_high: Byte,

    enabled: bool,
    // Counts from period down to 0, then reloads. Sequencer steps on each reload.
    // The triangle timer runs at the full CPU clock (no half-clock divider unlike square).
    timer_counter: Word,
    sequencer_pos: u8,
    length_counter: u8,

    // Linear counter state
    linear_counter_value: u8,
    linear_counter_reload_flag: bool,
}

impl TriangleChannel {
    pub fn is_active(&self) -> bool {
        self.length_counter > 0
    }

    /// Called when bit 2 of $4015 is written.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    /// Called when $400B (length and timer high) is written.
    /// Loads the length counter from the lookup table (if enabled) and
    /// sets the linear counter reload flag.
    pub fn on_length_timer_write(&mut self) {
        if self.enabled {
            let index = self.length_counter_load().as_usize();
            self.length_counter = LENGTH_TABLE[index];
        }
        self.linear_counter_reload_flag = true;
    }

    /// Advance the timer by one CPU cycle.
    /// The triangle timer runs at the full CPU clock: the sequencer advances
    /// every (period + 1) CPU cycles, but only while both the length counter
    /// and linear counter are non-zero. When either expires the sequencer
    /// freezes at its current position so the DAC holds the last value
    /// instead of jumping to 0 (which would cause an audible click).
    pub fn tick(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer();
            if self.length_counter > 0 && self.linear_counter_value > 0 {
                self.sequencer_pos = (self.sequencer_pos + 1) % 32;
            }
        } else {
            self.timer_counter -= 1;
        }
    }

    /// Clock the linear counter (called at ~240 Hz, on each quarter-frame).
    pub fn clock_linear_counter(&mut self) {
        if self.linear_counter_reload_flag {
            self.linear_counter_value = self.counter_reload().value();
        } else if self.linear_counter_value > 0 {
            self.linear_counter_value -= 1;
        }
        // Control flag (bit 7) doubles as halt: when clear, the reload flag
        // is cleared after use so the linear counter can expire normally.
        if !self.is_linear_counter_enabled() {
            self.linear_counter_reload_flag = false;
        }
    }

    /// Clock the length counter (called at ~120 Hz, on each half-frame).
    pub fn clock_length_counter(&mut self) {
        if !self.is_linear_counter_enabled() && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    /// Current output level in the range 0–15, ready for mixing.
    pub fn output(&self) -> Byte {
        // Silence ultrasonic periods: very short periods produce a DC-offset
        // pop on real hardware; suppress them the same way square does.
        if !self.enabled || self.timer() < 2 {
            return 0x00.into();
        }

        // When length or linear counter is 0, the sequencer is frozen (see tick()).
        // We output the held value rather than 0 to avoid a click on note-off.
        TRIANGLE_SEQUENCE[self.sequencer_pos as usize].into()
    }

    /// Bit 7 of $4008: doubles as both the linear counter enable and the
    /// length counter halt (control flag).
    pub fn is_linear_counter_enabled(self) -> bool {
        self.linear_counter.nth_bit::<7>()
    }

    pub fn counter_reload(&self) -> Byte {
        self.linear_counter & 0b0111_1111
    }

    fn timer(&self) -> Word {
        let timer_high = self.length_and_timer_high & 0b0000_0111;
        Word::from_le_bytes(self.timer_low, timer_high)
    }

    fn length_counter_load(&self) -> Byte {
        self.length_and_timer_high >> 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_counter_data() {
        let channel = TriangleChannel {
            linear_counter: 0b1011_0100.into(),
            ..TriangleChannel::default()
        };

        assert!(channel.is_linear_counter_enabled());
        assert_eq!(channel.counter_reload(), 0b0011_0100);
    }

    #[test]
    fn timer_data() {
        let channel = TriangleChannel {
            timer_low: Byte::new(0b1101_1011),
            length_and_timer_high: Byte::new(0b1011_0011),
            ..TriangleChannel::default()
        };

        assert_eq!(channel.length_counter_load(), 0b0001_0110);
        assert_eq!(channel.timer(), 0b0011_1101_1011);
    }

    #[test]
    fn output_silent_when_disabled() {
        let mut channel = TriangleChannel {
            linear_counter: 0b0111_1111.into(),
            timer_low: Byte::new(100),
            ..TriangleChannel::default()
        };
        channel.set_enabled(true);
        channel.on_length_timer_write();
        channel.clock_linear_counter();
        assert!(channel.output() > 0);

        channel.set_enabled(false);
        assert_eq!(channel.output(), 0);
    }

    #[test]
    fn output_frozen_when_linear_counter_zero() {
        let mut channel = TriangleChannel {
            // control flag clear, reload value = 0
            linear_counter: 0b0000_0000.into(),
            timer_low: Byte::new(100),
            ..TriangleChannel::default()
        };
        channel.set_enabled(true);
        channel.on_length_timer_write();
        // Clock linear counter: reloads to 0, then clears reload flag
        channel.clock_linear_counter();
        // Sequencer is frozen at pos 0; output holds that value rather than
        // jumping to 0, which would cause an audible click.
        assert_eq!(channel.output(), TRIANGLE_SEQUENCE[0]);
    }

    #[test]
    fn sequencer_steps_through_triangle_shape() {
        let period = 4;
        let mut channel = TriangleChannel {
            linear_counter: 0b0111_1111.into(),
            timer_low: Byte::new(period),
            ..TriangleChannel::default()
        };
        channel.set_enabled(true);
        channel.on_length_timer_write();
        channel.clock_linear_counter();

        // Read pos 0 before any ticks (timer_counter starts at 0, so the first
        // tick would immediately advance the sequencer).
        let mut outputs = vec![channel.output()];
        for _ in 0..31 {
            // Each step takes period+1 CPU ticks to advance to the next position.
            for _ in 0..=period {
                channel.tick();
            }
            outputs.push(channel.output());
        }
        assert_eq!(outputs, TRIANGLE_SEQUENCE);
    }
}
