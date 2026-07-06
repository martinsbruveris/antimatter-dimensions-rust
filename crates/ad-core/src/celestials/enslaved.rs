//! Enslaved — The Nameless Ones (Feature 7.3) — time storage/release and a
//! restrictive Reality. See `docs/design/2026-07-06-celestials.md` §3.
//!
//! **Work in progress:** the state block + run flag land here so save/load and
//! the shared celestial machinery compile; the full time-storage/release and
//! run-restriction logic is filled in by its own task.

use crate::state::GameState;

/// `Enslaved.glyphLevelMin` — inside the run glyph levels are boosted to at
/// least this (`getAdjustedGlyphLevel`).
pub const GLYPH_LEVEL_MIN: u32 = 5000;

/// `player.celestials.enslaved`.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EnslavedState {
    /// Whether game-time storage is active (`isStoring`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_storing: bool,
    /// Stored game time in ms (`stored`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub stored: f64,
    /// Whether real-time storage is active (`isStoringReal`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_storing_real: bool,
    /// Stored real time in ms (`storedReal`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub stored_real: f64,
    /// Unlock bits (ids 0/1, stored as a bitset of the original's `unlocks`
    /// array).
    #[cfg_attr(feature = "serde", serde(default))]
    pub unlock_bits: u32,
    /// Whether Enslaved's Reality is running (`run`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub run: bool,
    /// Whether Enslaved's Reality has been completed (`completed`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub completed: bool,
    /// Tesseracts bought (`tesseracts`), raising the Infinity-Dimension cap.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tesseracts: u32,
}

impl EnslavedState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether unlock `id` is owned.
    pub fn unlock_bought(&self, id: u8) -> bool {
        self.unlock_bits & (1u32 << id) != 0
    }
}

impl GameState {
    /// Whether Enslaved's Reality is unlocked (`RUN`, id 1). Stub until the
    /// Enslaved task wires the time-cost unlock + glyph requirement.
    pub fn enslaved_run_unlocked(&self) -> bool {
        self.celestials.enslaved.unlock_bought(1)
    }
}
