//! Reality (Feature 6.1): the third prestige layer. Once the Reality study is
//! bought and this reality's peak EP reaches `1e4000`, the player can Reality —
//! resetting everything from the Eternity layer down — for Reality Machines,
//! a Glyph, and a Perk Point.
//!
//! Mirrors `src/core/reality.js` (`isRealityAvailable`, `giveRealityRewards`,
//! `finishProcessReality`), `src/core/machines.js` (the RM formula), and the
//! glyph-level half of `src/core/glyphs/auto-glyph-processor.js`
//! (`getGlyphLevelInputs`). Glyphs themselves are Feature 6.2 (`glyphs.rs`).
//! See `docs/design/2026-07-05-reality.md`.

use break_infinity::Decimal;

use crate::achievements::IMPLEMENTED_ACHIEVEMENTS;
use crate::records::{RecentReality, ThisEternity, ThisInfinity, ThisReality};
use crate::replicanti::ReplicantiState;
use crate::state::{DimensionTier, GameState, TickspeedState};
use crate::time_dimensions::TimeDimension;

/// The Reality Machine hardcap (`MachineHandler.baseRMCap`); Imaginary
/// Machines beyond it are out of frontier.
pub const RM_HARDCAP: Decimal = Decimal::new_unchecked(1.0, 1000);

/// Pre-reality achievements span rows 1–13 (`Achievements.preReality`).
pub const PRE_REALITY_ACHIEVEMENT_ROWS: usize = 13;

/// `bestInfinity.time` reset sentinel (the original's quirky
/// `999999999999`, not `Number.MAX_VALUE`).
const BEST_INFINITY_RESET_MS: f64 = 999_999_999_999.0;

/// serde default mirrors of `player.reality` defaults.
#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}
#[cfg(feature = "serde")]
fn default_second_gaussian() -> f64 {
    1e6
}
#[cfg(feature = "serde")]
fn default_seed() -> f64 {
    1.0
}

/// The modelled slice of `player.reality` (minus glyphs, which live in
/// [`GlyphState`](crate::glyphs::GlyphState)). Persists forever.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RealityState {
    /// Reality Machines (`Currency.realityMachines`).
    pub machines: Decimal,
    /// The highest RM ever held (`player.reality.maxRM`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_rm: Decimal,
    /// Realities performed (`player.realities`, stored at the player root in
    /// the original save).
    pub realities: u32,
    /// Unspent Perk Points (`player.reality.perkPoints`).
    pub perk_points: f64,
    /// Bought perks (`player.reality.perks`, a Set of ids). Feature 6.3.
    #[cfg_attr(feature = "serde", serde(default))]
    pub perks: std::collections::BTreeSet<u8>,
    /// The glyph RNG seed (`player.reality.seed`). Locked to `initial_seed`
    /// on the first Reality; advanced (as a 32-bit xorshift state) by every
    /// finalized glyph roll. Kept as `f64` to round-trip the original save.
    #[cfg_attr(feature = "serde", serde(default = "default_seed"))]
    pub seed: f64,
    /// The save's fixed RNG base seed (`player.reality.initialSeed`), also
    /// used raw (full precision) by the glyph uniformity code.
    pub initial_seed: f64,
    /// The Marsaglia-polar spare normal deviate (`reality.secondGaussian`;
    /// sentinel `1e6` = none cached).
    #[cfg_attr(feature = "serde", serde(default = "default_second_gaussian"))]
    pub second_gaussian: f64,
    /// One-time Reality Upgrades (ids 6–25), bit `1 << id`
    /// (`player.reality.upgradeBits`). Feature 6.4.
    #[cfg_attr(feature = "serde", serde(default))]
    pub upgrade_bits: u32,
    /// Which one-time upgrades have met their requirement
    /// (`player.reality.upgReqs`). Feature 6.4.
    #[cfg_attr(feature = "serde", serde(default))]
    pub upg_reqs: u32,
    /// Player-armed requirement locks (`player.reality.reqLock.reality`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub req_lock: u32,
    /// Rebuyable Reality Upgrade purchase counts, ids 1–5
    /// (`player.reality.rebuyables`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub rebuyables: [u32; 5],
    /// Whether the next Reality unequips all glyphs (`player.reality.respec`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub respec: bool,
    /// Auto-achievement timer in ms (`player.reality.achTimer`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub ach_timer: f64,
    /// Whether auto-achievements regrant over time (`player.reality
    /// .autoAchieve`, default on).
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub auto_achieve: bool,
    /// Whether any achievement was auto-granted this reality
    /// (`player.reality.gainedAutoAchievements`; fails Reality Upgrade 8's
    /// requirement). Fresh saves start `true`.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub gained_auto_achievements: bool,
    /// Glyphs (`player.reality.glyphs`): equipped + inventory + sacrifice
    /// totals. See `glyphs.rs`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub glyphs: crate::glyphs::GlyphState,
    /// `player.reality.automator.forceUnlock`: the dev/testing flag that
    /// unlocks the Automator regardless of AP (see `automator_points.rs`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub automator_force_unlock: bool,
    /// Imaginary Machines (`Currency.imaginaryMachines`) + the highest ever
    /// (`player.reality.imaginaryUpgradeBits` etc.). Feature 6.4-late / 7.6.
    #[cfg_attr(feature = "serde", serde(default))]
    pub imaginary_machines: Decimal,
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_im: Decimal,
    /// One-time Imaginary Upgrades (ids 11–25), bit `1 << id`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub imaginary_upgrade_bits: u32,
    /// Rebuyable Imaginary Upgrade counts, ids 1–10 (`imaginaryRebuyables`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub imaginary_rebuyables: [u32; 10],
}

