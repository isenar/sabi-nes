# APU Channels Refactor Design

**Date:** 2026-03-21
**Branch:** apu-channels
**Approach:** B — Extract `Envelope` + clean up everything

---

## Goal

Simplify the APU channel implementations by eliminating copy-pasted code, extracting the shared `Envelope` sub-component into its own type, and fixing minor naming/visibility issues. No behaviour change.

---

## Architecture

### New files

| File | Purpose |
|------|---------|
| `src/apu/channels/common.rs` | Shared `LENGTH_TABLE` constant (`pub(super)`) |
| `src/apu/channels/envelope.rs` | `Envelope` struct with `clock`, `restart`, `decay_level` |

### Changed files

| File | Changes |
|------|---------|
| `src/apu/channels.rs` | Add `mod common; mod envelope;` |
| `src/apu/channels/square_channel.rs` | Use `Envelope`, remove 3 envelope fields, use shared `LENGTH_TABLE` |
| `src/apu/channels/noise_channel.rs` | Use `Envelope`, remove 3 envelope fields, use shared `LENGTH_TABLE`, fix visibility |
| `src/apu/channels/triangle_channel.rs` | Use shared `LENGTH_TABLE`, fix duplicate method, fix visibility |
| `src/apu/channels/dmc.rs` | Rename `direct_load` field to `direct_load_reg` |

---

## Components

### `Envelope` (`src/apu/channels/envelope.rs`)

Encapsulates the NES envelope generator that is shared between the square and noise channels.

**State:**
- `start_flag: bool` — set on note trigger; causes full reset on next clock
- `divider: u8` — counts down from the period, clocks the decay counter
- `decay: u8` — the current volume level (0–15)

**Interface:**
```rust
impl Envelope {
    pub fn restart(&mut self)
    pub fn clock(&mut self, period: u8, looping: bool)
    pub fn decay_level(&self) -> u8
}
```

- `restart()` — sets `start_flag`; called when the channel's fourth register (`$4003`/`$400F`) is written
- `clock(period, looping)` — called every quarter-frame (~240 Hz). `period` is the 4-bit value from bits 3–0 of the volume register. `looping` is the length-counter-halt/envelope-loop bit (bit 5 of the volume register). On start: decay resets to 15, divider reloads. On divider expire: decay decrements (wraps to 15 if `looping`).
- `decay_level()` — returns the current decay value for use in `output()`

**Channel usage:**
```rust
// clock_envelope in SquareChannel / NoiseChannel becomes:
pub fn clock_envelope(&mut self) {
    self.envelope.clock(self.volume_period(), self.is_length_counter_halted());
}

// output() uses:
if self.is_constant_volume() { self.volume_period() } else { self.envelope.decay_level() }
```

### Shared `LENGTH_TABLE` (`src/apu/channels/common.rs`)

```rust
pub(super) const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];
```

All three channel files use `use super::common::LENGTH_TABLE;` and drop their own copy.

---

## Fixes

### `TriangleChannel`
- Remove `is_control_flag_set` (private, duplicates `is_linear_counter_enabled`)
- Rename the surviving method to `is_control_flag()` and make it private
- `is_linear_counter_enabled()` (public) stays as-is — it's a register accessor used externally
- Make `timer()`, `counter_reload()`, `length_counter_load()` `pub(crate)` — they are only needed by the APU bus layer, not arbitrary external code

### `NoiseChannel`
- Make `is_length_counter_halted()`, `is_constant_volume()`, `volume_divider_period()`, `mode()`, `timer_period()` `pub(crate)` — same rationale

### `Dmc`
- Rename field `direct_load: Byte` → `direct_load_reg: Byte` to resolve the method/field name collision with `fn direct_load(self) -> Byte`

### `SquareChannel`
- Remove `#[allow(unused)]` from the impl block

---

## Testing

### `envelope.rs` tests (new)
- **`restart_resets_decay_to_15`** — after `restart()` + one `clock()`, `decay_level()` == 15
- **`divider_counts_down_before_decrementing_decay`** — decay doesn't change until divider expires
- **`decay_decrements_on_divider_expiry`** — decay goes from 15 to 14 after `period+1` clocks
- **`decay_holds_at_zero_when_not_looping`** — non-looping envelope stays at 0
- **`decay_wraps_to_15_when_looping`** — looping envelope wraps back to 15 from 0
- **`period_reloads_after_decay_step`** — divider reloads to period after each decay step

### `square_channel.rs` additions
- **`output_uses_envelope_when_not_constant_volume`** — verify envelope decay appears in output
- **`output_uses_constant_volume`** — when constant-volume bit set, output equals period value
- **`output_muted_when_sweep_target_overflows`** — period + shift exceeds 0x7FF → output == 0

### `noise_channel.rs` additions
- **`output_uses_envelope_decay`** — verify envelope decay appears in output (non-constant-volume mode)

---

## Non-Goals

- No changes to `FrameCounter`
- No `LengthCounter` extraction (kept simple inline logic per channel)
- No `Sweep` or `LinearCounter` extraction
- No DMC playback implementation
- No changes to `Apu` or the bus layer
