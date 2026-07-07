//! Pelle — The Doomed (Feature 7.7), the final Celestial. Dooming permanently
//! disables prior mechanics; Armageddon replaces prestige and grants Remnants,
//! which fund Reality Shards, five Rifts, Pelle Upgrades, the Galaxy Generator,
//! and the antimatter game-end. See `docs/design/2026-07-07-pelle.md`. Original:
//! `celestials/pelle/{pelle,rifts,strikes,galaxy-generator,game-end}.js`.
//!
//! **Scope.** The self-contained Pelle *layer* is ported: dooming/armageddon,
//! Remnants/Reality Shards, the 5 Rifts (fill/percentage/effect/milestones),
//! Strikes, Pelle Upgrades, the Galaxy Generator, and the game-end. The full
//! `isDisabled` disable-everything sweep is exposed as `pelle_is_disabled` but
//! only a subset of engine sites consult it (documented cut). Cosmetics (the
//! credits/song/`zalgo` text corruption) are cut.

use crate::state::GameState;
use break_infinity::Decimal;

// Rift indices.
pub const RIFT_VACUUM: usize = 0;
pub const RIFT_DECAY: usize = 1;
pub const RIFT_CHAOS: usize = 2;
pub const RIFT_RECURSION: usize = 3;
pub const RIFT_PARADOX: usize = 4;
pub const RIFT_COUNT: usize = 5;

/// Per-rift milestone percentage thresholds.
pub const RIFT_MILESTONES: [[f64; 3]; RIFT_COUNT] = [
    [0.04, 0.06, 0.4],   // vacuum
    [0.2, 0.6, 1.0],     // decay
    [0.09, 0.15, 1.0],   // chaos
    [0.10, 0.15, 1.0],   // recursion
    [0.15, 0.25, 0.5],   // paradox
];

/// Galaxy-Generator per-phase caps, sorted ascending (paradox 1e5 < vacuum 1000
/// … actually sorted): the phase advances through these thresholds.
pub const GG_THRESHOLDS: [(usize, f64); RIFT_COUNT] = [
    (RIFT_VACUUM, 1000.0),
    (RIFT_PARADOX, 1e5),
    (RIFT_DECAY, 1e7),
    (RIFT_CHAOS, 1e9),
    (RIFT_RECURSION, 1e10),
];

/// One Rift's saved state.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rift {
    #[cfg_attr(feature = "serde", serde(default))]
    pub fill: Decimal,
    #[cfg_attr(feature = "serde", serde(default))]
    pub active: bool,
    #[cfg_attr(feature = "serde", serde(default = "one_f64"))]
    pub reduced_to: f64,
    /// Decay's spent percentage (chaos drains it).
    #[cfg_attr(feature = "serde", serde(default))]
    pub percentage_spent: f64,
}

impl Default for Rift {
    fn default() -> Self {
        Self { fill: Decimal::ZERO, active: false, reduced_to: 1.0, percentage_spent: 0.0 }
    }
}

/// Pelle's doomed records (peak totals this doomed run).
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PelleRecords {
    #[cfg_attr(feature = "serde", serde(default))]
    pub total_antimatter: Decimal,
    #[cfg_attr(feature = "serde", serde(default))]
    pub total_infinity_points: Decimal,
    #[cfg_attr(feature = "serde", serde(default))]
    pub total_eternity_points: Decimal,
}

/// The Galaxy Generator's saved state.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GalaxyGenerator {
    #[cfg_attr(feature = "serde", serde(default))]
    pub unlocked: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub spent_galaxies: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub generated_galaxies: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub phase: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub sacrifice_active: bool,
}

