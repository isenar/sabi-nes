use crate::Byte;
use crate::bus::DmaOperation;
use crate::utils::NthBit;

#[derive(Debug, Default)]
pub struct FrameCounter {
    cycles: u16,
    mode: SequencerMode,
    irq: IrqState,
    pending_irq_clear: bool,
    reset_delay: Byte, // countdown to reset; 0 = no pending reset
}

#[derive(Debug, Default, PartialEq)]
enum SequencerMode {
    #[default]
    FourStep,
    FiveStep,
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
    /// Bit 7: mode (0 = 4-step, 1 = 5-step).
    /// Bit 6: IRQ inhibit — when set, clears any pending IRQ.
    ///
    /// Any write here resets the sequencer to 0. If bit 7 is set, a half-frame
    /// signal is generated immediately (clocking length counters and envelopes).
    pub fn write(&mut self, value: Byte, dma_operation: DmaOperation) -> Option<FrameSignal> {
        self.mode = if value.nth_bit::<7>() {
            SequencerMode::FiveStep
        } else {
            SequencerMode::FourStep
        };
        self.irq.is_inhibited = value.nth_bit::<6>();
        if self.irq.is_inhibited {
            self.force_clear_irq();
        }
        // Schedule delayed reset.
        // NOTE: On real hardware a $4017 write delays the frame counter reset
        //       by 3 CPU cycles on a PUT (odd) cycle or 4 CPU cycles on a GET
        //       (even) cycle.
        //       In our emulation model read_byte() is called *before* the
        //       cycle's tick_one(), so the read of $4015 observes the APU state
        //       before the current tick.  We compensate by counting one extra
        //       tick in the delay (4 for PUT, 5 for GET) so that the boundary
        //       cycles visible to software match the hardware specification.
        self.reset_delay = dma_operation.reset_delay();

        (self.mode == SequencerMode::FiveStep).then_some(FrameSignal::HalfFrame)
    }

    /// Returns true if the frame counter IRQ is currently pending.
    pub fn is_irq_pending(&self) -> bool {
        self.irq.is_pending
    }

    /// Schedule a deferred IRQ clear (called when $4015 is read).
    /// The actual clear happens on the next PUT (odd) CPU cycle.
    pub fn clear_irq(&mut self) {
        self.pending_irq_clear = true;
    }

    /// Immediately clear the IRQ flag and cancel any pending deferred clear.
    /// Used by the $4017 IRQ inhibit write.
    pub fn force_clear_irq(&mut self) {
        self.irq.is_pending = false;
        self.pending_irq_clear = false;
    }

    pub fn tick(&mut self, dma_operation: DmaOperation) -> Option<FrameSignal> {
        if dma_operation.is_put() && self.pending_irq_clear {
            self.irq.is_pending = false;
            self.pending_irq_clear = false;
        }

        // Apply delayed frame counter reset before incrementing cycles.
        if self.reset_delay > 0 {
            self.reset_delay -= 1;
            if self.reset_delay == 0 {
                self.cycles = 0;
            }
        }

        self.cycles += 1;

        match self.mode {
            SequencerMode::FourStep => {
                if !self.irq.is_inhibited
                    && (self.cycles == 29_828 || self.cycles == 29_829 || self.cycles == 29_830)
                {
                    self.irq.is_pending = true;
                }

                match self.cycles {
                    7_457 | 22_371 => Some(FrameSignal::QuarterFrame),
                    14_913 | 29_829 => Some(FrameSignal::HalfFrame),
                    29_830.. => {
                        self.cycles = 0;
                        None
                    }
                    _ => None,
                }
            }
            SequencerMode::FiveStep => match self.cycles {
                7_457 | 22_371 => Some(FrameSignal::QuarterFrame),
                14_913 | 29_829 | 37_281 => Some(FrameSignal::HalfFrame),
                37_282.. => {
                    self.cycles = 0;
                    None
                }
                _ => None,
            },
        }
    }
}