impl RealityState {
    pub fn new() -> Self {
        Self {
            machines: Decimal::ZERO,
            max_rm: Decimal::ZERO,
            realities: 0,
            perk_points: 0.0,
            perks: std::collections::BTreeSet::new(),
            // The original rolls `Date.now() * random()`; we need a fixed
            // value for determinism. The GUI/save layer may reseed a fresh
            // save with entropy.
            seed: 1.0,
            initial_seed: 4_294_967_291.0,
            second_gaussian: 1e6,
            upgrade_bits: 0,
            upg_reqs: 0,
            req_lock: 0,
            rebuyables: [0; 5],
            respec: false,
            ach_timer: 0.0,
            auto_achieve: true,
            gained_auto_achievements: true,
            glyphs: crate::glyphs::GlyphState::new(),
            automator_force_unlock: false,
            imaginary_machines: Decimal::ZERO,
            max_im: Decimal::ZERO,
            imaginary_upgrade_bits: 0,
            imaginary_rebuyables: [0; 10],
        }
    }
}

impl Default for RealityState {
    fn default() -> Self {
        Self::new()
    }
}

/// The modelled slice of `player.requirementChecks`: run-scoped "did the
/// player avoid X" flags consumed by Reality Upgrade requirements.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RequirementChecks {
    /// No Replicanti Galaxy bought this eternity (`eternity.noRG`).
    pub eternity_no_rg: bool,
    /// Only 8th Antimatter Dimensions bought this eternity (`eternity.onlyAD8`);
    /// cleared when any other tier is purchased.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub eternity_only_ad8: bool,
    /// Only 1st Antimatter Dimensions bought this eternity (`eternity.onlyAD1`);
    /// cleared when any other tier is purchased.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub eternity_only_ad1: bool,
    /// No 1st Antimatter Dimension bought this eternity (`eternity.noAD1`);
    /// cleared on an AD1 purchase or whenever AD1 has a nonzero amount.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub eternity_no_ad1: bool,
    /// No antimatter gained this reality (`reality.noAM`); cleared on any
    /// antimatter gain.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub reality_no_am: bool,
    /// A "Max All" was used this infinity (`infinity.maxAll`); a latch set only
    /// by the manual Max All action (never reset), so it round-trips unchanged.
    #[cfg_attr(feature = "serde", serde(default))]
    pub infinity_max_all: bool,
    /// No 8th Antimatter Dimension bought this eternity (`infinity.noAD8`);
    /// cleared on an AD8 purchase.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub infinity_no_ad8: bool,
    /// No Sacrifice performed since the last Galaxy (`infinity.noSacrifice`);
    /// cleared on a Sacrifice, restored on a Galaxy.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub infinity_no_sacrifice: bool,
    /// No manual Infinity this reality (`reality.noInfinities`).
    pub reality_no_infinities: bool,
    /// No manual Eternity this reality (`reality.noEternities`).
    pub reality_no_eternities: bool,
    /// Peak simultaneously-equipped glyph count this reality
    /// (`reality.maxGlyphs`).
    pub reality_max_glyphs: i32,
    /// Whether a 1st Infinity Dimension was purchased this reality
    /// (`reality.maxID1` ≠ 0). Gates Imaginary Upgrade 15.
    #[cfg_attr(feature = "serde", serde(default))]
    pub reality_had_id1: bool,
    /// Peak Time Study count this reality (`reality.maxStudies`). Gates
    /// Imaginary Upgrade 19.
    #[cfg_attr(feature = "serde", serde(default))]
    pub reality_max_studies: u32,
    /// Whether Continuum stayed disabled all reality (`reality.noContinuum`).
    /// Gates Imaginary Upgrade 21.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub reality_no_continuum: bool,
}

impl RequirementChecks {
    pub fn new() -> Self {
        Self {
            eternity_no_rg: true,
            eternity_only_ad8: true,
            eternity_only_ad1: true,
            eternity_no_ad1: true,
            reality_no_am: true,
            infinity_max_all: false,
            infinity_no_ad8: true,
            infinity_no_sacrifice: true,
            reality_no_infinities: true,
            reality_no_eternities: true,
            reality_max_glyphs: 0,
            reality_had_id1: false,
            reality_max_studies: 0,
            reality_no_continuum: true,
        }
    }
}

impl Default for RequirementChecks {
    fn default() -> Self {
        Self::new()
    }
}

/// A glyph level roll: the pre-instability (`rawLevel`) and softcapped
/// (`actualLevel`) values of `gainedGlyphLevel()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphLevel {
    pub raw_level: u32,
    pub actual_level: u32,
}

impl GameState {
    // --- Availability -----------------------------------------------------------

    /// Whether the player has ever realitied (`PlayerProgress.realityUnlocked`).
    pub fn reality_unlocked(&self) -> bool {
        self.reality.realities > 0
    }

