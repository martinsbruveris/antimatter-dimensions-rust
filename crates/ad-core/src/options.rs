//! Player options (UI/UX preferences).
//!
//! These mirror the original game's `player.options` object. The engine itself
//! is indifferent to almost all of them — they configure the frontend — but
//! they live in `GameState` so that a save file produced from a fresh game is
//! valid and so that options round-trip unchanged when a save is loaded, the
//! engine is run, and the save is written out again. Defaults match the
//! original game (`src/core/player.js`).
//!
//! Only the subset that is currently surfaced in the UI is modelled. More
//! fields are added as the corresponding options tabs are implemented.

/// Default game-loop cadence in milliseconds (original `updateRate: 33`).
pub const DEFAULT_UPDATE_RATE_MS: u32 = 33;
/// Slider bounds for the update rate, matching the original (33–200 ms).
pub const MIN_UPDATE_RATE_MS: u32 = 33;
pub const MAX_UPDATE_RATE_MS: u32 = 200;

/// The notation names the frontend can render (subset of the original's ~22).
/// These are the display names; the `ad-format` WASM matches them case-insensitively.
pub const NOTATIONS: [&str; 4] = ["Scientific", "Engineering", "Standard", "Letters"];
/// Default notation. The original defaults to "Mixed scientific" (not yet ported);
/// until then we default to "Standard".
pub const DEFAULT_NOTATION: &str = "Standard";

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Options {
    /// Whether keyboard shortcuts are active (original `hotkeys`).
    pub hotkeys: bool,
    /// Game-loop cadence in milliseconds (original `updateRate`). The frontend
    /// only ticks the engine once this much wall-clock time has elapsed, so a
    /// larger value means coarser, less frequent updates.
    pub update_rate: u32,
    /// Active number-formatting notation (original `notation`). Display name from
    /// [`NOTATIONS`]; the frontend hands it to the `ad-format` WASM formatter.
    pub notation: String,
}

impl Options {
    pub fn new() -> Self {
        Self {
            hotkeys: true,
            update_rate: DEFAULT_UPDATE_RATE_MS,
            notation: DEFAULT_NOTATION.to_string(),
        }
    }

    /// Set the update rate, clamped to the original game's slider range.
    pub fn set_update_rate(&mut self, rate: u32) {
        self.update_rate = rate.clamp(MIN_UPDATE_RATE_MS, MAX_UPDATE_RATE_MS);
    }

    /// Set the notation, ignoring any name not in [`NOTATIONS`].
    pub fn set_notation(&mut self, notation: &str) {
        if NOTATIONS.contains(&notation) {
            self.notation = notation.to_string();
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}
