use crate::utils::NthBit;
use crate::{Byte, Word};

// Maps the 5-bit length counter load index ($4003 bits 7-3) to the actual counter value.
// Indexed by the 5-bit value written to the register.
const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

// 4 duty cycle patterns, each 8 steps. Index is (volume register >> 6).
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0], // 12.5%
    [0, 1, 1, 0, 0, 0, 0, 0], // 25%
    [0, 1, 1, 1, 1, 0, 0, 0], // 50%
    [1, 0, 0, 1, 1, 1, 1, 1], // 75% (inverted 25%)
];

#[derive(Debug, Default, Copy, Clone)]
pub struct SquareChannel {
    // Raw register bytes – written directly by the bus.
    pub volume: Byte,
    pub sweep: Byte,
    pub timer_low: Byte,
    pub length_and_timer_high: Byte,

    // Synthesis state
    enabled: bool,
    // Down-counter. Reloaded to 2*(period+1)-1 when it reaches 0, so that the
    // sequencer advances at half the CPU clock (as on hardware).
    timer_counter: u16,
    sequencer_pos: u8,
    length_counter: u8,

    // Envelope state
    envelope_start_flag: bool,
    envelope_divider: u8,
    envelope_decay: u8,
}

#[allow(unused)]
impl SquareChannel {
    // ── Register field accessors ─────────────────────────────────────────────

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

    // ── Synthesis methods ────────────────────────────────────────────────────

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
            let index = self.length_counter_load().value() as usize;
            self.length_counter = LENGTH_TABLE[index];
        }
        // Restart envelope regardless of enabled state (hardware behaviour).
        self.envelope_start_flag = true;
    }

    /// Advance the timer by one CPU cycle.
    /// The NES pulse timer runs at half the CPU clock, so we reload with
    /// 2*(period+1)-1 to get the correct sequencer frequency.
    pub fn tick(&mut self) {
        if self.timer_counter == 0 {
            let period = self.timer().value();
            self.timer_counter = period * 2 + 1;
            self.sequencer_pos = (self.sequencer_pos + 1) % 8;
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
        if self.envelope_start_flag {
            self.envelope_start_flag = false;
            self.envelope_decay = 15;
            self.envelope_divider = self.volume().value();
        } else if self.envelope_divider == 0 {
            self.envelope_divider = self.volume().value();
            if self.envelope_decay > 0 {
                self.envelope_decay -= 1;
            } else if self.is_length_counter_halted() {
                // Length-counter-halt bit doubles as the envelope loop flag.
                self.envelope_decay = 15;
            }
        } else {
            self.envelope_divider -= 1;
        }
    }

    /// Current output level in the range 0–15, ready for mixing.
    pub fn output(&self) -> u8 {
        if self.length_counter == 0 {
            return 0;
        }
        // Hardware silences the channel for very short periods to avoid DC offset.
        if self.timer().value() < 8 {
            return 0;
        }
        let duty_bit = DUTY_TABLE[self.duty().value() as usize][self.sequencer_pos as usize];
        if duty_bit == 0 {
            return 0;
        }
        if self.is_constant_volume() {
            self.volume().value()
        } else {
            self.envelope_decay
        }
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
}