/// `player.celestials.pelle`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PelleState {
    #[cfg_attr(feature = "serde", serde(default))]
    pub doomed: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub remnants: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub reality_shards: Decimal,
    #[cfg_attr(feature = "serde", serde(default))]
    pub records: PelleRecords,
    /// One-time Pelle Upgrades (bit set of ids 0–22).
    #[cfg_attr(feature = "serde", serde(default))]
    pub upgrades: u32,
    /// The 5 Pelle rebuyables (`antimatterDimensionMult`…`galaxyPower`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub rebuyables: [u32; 5],
    /// The 5 Galaxy-Generator rebuyables.
    #[cfg_attr(feature = "serde", serde(default))]
    pub gg_rebuyables: [u32; 5],
    #[cfg_attr(feature = "serde", serde(default))]
    pub rifts: [Rift; RIFT_COUNT],
    /// Strike bits (`progressBits`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub progress_bits: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub galaxy_generator: GalaxyGenerator,
}

fn one_f64() -> f64 {
    1.0
}

impl Default for PelleState {
    fn default() -> Self {
        Self::new()
    }
}

impl PelleState {
    pub fn new() -> Self {
        Self {
            doomed: false,
            remnants: 0.0,
            reality_shards: Decimal::ZERO,
            records: PelleRecords::default(),
            upgrades: 0,
            rebuyables: [0; 5],
            gg_rebuyables: [0; 5],
            rifts: Default::default(),
            progress_bits: 0,
            galaxy_generator: GalaxyGenerator::default(),
        }
    }
}

impl GameState {
    // --- Availability -----------------------------------------------------------

    /// `Pelle.isUnlocked` = Imaginary Upgrade 25 bought.
    pub fn pelle_unlocked(&self) -> bool {
        self.imaginary_upgrade_bought(25)
    }

    pub fn is_doomed(&self) -> bool {
        self.celestials.pelle.doomed
    }

    /// Whether a Pelle Strike (id 1–5) has been triggered.
    pub fn pelle_has_strike(&self, id: u32) -> bool {
        self.celestials.pelle.progress_bits & (1u32 << id) != 0
    }

    /// A one-time Pelle Upgrade (id 0–22) is bought.
    pub fn pelle_upgrade_bought(&self, id: u32) -> bool {
        self.celestials.pelle.upgrades & (1u32 << id) != 0
    }

    /// Whether a mechanic is disabled by the doom (`Pelle.isDisabled`). Only a
    /// subset of engine sites consult this in-frontier (documented).
    pub fn pelle_is_disabled(&self, mechanic: &str) -> bool {
        if !self.is_doomed() {
            return false;
        }
        match mechanic {
            // Glyphs come back with the vacuum rift's first milestone.
            "glyphs" => !self.pelle_rift_milestone(RIFT_VACUUM, 0),
            _ => true,
        }
    }

    // --- Dooming & Armageddon ---------------------------------------------------

    /// `Pelle.initializeRun`: doom the current Reality (permanent). Simplified —
    /// sets the flag and routes through the Reality reset. Returns whether it
    /// happened.
    pub fn doom_reality(&mut self) -> bool {
        if !self.pelle_unlocked() || self.is_doomed() {
            return false;
        }
        self.celestials.pelle.doomed = true;
        self.clear_celestial_runs();
        self.reset_reality();
        true
    }

    /// `Pelle.remnantsGain` from the doomed records.
    pub fn remnants_gain(&self) -> f64 {
        let p = &self.celestials.pelle;
        let mut am = (p.records.total_antimatter + Decimal::ONE).pos_log10();
        let mut ip = (p.records.total_infinity_points + Decimal::ONE).pos_log10();
        let mut ep = (p.records.total_eternity_points + Decimal::ONE).pos_log10();
        if self.pelle_has_strike(5) {
            am *= 500.0;
            ip *= 10.0;
            ep *= 5.0;
        }
        let gain = (((am + 2.0).log10() + (ip + 2.0).log10() + (ep + 2.0).log10()) / 1.64)
            .powf(7.5);
        if gain < 1.0 {
            gain
        } else {
            (gain - p.remnants).floor()
        }
    }

    pub fn can_armageddon(&self) -> bool {
        self.remnants_gain() >= 1.0
    }