    /// `isRealityAvailable()`: the Reality study is bought and this reality's
    /// peak EP has reached `1e4000`.
    pub fn is_reality_available(&self) -> bool {
        self.records.this_reality.max_ep.exponent() >= 4000
            && self.dilation_study_bought(6)
    }

    /// The achievement half of the Reality study's requirement: the first
    /// perk waives it; otherwise every pre-Reality achievement must be
    /// unlocked. Our engine awards a subset of the original's achievements,
    /// so the check runs over the implemented ones (see the design doc).
    pub(crate) fn reality_study_achievements_ok(&self) -> bool {
        if self.perk_bought(0) {
            return true;
        }
        IMPLEMENTED_ACHIEVEMENTS
            .iter()
            .filter(|&&id| (id / 10) as usize <= PRE_REALITY_ACHIEVEMENT_ROWS)
            .all(|&id| self.achievement_unlocked(id))
    }

    /// Whether perk `id` is bought. The perk catalogue is Feature 6.3; the
    /// membership check is needed earlier (Reality study, achievements).
    pub fn perk_bought(&self, id: u8) -> bool {
        self.reality.perks.contains(&id)
    }

    // --- Reality Machines (machines.js) -----------------------------------------

    /// `MachineHandler.uncappedRM`: the RM the current reality is worth.
    pub fn uncapped_rm(&self) -> Decimal {
        let ep = self.records.this_reality.max_ep + self.gained_eternity_points();
        let mut log10_ep = ep.pos_log10();
        // Pre-first-reality softcap: hard 8000, 75% tax past 6000.
        if !self.reality_unlocked() {
            if log10_ep > 8000.0 {
                log10_ep = 8000.0;
            }
            if log10_ep > 6000.0 {
                log10_ep -= (log10_ep - 6000.0) * 0.75;
            }
        }
        let mut rm = Decimal::pow10(3.0 * (log10_ep / 4000.0 - 1.0));
        // Linear ramp from 1 to 10 RM.
        if rm >= Decimal::ONE && rm < Decimal::from_float(10.0) {
            rm = Decimal::from_float(27.0 / 4000.0 * log10_ep - 26.0);
        }
        // `realityMachineMultiplier`: Teresa's pool multiplier + Perk-Shop
        // rmMult (celestial/Ra sources are out of frontier → 1).
        rm *= Decimal::from_float(self.reality_machine_multiplier());
        rm.floor()
    }

    /// `MachineHandler.gainedRealityMachines`: hardcapped at `1e1000`.
    pub fn gained_reality_machines(&self) -> Decimal {
        let mut rm = self.uncapped_rm();
        // Achievement 167: more RM based on current RM (`max(1, log2(RM))`).
        if self.achievement_unlocked(167) {
            let factor =
                (self.reality.machines.pos_log10() / std::f64::consts::LOG10_2).max(1.0);
            rm *= Decimal::from_float(factor);
        }
        rm.min(&RM_HARDCAP)
    }

    // --- Glyph level (getGlyphLevelInputs) ---------------------------------------

    /// `gainedGlyphLevel()`: the level a glyph granted now would have.
    /// With default (untouched) Effarig weights the weight adjustment is the
    /// identity, so the level is the product of the EP/replicanti/DT/eternity
    /// factors, instability-softcapped, plus the static adders.
    pub fn gained_glyph_level(&self) -> GlyphLevel {
        let (raw, actual) = self.glyph_level_inputs();
        let floor_or_zero = |x: f64| {
            if x.is_finite() {
                x.max(0.0).floor() as u32
            } else {
                0
            }
        };
        GlyphLevel {
            raw_level: floor_or_zero(raw),
            actual_level: floor_or_zero(actual),
        }
    }

    /// The exact (unfloored) actual glyph level, for the Reality button's
    /// "% to next level" readout.
    pub fn gained_glyph_level_exact(&self) -> f64 {
        let (_, actual) = self.glyph_level_inputs();
        if actual.is_finite() {
            actual.max(0.0)
        } else {
            0.0
        }
    }

