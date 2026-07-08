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
//! dilation) land. See `docs/design/2026-07-02-infinity-points-and-records.md`.
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
    /// Game time (ms) of the most recent AD/Tickspeed purchase this infinity
    /// (`player.records.thisInfinity.lastBuyTime`). Under Infinity Challenge 8,
    /// AD production decays with `time - lastBuyTime`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub last_buy_time_ms: f64,
    /// Peak IP-per-minute rate this infinity (`thisInfinity.bestIPmin`),
    /// updated while a crunch is possible; shown on the header crunch button.
    #[cfg_attr(feature = "serde", serde(default))]
    pub best_ip_min: Decimal,
    /// The crunch IP gain at the moment the peak rate was set
    /// (`thisInfinity.bestIPminVal`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub best_ip_min_val: Decimal,
}

impl ThisInfinity {
    pub fn new() -> Self {
        Self {
            time_ms: 0.0,
            real_time_ms: 0.0,
            max_am: Decimal::ZERO,
            last_buy_time_ms: 0.0,
            best_ip_min: Decimal::ZERO,
            best_ip_min_val: Decimal::ZERO,
        }
    }
}

impl Default for ThisInfinity {
    fn default() -> Self {
        Self::new()
    }
}

/// Records for the current (in-progress) eternity. Reset by an Eternity; the
/// modelled slice of `player.records.thisEternity`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThisEternity {
    /// Game time elapsed in this eternity (ms).
    pub time_ms: f64,
    /// Real (wall-clock-equivalent) time elapsed in this eternity (ms).
    pub real_time_ms: f64,
    /// Peak antimatter reached this eternity (`thisEternity.maxAM`). Persists
    /// across a Big Crunch; gates Infinity-Challenge and Infinity-Dimension
    /// unlocks.
    pub max_am: Decimal,
    /// Peak Infinity Points reached this eternity (`thisEternity.maxIP`).
    /// Drives the Eternity goal check and the EP formula. Zeroed whenever IP is
    /// reset (i.e. on Eternity, mirroring `Currency.infinityPoints.reset()`).
    pub max_ip: Decimal,
    /// Peak EP-per-minute rate this eternity (`thisEternity.bestEPmin`),
    /// updated while an Eternity is possible; shown on the Eternity button.
    #[cfg_attr(feature = "serde", serde(default))]
    pub best_ep_min: Decimal,
    /// The Eternity EP gain at the moment the peak rate was set
    /// (`thisEternity.bestEPminVal`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub best_ep_min_val: Decimal,
}

impl ThisEternity {
    pub fn new() -> Self {
        Self {
            time_ms: 0.0,
            real_time_ms: 0.0,
            max_am: Decimal::ZERO,
            max_ip: Decimal::ZERO,
            best_ep_min: Decimal::ZERO,
            best_ep_min_val: Decimal::ZERO,
        }
    }
}

impl Default for ThisEternity {
    fn default() -> Self {
        Self::new()
    }
}

/// Records for the fastest eternity performed. Persists across an Eternity.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BestEternity {
    /// Fastest eternity by game time (ms). `f64::MAX` = "no eternity yet".
    pub time_ms: f64,
    /// Fastest eternity by real time (ms). Same sentinel.
    pub real_time_ms: f64,
}

impl BestEternity {
    pub fn new() -> Self {
        Self {
            time_ms: f64::MAX,
            real_time_ms: f64::MAX,
        }
    }
}

impl Default for BestEternity {
    fn default() -> Self {
        Self::new()
    }
}

/// One entry of the last-10-eternities ring (`player.records.recentEternities`
/// tuples `[time, realTime, EP, eternities]`; the trailing challenge/TT slots
/// are not modelled). The `f64::MAX` sentinel means "no run yet".
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RecentEternity {
    /// Game time of the run (ms).
    pub time_ms: f64,
    /// Real time of the run (ms). Feeds TS121's average-eternity-speed effect.
    pub real_time_ms: f64,
    /// EP gained by the run.
    pub ep: Decimal,
    /// Eternities gained by the run.
    pub eternities: Decimal,
}

impl RecentEternity {
    pub fn placeholder() -> Self {
        Self {
            time_ms: f64::MAX,
            real_time_ms: f64::MAX,
            ep: Decimal::ONE,
            eternities: Decimal::ONE,
        }
    }
}

/// serde default for the recent-eternities ring (10 placeholders).
#[cfg(feature = "serde")]
fn default_recent_eternities() -> Vec<RecentEternity> {
    vec![RecentEternity::placeholder(); 10]
}

/// Records for the current (in-progress) reality. Reset by a Reality; the
/// modelled slice of `player.records.thisReality`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThisReality {
    /// Game time elapsed in this reality (ms).
    pub time_ms: f64,
    /// Real time elapsed in this reality (ms).
    pub real_time_ms: f64,
    /// Peak antimatter reached this reality (`thisReality.maxAM`); maintained by
    /// the antimatter setter in the original (`currency.js`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_am: Decimal,
    /// Peak Infinity Points reached this reality (`thisReality.maxIP`);
    /// maintained by the IP setter in the original (`currency.js`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_ip: Decimal,
    /// Peak Eternity Points reached this reality (`thisReality.maxEP`).
    /// Maintained by the EP setter in the original; advanced in `tick` and on
    /// EP awards here. Gates the Reality study/availability and feeds the RM
    /// formula and glyph level.
    pub max_ep: Decimal,
    /// Peak Replicanti amount this reality (`thisReality.maxReplicanti`);
    /// feeds glyph level.
    pub max_replicanti: Decimal,
    /// Peak Dilated Time this reality (`thisReality.maxDT`); feeds glyph
    /// level.
    pub max_dt: Decimal,
}