    /// `Pelle.armageddon(true)`: bank Remnants and reset (a forced Reality).
    pub fn armageddon(&mut self, gain: bool) -> bool {
        if gain && !self.can_armageddon() {
            return false;
        }
        if gain {
            self.celestials.pelle.remnants += self.remnants_gain();
        }
        self.reset_reality();
        true
    }

    /// `Pelle.realityShardGain(r)`.
    fn reality_shard_gain(&self, remnants: f64) -> Decimal {
        (Decimal::pow10(remnants.powf(1.0 / 7.5) * 4.0) - Decimal::ONE)
            / Decimal::from_float(1e3)
    }

    pub fn reality_shard_gain_per_second(&self) -> Decimal {
        self.reality_shard_gain(self.celestials.pelle.remnants)
    }

    // --- Rifts ------------------------------------------------------------------

    /// Whether rift `i` is usable (its Strike is unlocked).
    pub fn pelle_rift_unlocked(&self, i: usize) -> bool {
        // Rift `i` maps to Strike `i + 1`.
        self.pelle_has_strike((i + 1) as u32)
    }

    /// `rift.percentage` — the clamped, spent-adjusted fill fraction.
    pub fn pelle_rift_percentage(&self, i: usize) -> f64 {
        let r = &self.celestials.pelle.rifts[i];
        if r.reduced_to > 1.0 {
            return r.reduced_to;
        }
        let raw = self.pelle_rift_real_percentage(i);
        if i == RIFT_DECAY {
            (raw - r.percentage_spent).min(r.reduced_to)
        } else {
            raw.min(r.reduced_to)
        }
    }

    fn pelle_rift_real_percentage(&self, i: usize) -> f64 {
        let fill = self.celestials.pelle.rifts[i].fill;
        let log = (fill + Decimal::ONE).pos_log10();
        match i {
            RIFT_VACUUM => (log * 10.0 + 1.0).log10().powf(2.5) / 100.0,
            RIFT_DECAY => log * 0.05 / 100.0,
            RIFT_CHAOS => fill.to_f64() / 10.0,
            RIFT_RECURSION => log.powf(0.4) / 4000f64.powf(0.4),
            RIFT_PARADOX => log / 100.0,
            _ => 0.0,
        }
    }

    fn pelle_rift_percentage_to_fill(&self, i: usize, pct: f64) -> Decimal {
        match i {
            RIFT_VACUUM => {
                let inner = 10f64.powf((pct * 100.0).powf(1.0 / 2.5)) / 10.0 - 0.1;
                Decimal::pow10(inner) - Decimal::ONE
            }
            RIFT_DECAY => Decimal::pow10(20.0 * pct * 100.0) - Decimal::ONE,
            RIFT_CHAOS => Decimal::from_float(10.0 * pct),
            RIFT_RECURSION => Decimal::pow10(pct.powf(2.5) * 4000.0) - Decimal::ONE,
            RIFT_PARADOX => Decimal::pow10(pct * 100.0) - Decimal::ONE,
            _ => Decimal::ZERO,
        }
    }

    /// `rift.effect(fill)` — the rift's primary effect value.
    pub(crate) fn pelle_rift_effect(&self, i: usize) -> Decimal {
        let fill = self.celestials.pelle.rifts[i].fill;
        let log = (fill + Decimal::ONE).pos_log10();
        match i {
            RIFT_VACUUM => (fill + Decimal::ONE).pow(&Decimal::from_float(0.33)),
            RIFT_DECAY => {
                if self.pelle_rift_milestone(RIFT_CHAOS, 0) {
                    Decimal::from_float((2001.0f64).sqrt())
                } else {
                    Decimal::from_float((log + 1.0).sqrt())
                }
            }
            RIFT_RECURSION => {
                Decimal::from_float(58.0 * log.powf(0.2) / 4000f64.powf(0.2))
            }
            RIFT_PARADOX => Decimal::from_float(1.0 + log * 0.004),
            RIFT_CHAOS => {
                let f = fill.to_f64();
                let fill = if f > 6.5 { (f - 6.5) / 7.0 + 6.5 } else { f };
                let a = 6f64.powf(6f64.powf(6f64.powf(fill / 10.0 + 0.1)) - 6.0) / 1e5;
                Decimal::from_float(a + 10f64.powf(fill / 10.0 + 0.1))
            }
            _ => Decimal::ONE,
        }
    }

