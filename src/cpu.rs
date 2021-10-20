#[derive(Debug, Default)]
pub struct Cpu {
    pub register_a: u8,
    pub register_x: u8,
    pub status: u8,
    pub program_counter: u32,
}

impl Cpu {
    #[allow(unused)]
    pub fn interpret(&mut self, data: &[u8]) {
        self.program_counter = 0;

        loop {
            let opcode = data[self.program_counter as usize];
            self.program_counter += 1;

            match opcode {
                0x00 => {
                    // BRK
                    return;
                }
                0xA9 => {
                    // LDA -> save param to register A
                    let lda_param = data[self.program_counter as usize];
                    self.program_counter += 1;

                    self.lda(lda_param);
                }
                0xAA => {
                    // TAX -> save register A value to register X
                    self.tax()
                }

                0xE8 => {
                    // INX -> increment register X
                    self.inx()
                }

                _ => todo!(),
            }
        }
    }

    fn lda(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.status |= 0b0000_0010;
        } else {
            self.status &= 0b1111_1101;
        }

        if result & 0b1000_0000 != 0 {
            self.status |= 0b1000_0000;
        } else {
            self.status &= 0b0111_1111;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod lda {
        use super::*;

        #[test]
        fn immediate_load() {
            let mut cpu = Cpu::default();
            let data = vec![0xa9, 0x05, 0x00];

            cpu.interpret(&data);

            assert_eq!(cpu.register_a, 0x05);
            assert_eq!(cpu.status & 0b000_0010, 0b00);
            assert_eq!(cpu.status & 0b1000_0000, 0b00)
        }

        #[test]
        fn zero_flag() {
            let mut cpu = Cpu::default();
            let data = vec![0xa9, 0x00, 0x00];

            cpu.interpret(&data);

            assert_eq!(cpu.status & 0b000_0010, 0b10);
        }
    }

    mod tax {
        use super::*;

        #[test]
        fn moves_reg_a_value_to_reg_x() {
            let mut cpu = Cpu {
                register_a: 10,
                ..Cpu::default()
            };
            let data = vec![0xaa, 0x00];

            cpu.interpret(&data);

            assert_eq!(cpu.register_x, 10);
        }
    }

    mod inx {
        use super::*;

        #[test]
        fn inx_overflow() {
            let mut cpu = Cpu {
                register_x: 0xff,
                ..Default::default()
            };
            let data = vec![0xe8, 0xe8, 0x00];

            cpu.interpret(&data);

            assert_eq!(cpu.register_x, 1);
        }
    }

    mod mixed {
        use super::*;

        #[test]
        fn simple_5_ops_working_together() {
            let mut cpu = Cpu::default();
            let data = vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00];

            cpu.interpret(&data);

            assert_eq!(cpu.register_x, 0xc1);
        }
    }
}