    /// The `(rawLevel, actualLevel)` pair of `getGlyphLevelInputs`.
    fn glyph_level_inputs(&self) -> (f64, f64) {
        // EP factor: pending Eternity gain counts while an Eternity is
        // possible (`getGlyphLevelSources`).
        let mut ep = if self.can_eternity() {
            self.eternity_points + self.gained_eternity_points()
        } else {
            self.eternity_points
        };
        ep = ep.max(&self.records.this_reality.max_ep);
        let ep_base = ep.pos_log10().max(1.0).powf(0.5) * 0.016;

        let repl_pow = 0.4 + self.glyph_effect_replicationglyphlevel();
        let repl_base = self
            .records
            .this_reality
            .max_replicanti
            .pos_log10()
            .max(1.0)
            .powf(repl_pow)
            * 0.025;

        let dt_base = self
            .records
            .this_reality
            .max_dt
            .pos_log10()
            .max(1.0)
            .powf(1.3)
            * 0.025;

        // RU18: eternity-count factor `max(√(log10(eternities+1))·0.45, 1)`.
        let eter_base = if self.reality_upgrade_bought(18) {
            ((self.eternities + Decimal::ONE).pos_log10().sqrt() * 0.45).max(1.0)
        } else {
            1.0
        };

        let mut base_level = ep_base * repl_base * dt_base * eter_base;
        // Lai'tela's `glyphLevelFromSingularities` milestone boosts the
        // pre-instability level.
        base_level *= self.singularity_milestone_effect_or(
            crate::celestials::singularity::GLYPH_LEVEL_FROM_SINGULARITIES,
            1.0,
        );

        // Instability softcaps: linear → quadratic past each threshold.
        let instability_softcap = |level: f64, begin: f64, rate: f64| {
            if level < begin {
                level
            } else {
                let excess = (level - begin) / rate;
                begin + 0.5 * rate * ((1.0 + 4.0 * excess).sqrt() - 1.0)
            }
        };
        let mut scaled = instability_softcap(base_level, 1000.0, 500.0);
        scaled = instability_softcap(scaled, 4000.0, 400.0);

        // Static post-instability adders: +1 per fully-bought Reality Upgrade
        // row, Ra's `relicShardGlyphLevelBoost`, and the achievement adders
        // `Effects.sum(Achievement(148), Achievement(166))`.
        let inc = self.reality_upgrade_row_factor() as f64
            + self.ra_relic_shard_glyph_level()
            + self.achievement_glyph_level_bonus() as f64;
        (base_level + inc, (scaled + inc).max(1.0))
    }

    /// The count of fully-purchased Reality Upgrade rows
    /// (`staticGlyphWeights().realityUpgrades`).
    fn reality_upgrade_row_factor(&self) -> u32 {
        let mut rows = 0;
        if self.reality.rebuyables.iter().all(|&n| n > 0) {
            rows += 1;
        }
        for row in 1..=4u32 {
            if (1..=5).all(|col| self.reality_upgrade_bought((5 * row + col) as u8)) {
                rows += 1;
            }
        }
        rows
    }

    /// Whether one-time Reality Upgrade `id` (6–25) is bought. The purchase
    /// logic is Feature 6.4; the bit test is needed by glyph level / resets.
    pub fn reality_upgrade_bought(&self, id: u8) -> bool {
        self.reality.upgrade_bits & (1u32 << id) != 0
    }

    // --- The Reality reset -------------------------------------------------------

    /// Perform a Reality (`processManualReality` without the glyph-choice UI
    /// path — the glyph grant itself is wired in by Feature 6.2). Returns
    /// whether it happened.
    pub fn reality(&mut self) -> bool {
        if !self.is_reality_available() {
            return false;
        }
        if self.reality.realities == 0 {
            // First reality: lock in the RNG seed.
            self.reality.seed = self.reality.initial_seed;
        }
        self.grant_reality_glyphs();
        self.finish_process_reality();
        true
    }

    /// The forced, reward-free Reality reset
    /// (`finishProcessReality(getRealityProps(true))` — the "Start this
    /// Reality over" button). No RM/glyph/Perk Point, no records.
    pub fn reset_reality(&mut self) -> bool {
        if !self.reality_unlocked() {
            return false;
        }
        self.reality_reset_internal();
        true
    }

    /// `giveRealityRewards` + `finishProcessReality`: award RM / a reality /
    /// a Perk Point, update the reality records, then reset everything from
    /// the Eternity layer down.
    pub(crate) fn finish_process_reality(&mut self) {
        // REALITY_RESET_BEFORE requirement checks (RU16–19/23/24).
        self.check_reality_upgrade_reqs_on_reality();
        // REALITY_RESET_BEFORE achievements (141, 148, 153, 154).
        self.check_reality_before_achievements();

        // -- Rewards (read from pre-reset state) --
        let final_ep = self.eternity_points + self.gained_eternity_points();
        if self.records.best_reality.best_ep < final_ep {
            self.records.best_reality.best_ep = final_ep;
        }

        let gained_rm = if self.reality.machines >= RM_HARDCAP {
            Decimal::ZERO
        } else {
            self.gained_reality_machines()
        };
        let glyph_level = self.gained_glyph_level();

        // `updateRealityRecords`.
        let minutes = (self.records.this_reality.real_time_ms / 60_000.0).max(0.000_5);
        let rm_min = gained_rm / Decimal::from_float(minutes);
        if self.records.best_reality.rm_min < rm_min {
            self.records.best_reality.rm_min = rm_min;
        }
        if self.records.best_reality.glyph_level < glyph_level.actual_level {
            self.records.best_reality.glyph_level = glyph_level.actual_level;
        }
        self.records.best_reality.time_ms = self
            .records
            .best_reality
            .time_ms
            .min(self.records.this_reality.time_ms);
        if self.records.this_reality.real_time_ms
            < self.records.best_reality.real_time_ms
        {
            self.records.best_reality.real_time_ms =
                self.records.this_reality.real_time_ms;
        }

        // `addRealityTime`: the last-10-realities ring.
        self.records.recent_realities.pop();
        self.records.recent_realities.insert(
            0,
            RecentReality {
                time_ms: self.records.this_reality.time_ms,
                real_time_ms: self.records.this_reality.real_time_ms,
                rm: gained_rm,
                reality_count: 1.0,
            },
        );

        self.reality.machines += gained_rm;
        self.reality.max_rm = self.reality.max_rm.max(&self.reality.machines);
        self.reality.realities += 1;
        self.reality.perk_points += 1.0;

        // Relic Shards + the per-celestial run-completion hooks (Teresa best AM,
        // Effarig stage unlock, Enslaved completion). Read the pre-reset run
        // flags, before `reality_reset_internal` clears them.
        self.effarig_gain_relic_shards();
        self.celestial_reality_completion_hooks();

        // Ra: run the Glyph-Alchemy reactions once per rewarded Reality
        // (`Ra.applyAlchemyReactions`, gated on Effarig's Memories).
        self.apply_alchemy_reactions();

        self.reality_reset_internal();

        // REALITY_RESET_AFTER achievements (175).
        self.check_reality_after_achievements();

        // The Automator's REALITY_RESET_AFTER handling: the prestige
        // notification, the optional event-log clear, and the force-restart
        // (any Reality — manual or automatic — restarts the running script
        // from the top when the toggle is on; `reality.js` calls
        // `AutomatorBackend.restart()` from inside the reset).
        self.automator_notify_prestige(
            crate::automator::PrestigeLayer::Reality,
            gained_rm,
        );
        if self.options.automator_events.clear_on_reality {
            self.automator_clear_event_log();
        }
        if self.automator_unlocked() && self.automator.state.force_restart {
            self.automator_restart();
        }
    }