    /// Whether rift `i`'s milestone `m` (0–2) is reached and its Strike is on.
    /// Chaos's first milestone forces every decay milestone "always active".
    pub fn pelle_rift_milestone(&self, i: usize, m: usize) -> bool {
        if i == RIFT_DECAY && self.pelle_chaos_forces_decay() {
            return true;
        }
        self.pelle_rift_unlocked(i) && self.pelle_rift_percentage(i) >= RIFT_MILESTONES[i][m]
    }

    fn pelle_chaos_forces_decay(&self) -> bool {
        self.pelle_rift_unlocked(RIFT_CHAOS)
            && self.pelle_rift_percentage(RIFT_CHAOS) >= RIFT_MILESTONES[RIFT_CHAOS][0]
    }

    /// Toggle a rift active (max 2 active).
    pub fn pelle_toggle_rift(&mut self, i: usize) -> bool {
        if !self.pelle_rift_unlocked(i) {
            return false;
        }
        let active = self.celestials.pelle.rifts.iter().filter(|r| r.active).count();
        let r = &mut self.celestials.pelle.rifts[i];
        if !r.active && active >= 2 {
            return false;
        }
        r.active = !r.active;
        true
    }

    /// The fill-currency drain for a rift (3%/s), banking into its fill.
    fn pelle_fill_rift(&mut self, i: usize, diff_ms: f64) {
        if !self.celestials.pelle.rifts[i].active || !self.pelle_rift_unlocked(i) {
            return;
        }
        if self.pelle_rift_percentage(i) >= 1.0 {
            self.celestials.pelle.rifts[i].active = false;
            return;
        }
        let drain = (1.0 - 0.03f64).powf(diff_ms / 1000.0);
        let max_fill = {
            let spent = self.celestials.pelle.rifts[i].percentage_spent;
            self.pelle_rift_percentage_to_fill(i, 1.0 + spent)
        };
        match i {
            RIFT_CHAOS => {
                // Chaos drains decay's percentage.
                let decay_pct = self.pelle_rift_percentage(RIFT_DECAY);
                if decay_pct <= 0.0 {
                    return;
                }
                let after = decay_pct * drain;
                let spent = decay_pct - after;
                self.celestials.pelle.rifts[RIFT_DECAY].percentage_spent += spent;
                let new_fill =
                    (self.celestials.pelle.rifts[i].fill + Decimal::from_float(spent)).min(&max_fill);
                self.celestials.pelle.rifts[i].fill = new_fill;
            }
            _ => {
                let value = self.pelle_fill_currency(i);
                if value <= Decimal::ONE {
                    return;
                }
                let after = value * Decimal::from_float(drain);
                let spent = value - after;
                let new_value = (value - spent).max(&Decimal::ONE);
                self.pelle_set_fill_currency(i, new_value);
                let new_fill = (self.celestials.pelle.rifts[i].fill + spent).min(&max_fill);
                self.celestials.pelle.rifts[i].fill = new_fill;
            }
        }
    }

    fn pelle_fill_currency(&self, i: usize) -> Decimal {
        match i {
            RIFT_VACUUM => self.infinity_points,
            RIFT_DECAY => self.replicanti.amount,
            RIFT_RECURSION => self.eternity_points,
            RIFT_PARADOX => self.dilation.dilated_time,
            _ => Decimal::ZERO,
        }
    }

    fn pelle_set_fill_currency(&mut self, i: usize, value: Decimal) {
        match i {
            RIFT_VACUUM => self.infinity_points = value,
            RIFT_DECAY => self.replicanti.amount = value,
            RIFT_RECURSION => self.eternity_points = value,
            RIFT_PARADOX => self.dilation.dilated_time = value,
            _ => {}
        }
    }

    // --- Strikes ----------------------------------------------------------------

