use crate::Byte;

/// NES envelope generator shared by the square and noise channels.
///
/// Each quarter-frame clock either restarts the envelope (when the start flag
/// is set) or steps the divider down, decrementing the decay counter when it
/// expires. The decay level is output as the channel volume when constant-volume
/// mode is off.
#[derive(Debug, Default, Copy, Clone)]
pub(super) struct Envelope {
    start_flag: bool,
    divider: Byte,
    decay: Byte,
}

impl Envelope {
    /// Set the start flag. Call this when the channel's fourth register is written
    /// ($4003 for square, $400F for noise). The envelope resets on the next clock.
    pub fn restart(&mut self) {
        self.start_flag = true;
    }

    /// Clock the envelope generator (called every quarter-frame, ~240 Hz).
    ///
    /// `period` is the 4-bit value from bits 3–0 of the volume register.
    /// `looping` is the length-counter-halt / envelope-loop bit (bit 5 of the
    /// volume register). When `looping` is true the decay wraps from 0 back to 15.
    pub fn clock(&mut self, period: Byte, is_looping: bool) {
        if self.start_flag {
            self.start_flag = false;
            self.decay = 15.into();
            self.divider = period;
        } else if self.divider == 0 {
            self.divider = period;
            if self.decay > 0 {
                self.decay -= 1;
            } else if is_looping {
                self.decay = 15.into();
            }
        } else {
            self.divider -= 1;
        }
    }

    /// Current decay level (0–15).
    pub fn decay_level(&self) -> Byte {
        self.decay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clocked(period: impl Into<Byte>, is_looping: bool, n: usize) -> Envelope {
        let period = period.into();
        let mut envelope = Envelope::default();
        envelope.restart();
        for _ in 0..n {
            envelope.clock(period, is_looping);
        }
        envelope
    }

    #[test]
    fn restart_resets_decay_to_15() {
        // First clock after restart fires the start-flag branch: decay → 15.
        let e = clocked(3, false, 1);
        assert_eq!(e.decay_level(), 15);
    }

    #[test]
    fn divider_counts_down_before_decrementing_decay() {
        // With period=3, decay should still be 15 after 3 additional clocks
        // (divider goes 3→2→1→0 but only steps decay when it hits 0, which
        // happens on the 4th clock, i.e. period+1 total clocks from restart).
        let e = clocked(3, false, 3);
        assert_eq!(e.decay_level(), 15);
    }

    #[test]
    fn decay_decrements_on_divider_expiry() {
        // Clock 1: start-flag fires → decay=15, divider=3.
        // Clocks 2/3/4: divider counts 3→2→1→0 (else branch, no decay change).
        // Clock 5: divider==0 branch fires → decay steps to 14.
        // Total: period+2 = 5 clocks.
        let e = clocked(3, false, 5);
        assert_eq!(e.decay_level(), 14);
    }

    #[test]
    fn decay_holds_at_zero_when_not_looping() {
        // Run envelope all the way down to 0 (15 decay steps × (period+1) clocks
        // each, plus the initial restart clock).
        let period = 1u8;
        let steps_to_zero = 1 + 15 * (period as usize + 1);
        let e = clocked(period, false, steps_to_zero);
        assert_eq!(e.decay_level(), 0);

        // One more divider expiry should leave decay at 0 (non-looping).
        let e = clocked(period, false, steps_to_zero + period as usize + 1);
        assert_eq!(e.decay_level(), 0);
    }

    #[test]
    fn decay_wraps_to_15_when_looping() {
        let period = 1u8;
        let steps_to_zero = 1 + 15 * (period as usize + 1);
        // One divider expiry past zero with looping=true should wrap to 15.
        let e = clocked(period, true, steps_to_zero + period as usize + 1);
        assert_eq!(e.decay_level(), 15);
    }

    #[test]
    fn period_reloads_after_each_decay_step() {
        // First step fires at clock period+2 (start-flag costs 1 clock, then
        // the divider counts period+1 more clocks before expiring).
        // Each subsequent step fires every period+1 clocks.
        let period = 2u8;
        let first_step = period as usize + 2; // = 4
        let interval = period as usize + 1; // = 3 (subsequent steps)

        let e = clocked(period, false, first_step);
        assert_eq!(e.decay_level(), 14, "should be 14 after first step");

        // Still 14 one clock before the second step.
        let e = clocked(period, false, first_step + interval - 1);
        assert_eq!(
            e.decay_level(),
            14,
            "should still be 14 just before second step"
        );

        // Drops to 13 exactly at the second step.
        let e = clocked(period, false, first_step + interval);
        assert_eq!(e.decay_level(), 13, "should be 13 after second step");
    }
}