    /// The reset half of `finishProcessReality`, shared by a rewarded Reality
    /// and the forced reset.
    fn reality_reset_internal(&mut self) {
        // `clearCelestialRuns()`: a Reality (rewarded or forced) always exits
        // any celestial run.
        self.clear_celestial_runs();

        // Ra: discharge the charged Infinity Upgrades if flagged, and reset the
        // per-Reality peak game-speed accumulator (both on every Reality).
        if self.celestials.ra.dis_charge {
            self.celestials.ra.charged = 0;
            self.celestials.ra.dis_charge = false;
        }
        self.ra_on_reality_reset();

        if self.reality.respec {
            self.unequip_all_glyphs();
            self.reality.respec = false;
        }

        self.sacrificed = Decimal::ZERO;

        self.lock_achievements_on_reality();

        // `initializeChallengeCompletions(true)`: cleared with no
        // milestone-regrant (eternities reset below).
        self.challenge.completed = 0;
        self.challenge.current = 0;
        self.infinity_challenge.completed = 0;
        self.infinity_challenge.current = 0;

        self.infinities = Decimal::ZERO;
        self.infinities_banked = Decimal::ZERO;
        self.records.best_infinity.time_ms = BEST_INFINITY_RESET_MS;
        self.records.best_infinity.real_time_ms = BEST_INFINITY_RESET_MS;
        self.records.best_infinity.best_ip_min_eternity = Decimal::ZERO;
        self.records.this_infinity = ThisInfinity::new();
        self.dim_boosts = 0;
        self.galaxies = 0;
        self.part_infinity_point = 0.0;
        self.broke_infinity = false;
        // `Currency.infinityPoints.reset()` (to the START-perk value).
        self.infinity_points = self.starting_ip();
        self.records.this_eternity.max_ip = self.infinity_points;
        self.infinity_power = Decimal::ZERO;
        self.time_shards = Decimal::ZERO;
        self.replicanti = ReplicantiState::new();

        // EP reset to its starting value — the START perks — which also sets
        // this reality's EP peak (`Currency.eternityPoints.reset()`).
        self.eternity_points = self.starting_ep();
        self.records.this_reality.max_ep = self.eternity_points;

        self.epmult_upgrades = 0;
        self.ip_mult_purchases = 0;
        self.eternities = Decimal::ZERO;
        self.records.this_eternity = ThisEternity::new();
        self.records.best_eternity = crate::records::BestEternity {
            time_ms: BEST_INFINITY_RESET_MS,
            real_time_ms: BEST_INFINITY_RESET_MS,
        };
        self.eternity_upgrades = 0;
        self.total_tick_gained = 0;
        self.eternity_challenges = [0; 12];
        self.eternity_challenge_unlocked = 0;
        self.eternity_challenge_current = 0;
        self.ec_requirement_bits = 0;
        self.respec = false;
        self.eterc8_ids = 50;
        self.eterc8_repl = 40;

        self.requirement_checks = RequirementChecks::new();
        self.requirement_checks.reality_max_glyphs = self.equipped_glyph_count();

        self.records.this_reality = ThisReality::new();
        // Re-seed the peaks from the START-perk starting currencies (the
        // original's currency `reset()`s run after the records resets).
        self.records.this_eternity.max_ip = self.infinity_points;
        self.records.this_reality.max_ep = self.eternity_points;

        // `Currency.timeTheorems.reset()`: respec + TT/max/purchases cleared.
        self.studies = Vec::new();
        self.time_theorems = Decimal::ZERO;
        self.max_theorem = Decimal::ZERO;
        self.tt_am_bought = 0;
        self.tt_ip_bought = 0;
        self.tt_ep_bought = 0;

        // Dilation: studies, run, upgrades, TP/DT/TGs all reset.
        self.dilation = crate::DilationState::new();

        self.records.this_infinity.max_am = Decimal::ZERO;
        self.records.this_eternity.max_am = Decimal::ZERO;

        self.antimatter = self.starting_antimatter();

        // `playerInfinityUpgradesOnReset` — eternities are 0, so everything
        // clears.
        self.infinity_upgrades = 0;
        self.break_infinity_upgrades = 0;
        self.infinity_rebuyables = [0, 0, 0];

        // `resetInfinityRuns` / `resetEternityRuns`.
        self.records.recent_eternities =
            vec![crate::records::RecentEternity::placeholder(); 10];

        // Infinity Dimensions full reset; Time Dimensions *full* reset
        // (purchases too — unlike an Eternity).
        self.infinity_dimensions = std::array::from_fn(crate::InfinityDimension::new);
        self.time_dimensions = std::array::from_fn(TimeDimension::new);
        self.ic_best_times_ms = [f64::MAX; 8];

        // `resetChallengeStuff` + per-run counters.
        self.reset_challenge_stuff();
        self.post_c4_tier = 1;
        self.ic2_count = 0.0;

        // ADs + tickspeed (`AntimatterDimensions.reset` / `resetTickspeed`).
        self.dimensions = std::array::from_fn(|_| DimensionTier::new());
        self.tickspeed = TickspeedState::new();

        // Autobuyer reset (eternities are 0 → no milestone keeps).
        self.reset_autobuyers_on_reality();

        // Post-reset upgrades/perks kick in (RU10 package etc., Feature 6.4).
        self.apply_post_reality_upgrades();

        // The prestige autobuyers' config resets (`Autobuyers.reset()` on
        // REALITY_RESET_AFTER). Runs after RU10's package so its 100
        // eternities keep the crunch modes / Eternity autobuyer, matching the
        // original's event ordering.
        self.reset_prestige_autobuyer_configs();

        self.reality.gained_auto_achievements = false;
    }

