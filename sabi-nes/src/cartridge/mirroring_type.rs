#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirroringType {
    /// Horizontal arrangement
    Horizontal,
    /// Vertical arrangement
    Vertical,
    /// Four screen VRAM
    FourScreen,
}

impl MirroringType {
    pub fn new(is_four_screen: bool, is_vertical: bool) -> Self {
        match (is_four_screen, is_vertical) {
            (true, _) => Self::FourScreen,
            (false, true) => Self::Vertical,
            (false, false) => Self::Horizontal,
        }
    }
}
