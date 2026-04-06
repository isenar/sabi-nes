use crate::utils::NthBit;
use crate::{Address, Byte};

// NTSC DMC rate table: CPU clock periods indexed by bits 3-0 of $4010.
const RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

#[derive(Debug)]
struct DmaState {
    start_address: Address,
    length: u16,
    current_address: Address,
    bytes_remaining: u16,
    pending_dma_address: Option<Address>,
}

/// The NES APU's delta modulation channel (DMC) can output 1-bit delta-encoded
/// samples or can have its 7-bit counter directly loaded,
/// allowing flexible manual sample playback.
#[derive(Debug)]
pub struct Dmc {
    pub flags_and_rate: Byte,
    pub direct_load: Byte,
    pub sample_address: Byte,
    pub sample_length: Byte,

    // Timer
    timer_counter: u16,

    // Output unit
    output_level: Byte,
    sample_buffer: Option<Byte>,
    shift_register: Byte,
    pub(crate) output_unit_bits_remaining: u8,

    // DMA state
    dma_state: DmaState,
    pub(crate) irq_pending: bool,
}

impl Default for Dmc {
    fn default() -> Self {
        Self {
            flags_and_rate: Byte::default(),
            direct_load: Byte::default(),
            sample_address: Byte::default(),
            sample_length: Byte::default(),
            timer_counter: RATE_TABLE[0],
            output_level: Byte::default(),
            sample_buffer: None,
            shift_register: Byte::default(),
            output_unit_bits_remaining: 8,
            dma_state: DmaState {
                start_address: Address::new(0xC000),
                length: 1,
                current_address: Address::new(0xC000),
                bytes_remaining: 0,
                pending_dma_address: None,
            },
            irq_pending: false,
        }
    }
}

impl Dmc {
    pub fn is_irq_enabled(&self) -> bool {
        self.flags_and_rate.nth_bit::<7>()
    }

    pub fn is_looping(&self) -> bool {
        self.flags_and_rate.nth_bit::<6>()
    }

    fn rate_index(&self) -> Byte {
        self.flags_and_rate & 0b0000_1111
    }

    pub const fn output(&self) -> Byte {
        self.output_level
    }

    pub fn sample_buffer(&self) -> Option<Byte> {
        self.sample_buffer
    }

    pub fn is_active(&self) -> bool {
        self.dma_state.bytes_remaining > 0
    }

    pub fn write_flags_and_rate(&mut self, value: Byte) {
        self.flags_and_rate = value;
        if !value.nth_bit::<7>() {
            self.irq_pending = false;
        }
    }

    pub fn write_direct_load(&mut self, value: Byte) {
        self.direct_load = value;
        self.output_level = value & 0x7F;
    }

    pub fn write_sample_address(&mut self, sample_address: Byte) {
        self.sample_address = sample_address;
        self.dma_state.start_address = Address::new(sample_address.value() as u16 * 64) + 0xc000; // TODO
    }

    pub fn write_sample_length(&mut self, value: Byte) {
        self.sample_length = value;
        self.dma_state.length = value.value() as u16 * 16 + 1;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled {
            if self.dma_state.bytes_remaining == 0 {
                self.dma_state.current_address = self.dma_state.start_address;
                self.dma_state.bytes_remaining = self.dma_state.length;
                self.schedule_dma();
            }
        } else {
            self.dma_state.bytes_remaining = 0;
        }
    }

    pub fn deliver_sample(&mut self, byte: Byte) {
        self.sample_buffer = Some(byte);
        self.dma_state.pending_dma_address = None;
        self.advance_dma_address();
    }

    /// Tick the DMC timer by one CPU cycle. Returns the DMA address to fetch, if any.
    pub fn tick(&mut self) -> Option<Address> {
        let pending = self.dma_state.pending_dma_address.take();
        if self.timer_counter == 0 {
            self.timer_counter = RATE_TABLE[self.rate_index().as_usize()];
            self.clock_output_unit();
        } else {
            self.timer_counter -= 1;
        }

        pending.or(self.dma_state.pending_dma_address)
    }

    /// Clock the output unit (called when timer expires).
    fn clock_output_unit(&mut self) {
        if self.output_unit_bits_remaining == 0 {
            // Reload from buffer or silence
            if let Some(sample) = self.sample_buffer {
                self.shift_register = sample;
                self.output_unit_bits_remaining = 8;
                self.sample_buffer = None;
                // Schedule next DMA now that buffer is consumed
                self.schedule_dma();
                // Now do the shift/step
                self.step_output();
            }
            // else: silenced cycle, no output change
        } else {
            self.step_output();
            if self.output_unit_bits_remaining == 0 {
                // Bits exhausted after stepping; schedule next DMA if needed
                self.schedule_dma();
            }
        }
    }

    fn step_output(&mut self) {
        let sample_bit = self.shift_register.nth_bit::<0>();
        self.shift_register >>= 1;

        match (sample_bit, self.output_level) {
            (true, output_level) if output_level <= 125 => {
                self.output_level += 2;
            }
            (true, _) => {
                self.output_level = Byte::new(127);
            }
            (false, output_level) if output_level >= 2 => {
                self.output_level -= 2;
            }
            _ => {
                self.output_level = Byte::new(0);
            }
        }

        self.output_unit_bits_remaining -= 1;
    }

