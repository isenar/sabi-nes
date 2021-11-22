use crate::Byte;

mod frame;
pub mod palettes;

pub use frame::Frame;

pub type Rgb = (Byte, Byte, Byte);
