use crate::Byte;
use crate::utils::NthBit;

/// The triangle channel produces a quantized triangle wave.
/// It has no volume control, but it has a length counter
/// as well as a higher resolution linear counter control (called "linear"
/// since it uses the 7-bit value written to $4008 directly instead of a
/// lookup table like the length counter).
#[derive(Debug, Default, Copy, Clone)]
pub struct TriangleChannel {
    pub linear_counter: Byte,
    pub timer_low: Byte,
    pub length_and_timer_high: Byte,
}

impl TriangleChannel {
    pub fn is_linear_counter_enabled(self) -> bool {
        self.linear_counter.nth_bit(7)
    }

    pub fn counter_reload(&self) -> Byte {
        self.linear_counter & 0b0111_1111
    }

    pub fn timer(&self) -> u16 {
        let timer_high = (self.length_and_timer_high & 0b0000_0111) as u16;
        let timer_low = self.timer_low as u16;

        (timer_high << 8) | timer_low
    }

    pub fn length_counter_load(&self) -> Byte {
        self.length_and_timer_high >> 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_counter_data() {
        let channel = TriangleChannel {
            linear_counter: 0b1011_0100,
            ..TriangleChannel::default()
        };

        assert!(channel.is_linear_counter_enabled());
        assert_eq!(0b0011_0100, channel.counter_reload());
    }

    #[test]
    fn timer_data() {
        let channel = TriangleChannel {
            timer_low: 0b1101_1011,
            length_and_timer_high: 0b1011_0011,
            ..TriangleChannel::default()
        };

        assert_eq!(0b0001_0110, channel.length_counter_load());
        assert_eq!(0b0011_1101_1011, channel.timer());
    }
}
