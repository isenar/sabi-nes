use crate::Byte;
use crate::utils::NthBit;

/// The NES APU's delta modulation channel (DMC) can output 1-bit delta-encoded
/// samples or can have its 7-bit counter directly loaded,
/// allowing flexible manual sample playback.
#[derive(Debug, Default, Clone, Copy)]
pub struct Dmc {
    pub flags_and_rate: Byte,
    pub direct_load: Byte,
    pub sample_address: Byte,
    pub sample_length: Byte,
}

impl Dmc {
    pub fn is_irq_enabled(self) -> bool {
        self.flags_and_rate.nth_bit(7)
    }

    pub fn is_looping(self) -> bool {
        self.flags_and_rate.nth_bit(6)
    }

    pub fn rate_index(self) -> Byte {
        self.flags_and_rate & 0b0000_0111
    }

    pub fn direct_load(self) -> Byte {
        self.direct_load & 0b0111_1111
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_and_rate_data() {
        let dmc = Dmc {
            flags_and_rate: 0b1010_0011,
            ..Dmc::default()
        };

        assert!(dmc.is_irq_enabled());
        assert!(!dmc.is_looping());
        assert_eq!(0b0011, dmc.rate_index());
    }

    #[test]
    fn direct_load_data() {
        let dmc = Dmc {
            direct_load: 0b1010_0101,
            ..Dmc::default()
        };

        assert_eq!(0b0010_0101, dmc.direct_load());
    }
}
