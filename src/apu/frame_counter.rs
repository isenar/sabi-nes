use crate::Byte;

#[derive(Debug, Default)]
pub struct FrameCounter {
    cycles: u16,
    irq: IrqState,
}

#[derive(Debug, Default)]
struct IrqState {
    is_inhibited: bool,
    is_pending: bool,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FrameSignal {
    QuarterFrame,
    HalfFrame,
}

impl FrameCounter {
    /// Handle a write to $4017.
    /// Bit 7: mode (0 = 4-step, 1 = 5-step; 5-step not yet implemented).
    /// Bit 6: IRQ inhibit — when set, clears any pending IRQ.
    pub fn write(&mut self, value: Byte) {
        self.irq.is_inhibited = value & 0x40 != 0;
        if self.irq.is_inhibited {
            self.irq.is_pending = false;
        }
    }

    /// Returns true if the frame counter IRQ is currently pending.
    pub fn is_irq_pending(&self) -> bool {
        self.irq.is_pending
    }

    /// Clear the frame counter IRQ flag (called when $4015 is read).
    pub fn clear_irq(&mut self) {
        self.irq.is_pending = false;
    }

    // 4-step mode (the default, controlled by $4017 bit 7 which isn't implemented yet).
    // Cycle counts are per CPU clock (1.789 MHz).
    pub fn tick(&mut self) -> Option<FrameSignal> {
        self.cycles += 1;

        // The IRQ flag is set at cycles 29,829 and 29,830.
        if !self.irq.is_inhibited && (self.cycles == 29_829 || self.cycles == 29_830) {
            self.irq.is_pending = true;
        }

        let signal = match self.cycles {
            7_457 | 22_371 => Some(FrameSignal::QuarterFrame),
            14_913 | 29_829 => Some(FrameSignal::HalfFrame),
            _ => None,
        };

        if self.cycles >= 29_830 {
            self.cycles = 0;
        }

        signal
    }
}
