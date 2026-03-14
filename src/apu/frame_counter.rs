#[derive(Debug, Default)]
pub struct FrameCounter {
    cycles: u16,
}

pub enum FrameSignal {
    None,
    QuarterFrame,
    HalfFrame,
}

impl FrameCounter {
    // 4-step mode (the default, controlled by $4017 bit 7 which isn't implemented yet).
    // Cycle counts are per CPU clock (1.789 MHz).
    pub fn tick(&mut self) -> FrameSignal {
        self.cycles += 1;

        let signal = match self.cycles {
            7_457 => FrameSignal::QuarterFrame,
            14_913 => FrameSignal::HalfFrame,
            22_371 => FrameSignal::QuarterFrame,
            29_829 => FrameSignal::HalfFrame,
            _ => FrameSignal::None,
        };

        if self.cycles >= 29_830 {
            self.cycles = 0;
        }

        signal
    }
}
