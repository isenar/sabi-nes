use crate::Byte;
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct JoypadButton: Byte {
        const RIGHT             = 0b1000_0000;
        const LEFT              = 0b0100_0000;
        const DOWN              = 0b0010_0000;
        const UP                = 0b0001_0000;
        const START             = 0b0000_1000;
        const SELECT            = 0b0000_0100;
        const BUTTON_B          = 0b0000_0010;
        const BUTTON_A          = 0b0000_0001;
    }
}

#[derive(Debug, Default)]
pub struct Joypad {
    strobe_mode: bool,
    button_index: Byte,
    button_status: JoypadButton,
}

impl Joypad {
    pub fn read(&mut self) -> Byte {
        if self.button_index > 7 {
            return 1;
        }

        let response = (self.button_status.bits & (1 << self.button_index)) >> self.button_index;

        if !self.strobe_mode && self.button_index <= 7 {
            self.button_index += 1;
        }

        response
    }

    pub fn write(&mut self, value: Byte) {
        self.strobe_mode = value & 1 == 1;

        if self.strobe_mode {
            self.button_index = 0;
        }
    }

    pub fn press_button(&mut self, button: JoypadButton) {
        self.set_button_pressed_status(button, true)
    }

    pub fn release_button(&mut self, button: JoypadButton) {
        self.set_button_pressed_status(button, false)
    }

    fn set_button_pressed_status(&mut self, button: JoypadButton, pressed: bool) {
        self.button_status.set(button, pressed);
    }
}
