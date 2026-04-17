use crate::Byte;
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
    pub struct JoypadButton: u8 {
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
    button_index: usize,
    button_status: JoypadButton,
}

impl Joypad {
    pub fn read(&mut self) -> Byte {
        if self.button_index > 7 {
            return 1.into();
        }

        let button_status = Byte::new(self.button_status.bits());
        let response = (button_status & (Byte::new(1) << self.button_index)) >> self.button_index;

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
        self.set_button_pressed_status(button, true);
    }

    pub fn release_button(&mut self, button: JoypadButton) {
        self.set_button_pressed_status(button, false);
    }

    pub fn set_all_buttons(&mut self, buttons: JoypadButton) {
        self.button_status = buttons;
    }

    fn set_button_pressed_status(&mut self, button: JoypadButton, pressed: bool) {
        self.button_status.set(button, pressed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_all_buttons_replaces_entire_state() {
        let mut joypad = Joypad::default();
        joypad.press_button(JoypadButton::BUTTON_A);
        joypad.set_all_buttons(JoypadButton::UP | JoypadButton::DOWN);
        assert_eq!(joypad.button_status, JoypadButton::UP | JoypadButton::DOWN);
    }
}