    /// `PelleStrikes.<x>.trigger`: unlock Strike `id` (1–5) while doomed. On the
    /// dilation Strike (5) the records are reset to the dilation baseline.
    pub(crate) fn pelle_trigger_strike(&mut self, id: u32) {
        if !self.is_doomed() || self.pelle_has_strike(id) {
            return;
        }
        self.celestials.pelle.progress_bits |= 1u32 << id;
        if id == 5 {
            self.pelle_reset_records_for_dilation();
        }
    }

    fn pelle_reset_records_for_dilation(&mut self) {
        self.celestials.pelle.records.total_antimatter = Decimal::new(1.0, 180_000);
        self.celestials.pelle.records.total_infinity_points = Decimal::new(1.0, 60_000);
        self.celestials.pelle.records.total_eternity_points = Decimal::new(1.0, 1050);
    }

    // --- Pelle Upgrades ---------------------------------------------------------

    /// `Pelle.antimatterDimensionMult(x)` — the first rebuyable's AD multiplier.
    pub(crate) fn pelle_ad_mult(&self) -> Decimal {
        if !self.is_doomed() {
            return Decimal::ONE;
        }
        let x = self.celestials.pelle.rebuyables[0] as f64;
        Decimal::pow10(((x + 1.0).log10()) + x.powf(5.1) / 1e3 + 4f64.powf(x) / 1e19)
    }

    /// `PelleUpgrade.timeSpeedMult` — game-speed multiplier (1.3^x).
    pub(crate) fn pelle_time_speed_mult(&self) -> f64 {
        if !self.is_doomed() {
            return 1.0;
        }
        1.3f64.powi(self.celestials.pelle.rebuyables[1] as i32)
    }

    /// The rebuyable cost `base1^x · max(base2^(x−incScale), 1) · coeff`.
    pub fn pelle_rebuyable_cost(&self, id: usize) -> Decimal {
        // (base1, base2, incScale, coeff) per the source.
        const P: [(f64, f64, f64, f64); 5] = [
            (10.0, 1e3, 41.0, 100.0),
            (20.0, 1e3, 30.0, 1e5),
            (30.0, 1e3, 25.0, 1e15),
            (40.0, 1e3, 20.0, 1e18),
            (1000.0, 1e3, 10.0, 1e30),
        ];
        let x = self.celestials.pelle.rebuyables[id] as f64;
        let (b1, b2, inc, coeff) = P[id];
        Decimal::from_float(b1).pow(&Decimal::from_float(x))
            * Decimal::from_float(b2)
                .pow(&Decimal::from_float(x - inc))
                .max(&Decimal::ONE)
            * Decimal::from_float(coeff)
    }

    pub const PELLE_REBUYABLE_CAPS: [u32; 5] = [44, 35, 26, 21, 9];

    pub fn buy_pelle_rebuyable(&mut self, id: usize) -> bool {
        if id >= 5 || self.celestials.pelle.rebuyables[id] >= Self::PELLE_REBUYABLE_CAPS[id] {
            return false;
        }
        let cost = self.pelle_rebuyable_cost(id);
        if self.celestials.pelle.reality_shards < cost {
            return false;
        }
        self.celestials.pelle.reality_shards -= cost;
        self.celestials.pelle.rebuyables[id] += 1;
        true
    }

    /// One-time Pelle Upgrade costs (id → cost).
    pub fn pelle_upgrade_cost(&self, id: u32) -> Decimal {
        const COSTS: [f64; 23] = [
            1e5, 5e5, 5e6, 2.5e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e14, 1e15, 1e16, 1e17, 1e19,
            1e20, 1e21, 1e22, 1e24, 1e25, 1e26, 1e45, 1e50, 1e30,
        ];
        Decimal::from_float(COSTS.get(id as usize).copied().unwrap_or(f64::INFINITY))
    }

