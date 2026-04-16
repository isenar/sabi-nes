use crate::Result;
use crate::input::joypad::Joypad;
use crate::render::Frame;

/// Trait for emulator frontends (rendering, input, timing, audio)
pub trait Frontend {
    /// Render a frame to the screen
    fn render_frame(&mut self, frame: &Frame) -> Result<()>;

    /// Handle input events and update the joypad state
    /// Returns false if the emulator should exit
    fn handle_input(&mut self, joypad: &mut Joypad) -> Result<bool>;

    /// Sleep to maintain the target frame rate
    fn frame_limit(&mut self);

    /// Push audio samples to the output device.
    /// Default implementation discards samples (no audio).
    fn queue_audio(&mut self, _samples: &[f32]) {}
}
