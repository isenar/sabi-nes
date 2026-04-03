use crate::Byte;

// 1_048_576 CPU cycles * 3 = 3_145_728 PPU cycles is about 586ms
// at 1.789 MHz (NTSC)
const DECAY_PPU_CYCLES: usize = 3_145_728;

#[derive(Debug)]
pub struct OpenBus {
    value: Byte,
    cycle_written_at: usize,
}

impl OpenBus {
    pub const fn new() -> Self {
        Self {
            value: Byte::new(0x00),
            cycle_written_at: 0,
        }
    }

    /// Returns the PPU open bus value, or 0 if it has fully decayed.
    /// Real hardware capacitors discharge over ~600 ms; we model the full
    /// byte as decayed after roughly one second of PPU cycles.
    pub const fn read(&self, current_cycle: usize) -> Byte {
        if current_cycle.saturating_sub(self.cycle_written_at) >= DECAY_PPU_CYCLES {
            Byte::new(0x00)
        } else {
            self.value
        }
    }

    pub const fn write(&mut self, value: Byte, current_cycle: usize) {
        self.value = value;
        self.cycle_written_at = current_cycle;
    }
}
