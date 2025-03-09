use crate::Byte;
use crate::utils::NthBit;

#[derive(Debug, Default, Clone, Copy)]
pub struct NoiseChannel {
    pub volume: Byte,
    pub mode_and_period: Byte,
    pub len_counter_and_env_restart: Byte,
}

impl NoiseChannel {
    pub fn is_length_counter_halted(&self) -> bool {
        self.volume.nth_bit(5)
    }

    pub fn is_constant_volume(&self) -> bool {
        self.volume.nth_bit(4)
    }

    pub fn volume_divider_period(&self) -> Byte {
        self.volume & 0b0000_1111
    }

    pub fn mode(&self) -> NoiseMode {
        match self.mode_and_period.nth_bit(7) {
            true => NoiseMode::Short,
            false => NoiseMode::Long,
        }
    }

    pub fn timer_period(&self) -> Byte {
        self.mode_and_period & 0b0000_1111
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
            volume: 0b1011_1010,
            ..NoiseChannel::default()
        };

        assert!(channel.is_length_counter_halted());
        assert!(channel.is_constant_volume());
        assert_eq!(0b1010, channel.volume_divider_period());
    }

    #[test]
    fn mode_and_period_data() {
        let channel = NoiseChannel {
            mode_and_period: 0b1010_0011,
            ..NoiseChannel::default()
        };

        assert_eq!(NoiseMode::Short, channel.mode());
        assert_eq!(0b0011, channel.timer_period());
    }
}