    pub fn buy_pelle_upgrade(&mut self, id: u32) -> bool {
        if !self.is_doomed() || self.pelle_upgrade_bought(id) || id >= 23 {
            return false;
        }
        let cost = self.pelle_upgrade_cost(id);
        if self.celestials.pelle.reality_shards < cost {
            return false;
        }
        self.celestials.pelle.reality_shards -= cost;
        self.celestials.pelle.upgrades |= 1u32 << id;
        true
    }

    // --- Galaxy Generator -------------------------------------------------------

    pub fn galaxy_generator_unlocked(&self) -> bool {
        self.celestials.pelle.galaxy_generator.unlocked
    }

    /// `GalaxyGenerator.gainPerSecond`.
    pub fn galaxy_generator_gain_per_second(&self) -> f64 {
        if !self.galaxy_generator_unlocked() {
            return 0.0;
        }
        let gg = &self.celestials.pelle.galaxy_generator;
        let additive = gg_effect(&self.celestials.pelle.gg_rebuyables, 0);
        let mult = gg_effect(&self.celestials.pelle.gg_rebuyables, 1)
            * gg_effect(&self.celestials.pelle.gg_rebuyables, 2)
            * gg_effect(&self.celestials.pelle.gg_rebuyables, 3)
            * gg_effect(&self.celestials.pelle.gg_rebuyables, 4);
        let _ = gg;
        additive * mult
    }

    /// The current phase's generation cap.
    pub fn galaxy_generator_cap(&self) -> f64 {
        GG_THRESHOLDS
            .get(self.celestials.pelle.galaxy_generator.phase as usize)
            .map(|(_, cap)| *cap)
            .unwrap_or(f64::INFINITY)
    }

    pub fn galaxy_generator_galaxies(&self) -> f64 {
        let gg = &self.celestials.pelle.galaxy_generator;
        gg.generated_galaxies - gg.spent_galaxies
    }

    /// `GalaxyGenerator.loop`: generate galaxies (capped), advance the sacrifice.
    pub(crate) fn galaxy_generator_loop(&mut self, diff_ms: f64) {
        if !self.galaxy_generator_unlocked() {
            return;
        }
        let cap = self.galaxy_generator_cap();
        if self.celestials.pelle.galaxy_generator.sacrifice_active {
            if let Some((rift, _)) =
                GG_THRESHOLDS.get(self.celestials.pelle.galaxy_generator.phase as usize)
            {
                let r = &mut self.celestials.pelle.rifts[*rift];
                r.reduced_to = (r.reduced_to - 0.03 * diff_ms / 1000.0).max(0.0);
                if r.reduced_to == 0.0 {
                    self.celestials.pelle.galaxy_generator.sacrifice_active = false;
                    self.celestials.pelle.galaxy_generator.phase += 1;
                }
            }
        }
        let gain = self.galaxy_generator_gain_per_second() * diff_ms / 1000.0;
        let gg = &mut self.celestials.pelle.galaxy_generator;
        gg.generated_galaxies = (gg.generated_galaxies + gain).min(cap);
    }

    pub fn galaxy_generator_start_sacrifice(&mut self) {
        self.celestials.pelle.galaxy_generator.sacrifice_active = true;
    }

    // --- Game loop & game-end ---------------------------------------------------

    /// `Pelle.gameLoop` + rift fills + the Galaxy-Generator loop, run each tick
    /// while doomed.
    pub(crate) fn pelle_tick(&mut self, diff_ms: f64) {
        if !self.is_doomed() {
            return;
        }
        // Reality Shards accrue from Remnants.
        let rs = self.reality_shard_gain_per_second() * Decimal::from_float(diff_ms / 1000.0);
        self.celestials.pelle.reality_shards += rs;
        // Track the doomed records (peak totals).
        self.celestials.pelle.records.total_antimatter =
            self.celestials.pelle.records.total_antimatter.max(&self.total_antimatter);
        self.celestials.pelle.records.total_infinity_points = self
            .celestials
            .pelle
            .records
            .total_infinity_points
            .max(&self.infinity_points);
        self.celestials.pelle.records.total_eternity_points = self
            .celestials
            .pelle
            .records
            .total_eternity_points
            .max(&self.eternity_points);
        // The ECs Strike (unlock the Recursion rift) fires at 115 Time Theorems.
        if self.time_theorems >= Decimal::from_float(115.0) {
            self.pelle_trigger_strike(4);
        }
        // Recursion's 3rd milestone permanently unlocks the Galaxy Generator.
        if self.pelle_rift_milestone(RIFT_RECURSION, 2) {
            self.celestials.pelle.galaxy_generator.unlocked = true;
        }
        // Fill the rifts.
        for i in 0..RIFT_COUNT {
            self.pelle_fill_rift(i, diff_ms);
        }
        self.galaxy_generator_loop(diff_ms);
        // Game-end check.
        if self.game_end_state() >= 1.0 {
            self.is_game_end = true;
        }
    }

