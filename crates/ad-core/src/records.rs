//! Time and prestige records — the slice of the original `player.records` that
//! Phase 1–2 needs.
//!
//! The original keeps a large records tree spanning every prestige layer; we
//! model only what current features read: total time played, the current
//! infinity's elapsed time and peak antimatter, and the fastest infinity. These
//! back the Infinity-Point formula's future break-branch (`this_infinity.max_am`),
//! the "fastest infinity" statistic (`best_infinity`), and the time-based Infinity
//! Upgrades to come (`total_time_played_ms`, `this_infinity.time_ms`).
//!
//! Times are milliseconds. Pre-Infinity the game-speed multiplier is 1, so game
//! time and real time advance together; they are kept as separate fields anyway to
//! match the original and to stay correct once game-speed effects (Black Holes,
//! dilation) land. See `design-docs/2026-07-02-infinity-points-and-records.md`.
//!
//! Note: the all-time `total_antimatter` record lives directly on [`GameState`]
//! (it predates this module and is wired through many call sites); only the
//! time/infinity records live here.

use break_infinity::Decimal;

/// Records for the current (in-progress) infinity. Reset by a Big Crunch.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThisInfinity {
    /// Game time elapsed in this infinity (ms). Pre-Infinity equals `real_time_ms`.
    pub time_ms: f64,
    /// Real (wall-clock-equivalent) time elapsed in this infinity (ms).
    pub real_time_ms: f64,
    /// Maximum antimatter reached during this infinity. Mirrors
    /// `player.records.thisInfinity.maxAM`; drives the post-break IP formula.
    pub max_am: Decimal,
}

impl ThisInfinity {
    pub fn new() -> Self {
        Self {
            time_ms: 0.0,
            real_time_ms: 0.0,
            max_am: Decimal::ZERO,
        }
    }
}

impl Default for ThisInfinity {
    fn default() -> Self {
        Self::new()
    }
}

/// Records for the fastest infinity performed. Persists across a Big Crunch.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BestInfinity {
    /// Fastest infinity by game time (ms). `f64::MAX` means "no infinity yet"
    /// (the original's `Number.MAX_VALUE` sentinel; `f64::MAX` equals it).
    pub time_ms: f64,
    /// Fastest infinity by real time (ms). Same `f64::MAX` sentinel.
    pub real_time_ms: f64,
}

impl BestInfinity {
    pub fn new() -> Self {
        Self {
            time_ms: f64::MAX,
            real_time_ms: f64::MAX,
        }
    }
}

impl Default for BestInfinity {
    fn default() -> Self {
        Self::new()
    }
}

/// The modelled slice of `player.records`: time played plus infinity timing.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Records {
    /// Total game time ever played (ms), monotonic. Mirrors
    /// `player.records.totalTimePlayed`.
    pub total_time_played_ms: f64,
    /// Total real time ever played (ms), monotonic. Mirrors
    /// `player.records.realTimePlayed`.
    pub real_time_played_ms: f64,
    /// The current infinity's records (reset on crunch).
    pub this_infinity: ThisInfinity,
    /// The fastest infinity's records (kept on crunch).
    pub best_infinity: BestInfinity,
}

impl Records {
    pub fn new() -> Self {
        Self {
            total_time_played_ms: 0.0,
            real_time_played_ms: 0.0,
            this_infinity: ThisInfinity::new(),
            best_infinity: BestInfinity::new(),
        }
    }
}

impl Default for Records {
    fn default() -> Self {
        Self::new()
    }
}
