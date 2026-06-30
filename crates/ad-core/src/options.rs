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

/// Slider bounds for the exponent-notation digit thresholds, matching the
/// original's Exponent Notation modal (3–15 digits).
pub const MIN_NOTATION_DIGITS: u32 = 3;
pub const MAX_NOTATION_DIGITS: u32 = 15;
/// Defaults for the two thresholds (original `notationDigits: { comma: 5, notation: 9 }`):
/// the exponent gets commas at 10^comma and switches to in-notation at 10^notation.
pub const DEFAULT_NOTATION_DIGITS_COMMA: u32 = 5;
pub const DEFAULT_NOTATION_DIGITS_NOTATION: u32 = 9;

/// Offline tick budget: across how many discrete ticks is an offline interval being
/// replayed (the resolution dial). Default matches the original (`offlineTicks:
/// 1e5`). Our slider range diverges from the original's (500..1e6): we run
/// 10K..=10M, exploiting the faster engine. See
/// `design-docs/2026-06-30-offline-progress.md`.
pub const DEFAULT_OFFLINE_TICKS: u32 = 100_000;
pub const MIN_OFFLINE_TICKS: u32 = 10_000;
pub const MAX_OFFLINE_TICKS: u32 = 10_000_000;

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
    /// Exponent digit count at/above which the exponent is comma-grouped
    /// (original `notationDigits.comma`); the threshold is 10^this.
    pub notation_digits_comma: u32,
    /// Exponent digit count at/above which the exponent is itself rendered in
    /// notation (original `notationDigits.notation`); the threshold is 10^this.
    /// Always `>= notation_digits_comma`.
    pub notation_digits_notation: u32,
    /// Offline replay resolution (original `offlineTicks`): the maximum number of
    /// discrete ticks an offline interval is spread across. Higher = finer.
    pub offline_ticks: u32,
}

impl Options {
    pub fn new() -> Self {
        Self {
            hotkeys: true,
            update_rate: DEFAULT_UPDATE_RATE_MS,
            notation: DEFAULT_NOTATION.to_string(),
            notation_digits_comma: DEFAULT_NOTATION_DIGITS_COMMA,
            notation_digits_notation: DEFAULT_NOTATION_DIGITS_NOTATION,
            offline_ticks: DEFAULT_OFFLINE_TICKS,
        }
    }

    /// Set the offline tick budget. Any positive value is accepted **as-is** —
    /// we deliberately do not clamp to the slider's 10K..=10M range, so a value
    /// from an imported save (including the original's out-of-range values) is
    /// preserved. A zero falls back to 1 (the budget is always at least one tick).
    pub fn set_offline_ticks(&mut self, ticks: u32) {
        self.offline_ticks = ticks.max(1);
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

    /// Set the exponent-notation digit thresholds. Each is clamped to the
    /// [3, 15] slider range, and the notation threshold is kept `>=` the comma
    /// threshold (original NotationModal invariant).
    pub fn set_notation_digits(&mut self, comma: u32, notation: u32) {
        let comma = comma.clamp(MIN_NOTATION_DIGITS, MAX_NOTATION_DIGITS);
        let notation = notation.clamp(MIN_NOTATION_DIGITS, MAX_NOTATION_DIGITS);
        self.notation_digits_comma = comma;
        self.notation_digits_notation = notation.max(comma);
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notation_digits_clamp_to_range() {
        let mut o = Options::new();
        o.set_notation_digits(0, 99);
        assert_eq!(o.notation_digits_comma, MIN_NOTATION_DIGITS);
        assert_eq!(o.notation_digits_notation, MAX_NOTATION_DIGITS);
    }

    #[test]
    fn notation_threshold_stays_at_least_comma() {
        let mut o = Options::new();
        // A notation threshold below the comma threshold is raised to match.
        o.set_notation_digits(10, 4);
        assert_eq!(o.notation_digits_comma, 10);
        assert_eq!(o.notation_digits_notation, 10);
    }

    #[test]
    fn offline_ticks_accepts_any_positive_value() {
        let mut o = Options::new();
        assert_eq!(o.offline_ticks, DEFAULT_OFFLINE_TICKS);

        // Values outside our slider range are kept (we diverge from the original
        // and accept imported values as-is).
        o.set_offline_ticks(500);
        assert_eq!(o.offline_ticks, 500);
        o.set_offline_ticks(50_000_000);
        assert_eq!(o.offline_ticks, 50_000_000);

        // Zero falls back to a single tick.
        o.set_offline_ticks(0);
        assert_eq!(o.offline_ticks, 1);
    }
}
