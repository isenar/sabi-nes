use crate::Byte;
use crate::utils::NthBit;

#[derive(Debug, Default, Copy, Clone)]
pub struct SquareChannel {
    pub volume: Byte,
    pub sweep: Byte,
    pub timer_low: Byte,
    pub length_and_timer_high: Byte,
}

#[allow(unused)]
impl SquareChannel {
    fn duty(self) -> Byte {
        self.volume >> 6
    }

    fn is_length_counter_halted(self) -> bool {
        self.volume.nth_bit(5)
    }

    fn is_constant_volume(self) -> bool {
        self.volume.nth_bit(4)
    }

    fn volume(self) -> Byte {
        self.volume & 0b0000_1111
    }

    fn is_sweep_enabled(self) -> bool {
        self.sweep.nth_bit(7)
    }

    fn sweep_period(self) -> Byte {
        (self.sweep >> 4) & 0b0000_0111
    }

    fn is_sweep_negated(self) -> bool {
        self.sweep.nth_bit(3)
    }

    fn sweep_shift(self) -> Byte {
        self.sweep & 0b0000_0111
    }

    fn timer(self) -> u16 {
        let timer_high = (self.length_and_timer_high & 0b0000_0111) as u16;
        let timer_low = self.timer_low as u16;

        (timer_high << 8) | timer_low
    }

    fn length_counter_load(self) -> Byte {
        self.length_and_timer_high >> 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_data() {
        let channel = SquareChannel {
            volume: 0b1011_0101,
            ..SquareChannel::default()
        };

        assert_eq!(0b10, channel.duty());
        assert!(channel.is_length_counter_halted());
        assert!(channel.is_constant_volume());
        assert_eq!(0b0101, channel.volume());
    }

    #[test]
    fn sweep_data() {
        let channel = SquareChannel {
            sweep: 0b1011_1101,
            ..SquareChannel::default()
        };

        assert!(channel.is_sweep_enabled());
        assert_eq!(0b011, channel.sweep_period());
        assert!(channel.is_sweep_negated());
        assert_eq!(0b101, channel.sweep_shift());
    }

    #[test]
    fn timer_and_length_counter_data() {
        let channel = SquareChannel {
            timer_low: 0b1011_1001,
            length_and_timer_high: 0b1011_1010,
            ..SquareChannel::default()
        };

        assert_eq!(0b0001_0111, channel.length_counter_load());
        assert_eq!(0b0010_1011_1001, channel.timer());
    }
}
