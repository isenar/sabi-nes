use sabi_nes::Result;
use sabi_nes::input::joypad::Joypad;
use sabi_nes::render::Frame;

/// Trait for emulator frontends (rendering, input, timing)
pub trait Frontend {
    /// Render a frame to the screen
    fn render_frame(&mut self, frame: &Frame) -> Result<()>;

    /// Handle input events and update the joypad state
    /// Returns false if the emulator should exit
    fn handle_input(&mut self, joypad: &mut Joypad) -> Result<bool>;

    /// Sleep to maintain the target frame rate
    fn frame_limit(&mut self);
}
