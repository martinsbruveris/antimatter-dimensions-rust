//! [`FormatOptions`] and the notation selector.
//!
//! These are the per-frame inputs that drive a single `format` call: the per-call
//! digit counts plus the exponent-display and "Infinite"-cutoff settings.
//!
//! See `docs/design/2026-06-25-number-formatting.md` ("What `FormatOptions` must
//! carry") for the full rationale.

use break_infinity::Decimal;

/// Which notation strategy to render with.
///
/// There are more formats we could be adding if we want...
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Notation {
    #[default]
    Scientific,
    Engineering,
    Standard,
    Letters,
    /// Standard below 1e33, Scientific above. The game's default notation.
    MixedScientific,
    /// Standard below 1e33, Engineering above.
    MixedEngineering,
    /// `e` followed by the base-10 logarithm of the value.
    Logarithm,
    /// The value counted in multiples of the float `MAX_VALUE` (`∞`-suffixed).
    Infinity,
}

/// Controls how a *large exponent* is itself rendered (plain, comma-grouped, or
/// recursively in-notation — it governs more than just commas).
///
/// An exponent below `min` is printed plain; below `max` (when `show`) it is
/// comma-grouped; at or above `max` it is recursively formatted in notation. In the
/// game `min`/`max` come from the player's notation-digits option
/// (`min = 10**comma`, `max = 10**notation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExponentDisplay {
    pub show: bool,
    pub min: i64,
    pub max: i64,
}

impl Default for ExponentDisplay {
    fn default() -> Self {
        // Library defaults: show, min 1e5, max 1e9.
        ExponentDisplay {
            show: true,
            min: 100_000,
            max: 1_000_000_000,
        }
    }
}

/// Everything `format` needs beyond the value itself.
///
/// This is **caller / UI state**, never part of `GameState`. `places`,
/// `places_under_1000` and `places_exponent` are per-call digit counts;
/// `exponent_display` and `inf_threshold` are per-frame settings the caller derives
/// from user options / game state.
#[derive(Clone, Debug, PartialEq)]
pub struct FormatOptions {
    /// Notation strategy (scientific, engineering, etc.).
    pub notation: Notation,
    /// Mantissa decimal places for numbers with |exponent| >= 3. Non-negativity is a
    /// type invariant (`u32`), so there is no "unspecified" sentinel to guard against.
    pub places: u32,
    /// Decimal places for numbers with |exponent| < 3 (and very-small values).
    pub places_under_1000: u32,
    /// Decimal places for the exponent once it is itself large enough to be in
    /// notation (e.g. `1e1.234e15`). The game hardcodes this to 3.
    pub places_exponent: u32,
    /// How a large exponent is rendered (plain / commas / recursive notation).
    pub exponent_display: ExponentDisplay,
    /// If `Some(t)`, any value with `|value| >= t` renders as `"Infinite"`. `None`
    /// (the default) never shows "Infinite". The caller derives this from game state
    /// (`Some(NUMBER_MAX_VALUE)` pre-break, `None` post-break); `format` only
    /// compares the value it is handed and never reads `GameState`.
    pub inf_threshold: Option<Decimal>,
}

impl Default for FormatOptions {
    fn default() -> Self {
        // Defaults match how `ad-gui` currently renders (Scientific, 2 mantissa
        // places), so a later A -> C swap stays low-risk.
        FormatOptions {
            notation: Notation::default(),
            places: 2,
            places_under_1000: 0,
            places_exponent: 3,
            exponent_display: ExponentDisplay::default(),
            inf_threshold: None,
        }
    }
}

impl FormatOptions {
    /// Default options with a specific notation selected.
    pub fn new(notation: Notation) -> Self {
        FormatOptions {
            notation,
            ..FormatOptions::default()
        }
    }
}
