use sabi_nes_core::Rom;
use sabi_nes_core::input::joypad::JoypadButton;
use sabi_nes_core::render::Frame;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

pub(crate) fn key_code_to_button(key: u8) -> Option<JoypadButton> {
    match key {
        0 => Some(JoypadButton::BUTTON_A),
        1 => Some(JoypadButton::BUTTON_B),
        2 => Some(JoypadButton::SELECT),
        3 => Some(JoypadButton::START),
        4 => Some(JoypadButton::UP),
        5 => Some(JoypadButton::DOWN),
        6 => Some(JoypadButton::LEFT),
        7 => Some(JoypadButton::RIGHT),
        _ => None,
    }
}

pub(crate) fn rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(rgb.len() / 3 * 4);
    for chunk in rgb.chunks_exact(3) {
        rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
    }
    rgba
}

#[wasm_bindgen]
pub struct WasmEmulator {
    context: CanvasRenderingContext2d,
    emulator: Option<sabi_nes_core::Emulator>,
    keys: JoypadButton,
}

#[wasm_bindgen]
impl WasmEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str) -> Result<WasmEmulator, JsValue> {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Debug);
        let document = web_sys::window()
            .ok_or("no window")?
            .document()
            .ok_or("no document")?;
        let canvas: HtmlCanvasElement = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| format!("no element with id '{canvas_id}'"))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("element is not a canvas"))?;
        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")
            .map_err(|e| format!("{e:?}"))?
            .ok_or("could not get 2d context")?
            .dyn_into()
            .map_err(|_| JsValue::from_str("context is not 2d"))?;
        Ok(Self {
            context: ctx,
            emulator: None,
            keys: JoypadButton::empty(),
        })
    }

    pub fn load_rom(&mut self, data: &[u8]) -> Result<(), JsValue> {
        self.emulator = None;
        let rom = Rom::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.emulator = Some(
            sabi_nes_core::Emulator::new(rom)
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
        );
        Ok(())
    }

    pub fn step_frame(&mut self) -> Result<(), JsValue> {
        let Some(emu) = &mut self.emulator else {
            return Ok(());
        };
        emu.set_joypad(self.keys);
        emu.step_frame()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let rgba = rgb_to_rgba(emu.frame().pixel_data());
        let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&rgba),
            Frame::WIDTH as u32,
            Frame::HEIGHT as u32,
        )
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
        self.context
            .put_image_data(&image_data, 0.0, 0.0)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
        let _ = emu.drain_audio();
        Ok(())
    }

    pub fn key_down(&mut self, key: u8) {
        if let Some(button) = key_code_to_button(key) {
            self.keys.insert(button);
        }
    }

    pub fn key_up(&mut self, key: u8) {
        if let Some(button) = key_code_to_button(key) {
            self.keys.remove(button);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_code_maps_to_all_buttons() {
        assert_eq!(key_code_to_button(0), Some(JoypadButton::BUTTON_A));
        assert_eq!(key_code_to_button(1), Some(JoypadButton::BUTTON_B));
        assert_eq!(key_code_to_button(2), Some(JoypadButton::SELECT));
        assert_eq!(key_code_to_button(3), Some(JoypadButton::START));
        assert_eq!(key_code_to_button(4), Some(JoypadButton::UP));
        assert_eq!(key_code_to_button(5), Some(JoypadButton::DOWN));
        assert_eq!(key_code_to_button(6), Some(JoypadButton::LEFT));
        assert_eq!(key_code_to_button(7), Some(JoypadButton::RIGHT));
        assert_eq!(key_code_to_button(8), None);
    }

    #[test]
    fn rgb_to_rgba_adds_opaque_alpha() {
        let rgb = [255u8, 0, 0, 0, 255, 0]; // red pixel, green pixel
        assert_eq!(rgb_to_rgba(&rgb), vec![255, 0, 0, 255, 0, 255, 0, 255]);
    }

    #[test]
    fn rgb_to_rgba_empty_input() {
        assert_eq!(rgb_to_rgba(&[]), Vec::<u8>::new());
    }
}