impl ThisReality {
    pub fn new() -> Self {
        Self {
            time_ms: 0.0,
            real_time_ms: 0.0,
            max_am: Decimal::ZERO,
            max_ip: Decimal::ZERO,
            max_ep: Decimal::ZERO,
            max_replicanti: Decimal::ZERO,
            max_dt: Decimal::ZERO,
        }
    }
}

impl Default for ThisReality {
    fn default() -> Self {
        Self::new()
    }
}

/// Records of the best realities (`player.records.bestReality`, the modelled
/// slice — the equipped-glyph-loadout snapshots are display-only and out of
/// frontier). Persists across a Reality.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BestReality {
    /// Fastest reality by game time (ms). `f64::MAX` = "no reality yet".
    pub time_ms: f64,
    /// Fastest reality by real time (ms). Same sentinel.
    pub real_time_ms: f64,
    /// Best RM-per-real-minute rate over any reality (`bestReality.RMmin`).
    pub rm_min: Decimal,
    /// Highest glyph level attained at a reality (`bestReality.glyphLevel`).
    pub glyph_level: u32,
    /// Highest EP ever held (`bestReality.bestEP`; maintained by the EP
    /// setter). Backs Reality Upgrade 25's requirement.
    pub best_ep: Decimal,
    /// Best glyph strength (rarity) ever picked up
    /// (`bestReality.glyphStrength`).
    pub glyph_strength: f64,
}

impl BestReality {
    pub fn new() -> Self {
        Self {
            time_ms: f64::MAX,
            real_time_ms: f64::MAX,
            rm_min: Decimal::ZERO,
            glyph_level: 0,
            best_ep: Decimal::ZERO,
            glyph_strength: 1.0,
        }
    }
}

impl Default for BestReality {
    fn default() -> Self {
        Self::new()
    }
}

/// One entry of the last-10-realities ring (`player.records.recentRealities`
/// tuples `[time, realTime, RM, realityCount, challenge, projIM]`; the
/// challenge/iM slots are out of frontier). `f64::MAX` = "no run yet".
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RecentReality {
    /// Game time of the run (ms).
    pub time_ms: f64,
    /// Real time of the run (ms).
    pub real_time_ms: f64,
    /// RM gained by the run.
    pub rm: Decimal,
    /// Realities gained by the run.
    pub reality_count: f64,
}

impl RecentReality {
    pub fn placeholder() -> Self {
        Self {
            time_ms: f64::MAX,
            real_time_ms: f64::MAX,
            rm: Decimal::ONE,
            reality_count: 1.0,
        }
    }
}

/// serde default for the recent-realities ring (10 placeholders).
#[cfg(feature = "serde")]
fn default_recent_realities() -> Vec<RecentReality> {
    vec![RecentReality::placeholder(); 10]
}

/// serde default for "not yet" time sentinels (`Number.MAX_VALUE`).
#[cfg(feature = "serde")]
fn default_f64_max() -> f64 {
    f64::MAX
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
    /// Total game time played when the Black Hole was unlocked
    /// (`records.timePlayedAtBHUnlock`; `f64::MAX` = not yet). Feeds Reality
    /// Upgrade 20's requirement.
    #[cfg_attr(feature = "serde", serde(default = "default_f64_max"))]
    pub time_played_at_bh_unlock_ms: f64,
    /// The current infinity's records (reset on crunch).
    pub this_infinity: ThisInfinity,
    /// The fastest infinity's records (kept on crunch).
    pub best_infinity: BestInfinity,
    /// The current eternity's records (reset on Eternity). The peak-antimatter
    /// component persists across a Big Crunch and gates Infinity-Challenge /
    /// Infinity-Dimension unlocks.
    #[cfg_attr(feature = "serde", serde(default))]
    pub this_eternity: ThisEternity,
    /// The fastest eternity's records (kept on Eternity).
    #[cfg_attr(feature = "serde", serde(default))]
    pub best_eternity: BestEternity,
    /// The last 10 eternities, newest first (`records.recentEternities`).
    #[cfg_attr(feature = "serde", serde(default = "default_recent_eternities"))]
    pub recent_eternities: Vec<RecentEternity>,
    /// The current reality's records (reset on Reality).
    #[cfg_attr(feature = "serde", serde(default))]
    pub this_reality: ThisReality,
    /// The best realities' records (kept on Reality).
    #[cfg_attr(feature = "serde", serde(default))]
    pub best_reality: BestReality,
    /// The last 10 realities, newest first (`records.recentRealities`).
    #[cfg_attr(feature = "serde", serde(default = "default_recent_realities"))]
    pub recent_realities: Vec<RecentReality>,
}

impl Records {
    pub fn new() -> Self {
        Self {
            total_time_played_ms: 0.0,
            real_time_played_ms: 0.0,
            time_played_at_bh_unlock_ms: f64::MAX,
            this_infinity: ThisInfinity::new(),
            best_infinity: BestInfinity::new(),
            this_eternity: ThisEternity::new(),
            best_eternity: BestEternity::new(),
            recent_eternities: vec![RecentEternity::placeholder(); 10],
            this_reality: ThisReality::new(),
            best_reality: BestReality::new(),
            recent_realities: vec![RecentReality::placeholder(); 10],
        }
    }
}

impl Default for Records {
    fn default() -> Self {
        Self::new()
    }
}
