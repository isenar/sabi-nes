//! 7  bit  0
//! ---- ----
//! VSO. ....
//! |||| ||||
//! |||+-++++- Least significant bits previously written into a PPU register
//! |||        (due to register not being updated for this address)
//! ||+------- Sprite overflow. The intent was for this flag to be set
//! ||         whenever more than eight sprites appear on a scanline, but a
//! ||         hardware bug causes the actual behavior to be more complicated
//! ||         and generate false positives as well as false negatives; see
//! ||         PPU sprite evaluation. This flag is set during sprite
//! ||         evaluation and cleared at dot 1 (the second dot) of the
//! ||         pre-render line.
//! |+-------- Sprite 0 Hit.  Set when a nonzero pixel of sprite 0 overlaps
//! |          a nonzero background pixel; cleared at dot 1 of the pre-render
//! |          line.  Used for raster timing.
//! +--------- Vertical blank has started (0: not in vblank; 1: in vblank).
//!            Set at dot 1 of line 241 (the line *after* the post-render
//!            line); cleared after reading $2002 and at dot 1 of the
//!            pre-render line.

use crate::Byte;
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct StatusRegister: Byte {
        const UNUSED1         = 0b0000_0001;
        const UNUSED2         = 0b0000_0010;
        const UNUSED3         = 0b0000_0100;
        const UNUSED4         = 0b0000_1000;
        const UNUSED5         = 0b0001_0000;
        const SPRITE_OVERFLOW = 0b0010_0000;
        const SPRITE_ZERO_HIT = 0b0100_0000;
        const VBLANK_STARTED  = 0b1000_0000;
    }
}