    /// `GameEnd.endState` (the additional-end animation term is cut).
    pub fn game_end_state(&self) -> f64 {
        let total = self.celestials.pelle.records.total_antimatter;
        let inner = (total + Decimal::ONE).pos_log10();
        (((inner + 1.0).log10() - 8.7) / (9e15f64.log10() - 8.7)).max(0.0)
    }
}

/// A Galaxy-Generator rebuyable's effect (additive `2x`; the four mults).
fn gg_effect(rebuyables: &[u32; 5], id: usize) -> f64 {
    let x = rebuyables[id] as f64;
    match id {
        0 => x * 2.0,
        1 => 2.5f64.powf(x),
        _ => 2f64.powf(x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pelle_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game.reality.imaginary_upgrade_bits |= 1 << 25; // unlock Pelle
        game
    }

    #[test]
    fn dooming_sets_the_flag() {
        let mut game = pelle_game();
        assert!(!game.is_doomed());
        assert!(game.doom_reality());
        assert!(game.is_doomed());
        assert!(!game.doom_reality()); // already doomed
    }

    #[test]
    fn remnants_gain_scales_with_records() {
        let mut game = pelle_game();
        game.celestials.pelle.doomed = true;
        game.celestials.pelle.records.total_antimatter = Decimal::new(1.0, 3000);
        game.celestials.pelle.records.total_infinity_points = Decimal::new(1.0, 1000);
        game.celestials.pelle.records.total_eternity_points = Decimal::new(1.0, 100);
        assert!(game.remnants_gain() > 1.0);
        assert!(game.can_armageddon());
    }

    #[test]
    fn strikes_unlock_rifts() {
        let mut game = pelle_game();
        game.celestials.pelle.doomed = true;
        assert!(!game.pelle_rift_unlocked(RIFT_VACUUM));
        game.pelle_trigger_strike(1);
        assert!(game.pelle_rift_unlocked(RIFT_VACUUM));
        assert!(game.pelle_has_strike(1));
    }

    #[test]
    fn dilation_strike_resets_records() {
        let mut game = pelle_game();
        game.celestials.pelle.doomed = true;
        game.pelle_trigger_strike(5);
        assert_eq!(
            game.celestials.pelle.records.total_antimatter,
            Decimal::new(1.0, 180_000)
        );
    }

    #[test]
    fn pelle_rebuyable_respects_cap() {
        let mut game = pelle_game();
        game.celestials.pelle.doomed = true;
        game.celestials.pelle.reality_shards = Decimal::new(1.0, 10);
        game.celestials.pelle.rebuyables[4] = 9; // galaxyPower cap
        assert!(!game.buy_pelle_rebuyable(4));
    }

    #[test]
    fn game_ends_at_astronomical_antimatter() {
        let mut game = pelle_game();
        game.celestials.pelle.doomed = true;
        assert!(game.game_end_state() < 1.0);
        // The game-end antimatter is ~1e(9e15) (a log10-of-log10 threshold).
        game.celestials.pelle.records.total_antimatter = Decimal::new(1.0, 9_500_000_000_000_000);
        game.pelle_tick(1000.0);
        assert!(game.game_end_state() >= 1.0);
        assert!(game.is_game_end);
    }
}