    fn schedule_dma(&mut self) {
        if self.dma_state.bytes_remaining > 0 && self.sample_buffer.is_none() {
            self.dma_state.pending_dma_address = Some(self.dma_state.current_address);
        }
    }

    fn advance_dma_address(&mut self) {
        if self.dma_state.bytes_remaining == 0 {
            return;
        }
        self.dma_state.bytes_remaining -= 1;
        if self.dma_state.current_address == 0xFFFF {
            self.dma_state.current_address = Address::new(0x8000);
        } else {
            self.dma_state.current_address += 1;
        }
        if self.dma_state.bytes_remaining == 0 {
            if self.is_looping() {
                self.dma_state.current_address = self.dma_state.start_address;
                self.dma_state.bytes_remaining = self.dma_state.length;
            } else if self.is_irq_enabled() {
                self.irq_pending = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Byte;

    fn make_dmc() -> Dmc {
        Dmc::default()
    }

    #[test]
    fn flags_and_rate_data() {
        let dmc = Dmc {
            flags_and_rate: Byte::new(0b1010_0011),
            ..Dmc::default()
        };

        assert!(dmc.is_irq_enabled());
        assert!(!dmc.is_looping());
        assert_eq!(dmc.rate_index(), 0b0011);
    }

    #[test]
    fn output_level_starts_at_zero() {
        assert_eq!(make_dmc().output(), 0);
    }

    #[test]
    fn direct_load_sets_output_immediately() {
        let mut dmc = make_dmc();
        dmc.write_direct_load(Byte::new(0b1011_0101)); // top bit ignored
        assert_eq!(dmc.output(), 0b011_0101);
    }

    #[test]
    fn output_clamps_up_at_127() {
        let mut dmc = make_dmc();
        dmc.write_direct_load(Byte::new(126));
        dmc.deliver_sample(Byte::new(0xFF));
        dmc.output_unit_bits_remaining = 0;
        dmc.clock_output_unit();
        assert_eq!(dmc.output(), 127);
    }

    #[test]
    fn output_clamps_down_at_zero() {
        let mut dmc = make_dmc();
        dmc.write_direct_load(Byte::new(0));
        dmc.deliver_sample(Byte::new(0x00));
        dmc.output_unit_bits_remaining = 0;
        dmc.clock_output_unit();
        assert_eq!(dmc.output(), 0);
    }

    #[test]
    fn sample_address_register() {
        let mut dmc = make_dmc();
        dmc.write_sample_address(Byte::new(0x01));
        assert_eq!(dmc.dma_state.start_address, 0xC040);
    }

    #[test]
    fn sample_length_register() {
        let mut dmc = make_dmc();
        dmc.write_sample_length(Byte::new(0x01));
        assert_eq!(dmc.dma_state.length, 17);
    }

    #[test]
    fn set_enabled_starts_playback_when_bytes_remain_zero() {
        let mut dmc = make_dmc();
        dmc.write_sample_address(Byte::new(0x10));
        dmc.write_sample_length(Byte::new(0x02));
        dmc.set_enabled(true);
        assert!(dmc.is_active());
        assert_eq!(dmc.dma_state.bytes_remaining, 33);
        assert_eq!(dmc.dma_state.current_address, 0xC000 + 0x10 * 64);
    }

    #[test]
    fn set_enabled_false_clears_bytes_remaining() {
        let mut dmc = make_dmc();
        dmc.write_sample_address(Byte::new(0x10));
        dmc.write_sample_length(Byte::new(0x02));
        dmc.set_enabled(true);
        dmc.set_enabled(false);
        assert!(!dmc.is_active());
        assert_eq!(dmc.dma_state.bytes_remaining, 0);
    }

    #[test]
    fn irq_pending_cleared_when_irq_enable_bit_cleared() {
        let mut dmc = make_dmc();
        dmc.irq_pending = true;
        dmc.write_flags_and_rate(Byte::new(0x00)); // bit 7 = 0
        assert!(!dmc.irq_pending);
    }

    #[test]
    fn deliver_sample_marks_buffer_ready() {
        let mut dmc = make_dmc();
        assert_eq!(dmc.sample_buffer, None);

        dmc.deliver_sample(Byte::new(0x42));
        assert_eq!(dmc.sample_buffer, Some(Byte::new(0x42)));
    }

    #[test]
    fn dma_address_wraps_at_ffff() {
        let mut dmc = make_dmc();
        dmc.dma_state.current_address = Address::new(0xFFFF);
        dmc.dma_state.bytes_remaining = 2;
        // Load a sample so output unit can consume it and schedule next fetch
        dmc.deliver_sample(Byte::new(0x00));
        dmc.output_unit_bits_remaining = 0;
        dmc.clock_output_unit();
        assert_eq!(
            dmc.dma_state.pending_dma_address,
            Some(Address::new(0x8000)),
            "address should wrap from 0xFFFF to 0x8000"
        );
    }
}