    /// The autobuyer half of the reset. Reuses the Eternity path; the
    /// milestone keeps are moot because eternities were just zeroed.
    fn reset_autobuyers_on_reality(&mut self) {
        // `eternity_full_reset`'s autobuyer handling is private to
        // eternity.rs; the shared implementation lives there.
        self.reset_autobuyers_for_prestige();
    }

    /// Hook for Feature 6.2: the glyph grant on Reality (starting/companion
    /// glyphs on the first, a generated glyph afterwards).
    fn grant_reality_glyphs(&mut self) {
        self.grant_reality_glyphs_impl();
    }

    /// Hook for Feature 6.4 (RU10's start-of-reality package, RU14's flow) and
    /// Teresa's `startEU` (grant all 6 Eternity Upgrades on reset).
    fn apply_post_reality_upgrades(&mut self) {
        if self.reality_upgrade_bought(10) {
            self.apply_rupg10();
        }
        self.apply_teresa_start_eu();
    }

    // --- Achievements on Reality -------------------------------------------------

    /// `lockAchievementsOnReality`: without the ACHNR perk (id 205) all
    /// pre-Reality achievements re-lock and the auto-grant timer restarts.
    fn lock_achievements_on_reality(&mut self) {
        if self.perk_bought(205) {
            return;
        }
        for row in 0..PRE_REALITY_ACHIEVEMENT_ROWS {
            self.achievement_bits[row] = 0;
        }
        self.reality.ach_timer = 0.0;
    }

    /// The auto-achievement period in ms: 30 minutes, less the ACH perks'
    /// reductions (`GameCache.achievementPeriod`).
    pub fn achievement_period_ms(&self) -> f64 {
        let mut minutes = 30.0;
        for (perk, reduction) in [(201u8, 10.0), (202, 8.0), (203, 6.0), (204, 4.0)] {
            if self.perk_bought(perk) {
                minutes -= reduction;
            }
        }
        minutes * 60_000.0
    }

    /// `Achievements.autoAchieveUpdate`: after the first Reality, locked
    /// pre-Reality achievements regrant in id order, one per period.
    pub(crate) fn tick_auto_achievements(&mut self, real_dt_ms: f64) {
        if !self.reality_unlocked() {
            return;
        }
        let period = self.achievement_period_ms();
        if !self.reality.auto_achieve {
            self.reality.ach_timer = (self.reality.ach_timer + real_dt_ms).min(period);
            return;
        }
        if self.pre_reality_achievements_complete() {
            return;
        }
        self.reality.ach_timer += real_dt_ms;
        if self.reality.ach_timer < period {
            return;
        }
        for row in 1..=PRE_REALITY_ACHIEVEMENT_ROWS as u16 {
            for column in 1..=crate::achievements::ACHIEVEMENTS_PER_ROW {
                let id = row * 10 + column;
                if self.achievement_unlocked(id) {
                    continue;
                }
                self.unlock_achievement(id);
                self.reality.ach_timer -= period;
                if self.reality.ach_timer < period {
                    self.reality.gained_auto_achievements = true;
                    return;
                }
            }
        }
        self.reality.gained_auto_achievements = true;
    }

    /// Whether every pre-Reality achievement (rows 1–13) is unlocked.
    pub fn pre_reality_achievements_complete(&self) -> bool {
        self.achievement_bits[..PRE_REALITY_ACHIEVEMENT_ROWS]
            .iter()
            .all(|&bits| {
                (bits & 0xFF).count_ones()
                    == crate::achievements::ACHIEVEMENTS_PER_ROW as u32
            })
    }
}

