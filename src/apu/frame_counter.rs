#[derive(Debug, Default)]
pub struct FrameCounter {
    cycles: u16,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FrameSignal {
    QuarterFrame,
    HalfFrame,
}

impl FrameCounter {
    // 4-step mode (the default, controlled by $4017 bit 7 which isn't implemented yet).
    // Cycle counts are per CPU clock (1.789 MHz).
    pub fn tick(&mut self) -> Option<FrameSignal> {
        self.cycles += 1;

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
