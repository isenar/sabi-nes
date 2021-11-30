use crate::Byte;
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct JoypadButton: Byte {
        const RIGHT             = 0b10000000;
        const LEFT              = 0b01000000;
        const DOWN              = 0b00100000;
        const UP                = 0b00010000;
        const START             = 0b00001000;
        const SELECT            = 0b00000100;
        const BUTTON_B          = 0b00000010;
        const BUTTON_A          = 0b00000001;
    }
}

#[derive(Debug)]
pub struct Joypad {
    strobe_mode: bool,
    button_index: Byte,
    button_status: JoypadButton,
}

impl Default for Joypad {
    fn default() -> Self {
        Self {
            strobe_mode: false,
            button_index: 0,
            button_status: JoypadButton::default(),
        }
    }
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

    pub fn set_button_pressed_status(&mut self, button: JoypadButton, pressed: bool) {
        self.button_status.set(button, pressed);
    }
}