/// Seams into the glyph module (Feature 6.2) and the Reality Upgrades
/// (Feature 6.4).
impl GameState {
    /// The `replicationglyphlevel` glyph effect.
    pub(crate) fn glyph_effect_replicationglyphlevel(&self) -> f64 {
        self.glyph_effect_replicationglyphlevel_impl()
    }

    /// Unequip every equipped glyph into the inventory (the respec path).
    pub(crate) fn unequip_all_glyphs(&mut self) {
        self.unequip_all_glyphs_impl();
    }

    /// Currently equipped glyph count (excluding the companion).
    pub(crate) fn equipped_glyph_count(&self) -> i32 {
        self.active_glyphs_without_companion().len() as i32
    }

    /// The glyph grant on a plain `reality()` call (no explicit choice: the
    /// first/deterministic option, kept).
    pub(crate) fn grant_reality_glyphs_impl(&mut self) {
        self.grant_glyphs_on_reality(None, false);
    }

    /// RU10's start-of-reality package (Feature 6.4).
    pub(crate) fn apply_rupg10(&mut self) {
        self.apply_rupg10_impl();
    }

    /// The Black Holes' game-speed multiplier (Feature 6.5).
    pub(crate) fn black_hole_speed_factor(&self) -> f64 {
        self.black_hole_speed_factor_impl()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A state eligible for Reality: study bought, 1e4000 peak EP.
    pub(crate) fn game_at_reality_goal() -> GameState {
        let mut game = GameState::new();
        game.eternity_unlocked = true;
        game.eternities = Decimal::from_float(1e6);
        game.eternity_points = Decimal::new(1.0, 4000);
        game.records.this_reality.max_ep = Decimal::new(1.0, 4000);
        game.dilation.studies = vec![1, 2, 3, 4, 5, 6];
        game.time_theorems = Decimal::from_float(1e10);
        game.max_theorem = Decimal::from_float(1e10);
        game
    }

    #[test]
    fn reality_unavailable_without_study_or_ep() {
        let mut game = GameState::new();
        assert!(!game.is_reality_available());
        game.records.this_reality.max_ep = Decimal::new(1.0, 4000);
        assert!(!game.is_reality_available());
        game.dilation.studies = vec![1, 2, 3, 4, 5, 6];
        assert!(game.is_reality_available());
        game.records.this_reality.max_ep = Decimal::new(1.0, 3999);
        assert!(!game.is_reality_available());
    }

    #[test]
    fn rm_formula_matches_original() {
        let mut game = game_at_reality_goal();
        // At exactly 1e4000 EP: 1000^(4000/4000 - 1) = 1 → linear ramp:
        // 27/4000·4000 − 26 = 1.
        assert_eq!(game.gained_reality_machines(), Decimal::ONE);

        // At 1e6000: 1000^0.5 ≈ 31.6 → floor 31.
        game.records.this_reality.max_ep = Decimal::new(1.0, 6000);
        assert_eq!(game.gained_reality_machines(), Decimal::from_float(31.0));

        // Pre-first-reality softcap: 1e8000 EP is taxed to an effective
        // 6500 → 1000^0.625 ≈ 74.98 → 74.
        game.records.this_reality.max_ep = Decimal::new(1.0, 8000);
        assert_eq!(game.gained_reality_machines(), Decimal::from_float(74.0));

        // After the first reality the softcap lifts: 1000^1 = 1000.
        game.reality.realities = 1;
        assert_eq!(game.gained_reality_machines(), Decimal::from_float(1000.0));
    }

    #[test]
    fn rm_linear_ramp_between_1_and_10() {
        let mut game = game_at_reality_goal();
        // At 1e5000 EP: 1000^0.25 ≈ 5.6 → ramp: 27/4000·5000 − 26 = 7.75 → 7.
        game.records.this_reality.max_ep = Decimal::new(1.0, 5000);
        assert_eq!(game.gained_reality_machines(), Decimal::from_float(7.0));
    }

    #[test]
    fn glyph_level_from_records() {
        let mut game = game_at_reality_goal();
        game.records.this_reality.max_ep = Decimal::new(1.0, 8000);
        game.records.this_reality.max_replicanti = Decimal::new(1.0, 20_000);
        game.records.this_reality.max_dt = Decimal::new(1.0, 15);
        // ep: (8000)^0.5·0.016 ≈ 1.431; repl: 20000^0.4·0.025 ≈ 1.318;
        // dt: 15^1.3·0.025 ≈ 0.8368 → level ≈ 1.579 → floor 1.
        let level = game.gained_glyph_level();
        assert_eq!(level.actual_level, 1);
        assert_eq!(level.raw_level, 1);
    }

    #[test]
    fn glyph_level_instability_softcap() {
        let mut game = game_at_reality_goal();
        // Force a huge base level via absurd records.
        game.records.this_reality.max_ep = Decimal::new(1.0, 4_000_000);
        game.records.this_reality.max_replicanti = Decimal::new(1.0, 4_000_000);
        game.records.this_reality.max_dt = Decimal::new(1.0, 4_000_000);
        let level = game.gained_glyph_level();
        assert!(level.raw_level > level.actual_level);
        assert!(level.actual_level >= 4000);
    }

    #[test]
    fn reality_awards_rm_realities_and_perk_points() {
        let mut game = game_at_reality_goal();
        game.records.this_reality.time_ms = 3_600_000.0;
        game.records.this_reality.real_time_ms = 3_600_000.0;
        assert!(game.reality());

        assert_eq!(game.reality.machines, Decimal::ONE);
        assert_eq!(game.reality.realities, 1);
        assert_eq!(game.reality.perk_points, 1.0);
        // Seed locked in from the initial seed on the first reality.
        assert_eq!(game.reality.seed, game.reality.initial_seed);
        // Records.
        assert_eq!(game.records.best_reality.time_ms, 3_600_000.0);
        assert_eq!(game.records.recent_realities[0].rm, Decimal::ONE);
        assert!(game.records.best_reality.best_ep >= Decimal::new(1.0, 4000));
    }

    #[test]
    fn reality_resets_the_eternity_layer() {
        let mut game = game_at_reality_goal();
        game.infinities = Decimal::from_float(1e30);
        game.infinities_banked = Decimal::from_float(1e20);
        game.eternity_upgrades = 0b111111;
        game.epmult_upgrades = 50;
        game.studies = vec![11, 21, 22];
        game.eternity_challenges = [5; 12];
        game.dilation.tachyon_particles = Decimal::from_float(1e10);
        game.dilation.dilated_time = Decimal::from_float(1e12);
        game.time_dimensions[0].bought = 100;
        game.broke_infinity = true;
        game.replicanti.unlocked = true;

        assert!(game.reality());

        assert_eq!(game.eternity_points, Decimal::ZERO);
        assert_eq!(game.eternities, Decimal::ZERO);
        assert_eq!(game.infinities, Decimal::ZERO);
        assert_eq!(game.infinities_banked, Decimal::ZERO);
        assert_eq!(game.eternity_upgrades, 0);
        assert_eq!(game.epmult_upgrades, 0);
        assert!(game.studies.is_empty());
        assert_eq!(game.time_theorems, Decimal::ZERO);
        assert_eq!(game.max_theorem, Decimal::ZERO);
        assert_eq!(game.eternity_challenges, [0; 12]);
        assert!(!game.dilation_unlocked());
        assert_eq!(game.dilation.tachyon_particles, Decimal::ZERO);
        assert_eq!(game.dilation.dilated_time, Decimal::ZERO);
        assert_eq!(game.time_dimensions[0].bought, 0);
        assert!(!game.broke_infinity);
        assert!(!game.replicanti.unlocked);
        assert_eq!(game.records.this_reality.max_ep, Decimal::ZERO);
        assert!(!game.is_reality_available());

        // Reality-layer state persists.
        assert_eq!(game.reality.realities, 1);
        assert!(game.reality.machines > Decimal::ZERO);
    }

    #[test]
    fn reality_locks_pre_reality_achievements() {
        let mut game = game_at_reality_goal();
        game.unlock_achievement(11);
        game.unlock_achievement(136);
        // A (hypothetical) row-14 achievement bit survives.
        game.achievement_bits[13] = 1;

        assert!(game.reality());

        assert!(!game.achievement_unlocked(11));
        assert!(!game.achievement_unlocked(136));
        assert_eq!(game.achievement_bits[13], 1);
        assert!(!game.reality.gained_auto_achievements);
    }

    #[test]
    fn auto_achievements_regrant_over_time() {
        let mut game = game_at_reality_goal();
        assert!(game.reality());
        assert_eq!(game.reality.ach_timer, 0.0);

        // 30 minutes → first achievement (id 11) regrants.
        game.tick_auto_achievements(30.0 * 60_000.0);
        assert!(game.achievement_unlocked(11));
        assert!(!game.achievement_unlocked(12));
        assert!(game.reality.gained_auto_achievements);

        // Two more periods at once → the next two in id order.
        game.tick_auto_achievements(60.0 * 60_000.0);
        assert!(game.achievement_unlocked(12));
        assert!(game.achievement_unlocked(13));
        assert!(!game.achievement_unlocked(14));
    }

    #[test]
    fn auto_achieve_toggle_pauses_the_timer() {
        let mut game = game_at_reality_goal();
        assert!(game.reality());
        game.reality.auto_achieve = false;
        game.tick_auto_achievements(120.0 * 60_000.0);
        assert!(!game.achievement_unlocked(11));
        // The timer holds at one period while off (`clampMax`).
        assert_eq!(game.reality.ach_timer, game.achievement_period_ms());
        // Switching it on grants on the next tick.
        game.reality.auto_achieve = true;
        game.tick_auto_achievements(0.0);
        assert!(game.achievement_unlocked(11));
    }

    #[test]
    fn reality_study_gating() {
        let mut game = GameState::new();
        game.time_theorems = Decimal::from_float(1e10);
        game.max_theorem = Decimal::from_float(1e10);
        game.dilation.studies = vec![1, 2, 3, 4, 5];
        game.records.this_reality.max_ep = Decimal::new(1.0, 4000);
        // Implemented pre-reality achievements are missing → blocked.
        assert!(!game.can_buy_dilation_study(6));
        for &id in IMPLEMENTED_ACHIEVEMENTS {
            game.unlock_achievement(id);
        }
        assert!(game.can_buy_dilation_study(6));
        assert!(game.buy_dilation_study(6));
        assert!(game.is_reality_available());
    }
}
