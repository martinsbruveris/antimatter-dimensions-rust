//! Lai'tela (Feature 7.6) — the Celestial of Dimensions. Four Dark Matter
//! Dimensions produce Dark Matter + Dark Energy in real time; Dark Energy
//! condenses into Singularities (`singularity.rs`); Continuum replaces buy-10
//! with continuous purchases; and Lai'tela's Reality is a destabilization run.
//! See `docs/design/2026-07-07-laitela.md`. Original:
//! `celestials/laitela/{laitela,dark-matter-dimension}.js`.
//!
//! **Scope.** The DMD economy (production/upgrades/ascension + the faithful
//! `maxAllDMDimensions` bulk-buy), annihilation, Singularities + the 30
//! milestones (`singularity.rs`), the exact Continuum (`getContinuumValue`
//! including the super-exponential branch and the `totalAmount` effective
//! amounts), the DMD/ascension/annihilation/condense autobuyers
//! (`autobuyers.rs`), and the entropy/destabilization run are ported.

use crate::celestials::singularity as sing;
use crate::state::GameState;
use break_infinity::Decimal;

/// Per-tier interval-cost geometric base for Power DE (`POWER_DE_COST_MULTS`).
const POWER_DE_COST_MULTS: [f64; 4] = [1.65, 1.6, 1.55, 1.5];
/// `adjustedStartingCost` tier exponent (`tiers` array, 1-indexed → 0-indexed).
const TIER_COST_EXP: [i32; 4] = [0, 2, 5, 13];
const INTERVAL_PER_UPGRADE: f64 = 0.92;
const POWER_DM_PER_ASCENSION: f64 = 500.0;
const POWER_DE_PER_ASCENSION: f64 = 500.0;
const INTERVAL_CAP_MS: f64 = 10.0;
/// The DM required to annihilate (`annihilationDMRequirement`).
pub const ANNIHILATION_DM: f64 = 1e60;

/// One Dark Matter Dimension (`player.celestials.laitela.dimensions[i]`).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DarkMatterDimension {
    #[cfg_attr(feature = "serde", serde(default))]
    pub amount: Decimal,
    #[cfg_attr(feature = "serde", serde(default))]
    pub interval_upgrades: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub power_dm_upgrades: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub power_de_upgrades: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub time_since_last_update: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub ascension_count: u32,
}

impl Default for DarkMatterDimension {
    fn default() -> Self {
        Self {
            amount: Decimal::ZERO,
            interval_upgrades: 0,
            power_dm_upgrades: 0,
            power_de_upgrades: 0,
            time_since_last_update: 0.0,
            ascension_count: 0,
        }
    }
}

/// `player.celestials.laitela`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LaitelaState {
    #[cfg_attr(feature = "serde", serde(default))]
    pub dark_matter: Decimal,
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_dark_matter: Decimal,
    #[cfg_attr(feature = "serde", serde(default))]
    pub dark_energy: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub singularities: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub singularity_cap_increases: u32,
    #[cfg_attr(feature = "serde", serde(default = "one_f64"))]
    pub dark_matter_mult: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub run: bool,
    /// Destabilization progress 0..1 (−1 = completed this Reality).
    #[cfg_attr(feature = "serde", serde(default))]
    pub entropy: f64,
    #[cfg_attr(feature = "serde", serde(default = "secs_3600"))]
    pub this_completion: f64,
    #[cfg_attr(feature = "serde", serde(default = "secs_3600"))]
    pub fastest_completion: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub difficulty_tier: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub dimensions: [DarkMatterDimension; 4],
    /// Continuum toggle (`player.auto.disableContinuum`, inverted).
    #[cfg_attr(feature = "serde", serde(default))]
    pub disable_continuum: bool,
}

fn one_f64() -> f64 {
    1.0
}
fn secs_3600() -> f64 {
    3600.0
}

impl Default for LaitelaState {
    fn default() -> Self {
        Self::new()
    }
}

impl LaitelaState {
    pub fn new() -> Self {
        Self {
            dark_matter: Decimal::ZERO,
            max_dark_matter: Decimal::ZERO,
            dark_energy: 0.0,
            singularities: 0.0,
            singularity_cap_increases: 0,
            dark_matter_mult: 1.0,
            run: false,
            entropy: 0.0,
            this_completion: 3600.0,
            fastest_completion: 3600.0,
            difficulty_tier: 0,
            dimensions: Default::default(),
            disable_continuum: false,
        }
    }
}

impl GameState {
    // --- Availability -----------------------------------------------------------

    /// `Laitela.isUnlocked` = Imaginary Upgrade 15 bought.
    pub fn laitela_unlocked(&self) -> bool {
        self.imaginary_upgrade_bought(15)
    }

    pub fn laitela_is_running(&self) -> bool {
        self.celestials.laitela.run
    }

    pub fn laitela_run_unlocked(&self) -> bool {
        self.laitela_unlocked()
    }

    /// `Laitela.maxAllowedDimension` = `8 − difficultyTier`.
    pub fn laitela_max_allowed_dimension(&self) -> u32 {
        8u32.saturating_sub(self.celestials.laitela.difficulty_tier)
    }

    pub fn laitela_is_fully_destabilized(&self) -> bool {
        self.laitela_max_allowed_dimension() == 0
    }

    /// Whether a dimension `tier` (1-indexed) is silenced by Lai'tela's run.
    pub(crate) fn laitela_dimension_disabled(&self, tier: u32) -> bool {
        self.celestials.laitela.run && tier > self.laitela_max_allowed_dimension()
    }

    /// `Laitela.realityReward` — the DMD boost from difficulty + fastest time.
    pub fn laitela_reality_reward(&self) -> f64 {
        let l = &self.celestials.laitela;
        (100f64.powi(l.difficulty_tier as i32) * (360.0 / l.fastest_completion).powi(2))
            .max(1.0)
    }

    // --- Dark Matter Dimensions -------------------------------------------------

    pub fn dmd_unlocked(&self, tier: usize) -> bool {
        // DMD (0-indexed) `tier` unlocks with Imaginary Upgrade `tier + 15`
        // (the original's `ImaginaryUpgrade(oneIndexedTier + 14)`).
        self.imaginary_upgrade_bought((tier as u8) + 15)
    }

    /// The common dark multiplier (product of the `darkFrom*` milestones).
    fn dmd_common_dark_mult(&self) -> f64 {
        [
            sing::DARK_FROM_TESSERACTS,
            sing::DARK_FROM_GLYPH_LEVEL,
            sing::DARK_FROM_THEOREMS,
            sing::DARK_FROM_DM4,
            sing::DARK_FROM_GAMESPEED,
            sing::DARK_FROM_DILATED_TIME,
        ]
        .iter()
        .map(|&id| self.singularity_milestone_effect_or(id, 1.0))
        .product()
    }

    fn dmd_power_dm_per_ascension(&self) -> f64 {
        POWER_DM_PER_ASCENSION
            + self.singularity_milestone_effect_or(sing::IMPROVED_ASCENSION_DM, 0.0)
    }

    /// `DarkMatterDimension.powerDM` (Dark Matter production per amount·tick).
    pub fn dmd_power_dm(&self, tier: usize) -> Decimal {
        if !self.dmd_unlocked(tier) {
            return Decimal::ZERO;
        }
        let d = &self.celestials.laitela.dimensions[tier];
        let base = 1.0 + 2.0 * 1.15f64.powi(d.power_dm_upgrades as i32);
        let mut mult = Decimal::from_float(base)
            * Decimal::from_float(self.laitela_reality_reward())
            * Decimal::from_float(self.celestials.laitela.dark_matter_mult)
            * Decimal::from_float(self.dmd_common_dark_mult())
            * Decimal::from_float(
                self.dmd_power_dm_per_ascension()
                    .powi(d.ascension_count as i32),
            );
        mult *= Decimal::from_float(
            self.singularity_milestone_effect_or(sing::DARK_MATTER_MULT, 1.0),
        );
        mult *= Decimal::from_float(
            self.singularity_milestone_effect_or(sing::MULT_FROM_INFINITIED, 1.0),
        );
        mult /= Decimal::from_float(1e4f64.powf((tier as f64).powf(0.5)));
        mult
    }

    /// `DarkMatterDimension.powerDE` (Dark Energy production per tick).
    pub fn dmd_power_de(&self, tier: usize) -> f64 {
        if !self.dmd_unlocked(tier) {
            return 0.0;
        }
        let d = &self.celestials.laitela.dimensions[tier];
        let tier_factor = 15f64.powi(tier as i32);
        let destabilize = if self.laitela_is_fully_destabilized() {
            8.0
        } else {
            1.0
        };
        ((1.0 + d.power_de_upgrades as f64 * 0.1)
            * 1.005f64.powi(d.power_de_upgrades as i32)
            * tier_factor
            / 1000.0)
            * self.dmd_common_dark_mult()
            * POWER_DE_PER_ASCENSION.powi(d.ascension_count as i32)
            * self.singularity_milestone_effect_or(sing::DARK_ENERGY_MULT, 1.0)
            * self.singularity_milestone_effect_or(sing::REALITY_DE_MULTIPLIER, 1.0)
            * self.singularity_milestone_effect_or(sing::MULT_FROM_INFINITIED, 1.0)
            * destabilize
    }

    /// `DarkMatterDimension.rawInterval` (ms).
    fn dmd_raw_interval(&self, tier: usize) -> f64 {
        let d = &self.celestials.laitela.dimensions[tier];
        1000.0
            * 4f64.powi(tier as i32)
            * INTERVAL_PER_UPGRADE.powi(d.interval_upgrades as i32)
            * self
                .singularity_milestone_effect_or(
                    sing::ASCENSION_INTERVAL_SCALING,
                    1200.0,
                )
                .powi(d.ascension_count as i32)
            * self
                .singularity_milestone_effect_or(sing::DARK_DIM_INTERVAL_REDUCTION, 1.0)
    }

    /// `DarkMatterDimension.interval` (floored at 10 ms). `tier` is 0-indexed.
    pub fn dmd_interval(&self, tier: usize) -> f64 {
        self.dmd_raw_interval(tier).max(INTERVAL_CAP_MS)
    }

    fn dmd_adjusted_starting_cost(&self, tier: usize) -> Decimal {
        Decimal::from_float(10.0)
            * Decimal::from_float(1200.0).pow(&Decimal::from(TIER_COST_EXP[tier] as u64))
            * Decimal::from_float(
                self.singularity_milestone_effect_or(sing::DARK_DIM_COST_REDUCTION, 1.0),
            )
    }

    fn dmd_interval_cost_increase(&self) -> f64 {
        5f64.powf(
            self.singularity_milestone_effect_or(
                sing::INTERVAL_COST_SCALING_REDUCTION,
                1.0,
            ),
        )
    }

    pub fn dmd_interval_cost(&self, tier: usize) -> Decimal {
        let d = &self.celestials.laitela.dimensions[tier];
        (Decimal::from_float(self.dmd_interval_cost_increase())
            .pow(&Decimal::from(d.interval_upgrades as u64))
            * self.dmd_adjusted_starting_cost(tier)
            * Decimal::from_float(10.0))
        .floor()
    }

    pub fn dmd_power_dm_cost(&self, tier: usize) -> Decimal {
        let d = &self.celestials.laitela.dimensions[tier];
        (Decimal::from_float(10.0).pow(&Decimal::from(d.power_dm_upgrades as u64))
            * self.dmd_adjusted_starting_cost(tier)
            * Decimal::from_float(10.0))
        .floor()
    }

    pub fn dmd_power_de_cost(&self, tier: usize) -> Decimal {
        let d = &self.celestials.laitela.dimensions[tier];
        (Decimal::from_float(POWER_DE_COST_MULTS[tier])
            .pow(&Decimal::from(d.power_de_upgrades as u64))
            * self.dmd_adjusted_starting_cost(tier)
            * Decimal::from_float(10.0))
        .floor()
    }

    pub fn dmd_can_buy_interval(&self, tier: usize) -> bool {
        self.celestials.laitela.dark_matter >= self.dmd_interval_cost(tier)
            && self.dmd_interval(tier) > INTERVAL_CAP_MS
    }

    pub fn dmd_buy_interval(&mut self, tier: usize) -> bool {
        if !self.dmd_can_buy_interval(tier) {
            return false;
        }
        let cost = self.dmd_interval_cost(tier);
        self.celestials.laitela.dark_matter -= cost;
        self.celestials.laitela.dimensions[tier].interval_upgrades += 1;
        true
    }

    pub fn dmd_buy_power_dm(&mut self, tier: usize) -> bool {
        let cost = self.dmd_power_dm_cost(tier);
        if self.celestials.laitela.dark_matter < cost {
            return false;
        }
        self.celestials.laitela.dark_matter -= cost;
        self.celestials.laitela.dimensions[tier].power_dm_upgrades += 1;
        true
    }

    pub fn dmd_buy_power_de(&mut self, tier: usize) -> bool {
        let cost = self.dmd_power_de_cost(tier);
        if self.celestials.laitela.dark_matter < cost {
            return false;
        }
        self.celestials.laitela.dark_matter -= cost;
        self.celestials.laitela.dimensions[tier].power_de_upgrades += 1;
        true
    }

    /// `rawIntervalCost` — the unfloored working cost the bulk-buy math uses.
    fn dmd_raw_interval_cost(&self, tier: usize) -> Decimal {
        let d = &self.celestials.laitela.dimensions[tier];
        Decimal::from_float(self.dmd_interval_cost_increase())
            .pow(&Decimal::from(d.interval_upgrades as u64))
            * self.dmd_adjusted_starting_cost(tier)
            * Decimal::from_float(10.0)
    }

    fn dmd_raw_power_dm_cost(&self, tier: usize) -> Decimal {
        let d = &self.celestials.laitela.dimensions[tier];
        Decimal::from_float(10.0).pow(&Decimal::from(d.power_dm_upgrades as u64))
            * self.dmd_adjusted_starting_cost(tier)
            * Decimal::from_float(10.0)
    }

    fn dmd_raw_power_de_cost(&self, tier: usize) -> Decimal {
        let d = &self.celestials.laitela.dimensions[tier];
        Decimal::from_float(POWER_DE_COST_MULTS[tier])
            .pow(&Decimal::from(d.power_de_upgrades as u64))
            * self.dmd_adjusted_starting_cost(tier)
            * Decimal::from_float(10.0)
    }

    /// `maxIntervalPurchases`: how many more interval upgrades fit before the
    /// 10 ms cap.
    fn dmd_max_interval_purchases(&self, tier: usize) -> f64 {
        (INTERVAL_CAP_MS / self.dmd_interval(tier)).ln() / INTERVAL_PER_UPGRADE.ln()
    }

    /// `buyManyInterval(x)`: cumulative-geometric bulk purchase.
    fn dmd_buy_many_interval(&mut self, tier: usize, x: f64) -> bool {
        if x > self.dmd_max_interval_purchases(tier).ceil() {
            return false;
        }
        let increase = self.dmd_interval_cost_increase();
        let cost = (self.dmd_raw_interval_cost(tier)
            * (Decimal::from_float(increase).pow(&Decimal::from_float(x))
                - Decimal::ONE)
            / Decimal::from_float(increase - 1.0))
        .floor();
        if self.celestials.laitela.dark_matter < cost {
            return false;
        }
        self.celestials.laitela.dark_matter -= cost;
        self.celestials.laitela.dimensions[tier].interval_upgrades += x as u32;
        true
    }

    fn dmd_buy_many_power_dm(&mut self, tier: usize, x: f64) -> bool {
        let cost = (self.dmd_raw_power_dm_cost(tier)
            * (Decimal::from_float(10.0).pow(&Decimal::from_float(x)) - Decimal::ONE)
            / Decimal::from_float(9.0))
        .floor();
        if self.celestials.laitela.dark_matter < cost {
            return false;
        }
        self.celestials.laitela.dark_matter -= cost;
        self.celestials.laitela.dimensions[tier].power_dm_upgrades += x as u32;
        true
    }

    fn dmd_buy_many_power_de(&mut self, tier: usize, x: f64) -> bool {
        let increase = POWER_DE_COST_MULTS[tier];
        let cost = (self.dmd_raw_power_de_cost(tier)
            * (Decimal::from_float(increase).pow(&Decimal::from_float(x))
                - Decimal::ONE)
            / Decimal::from_float(increase - 1.0))
        .floor();
        if self.celestials.laitela.dark_matter < cost {
            return false;
        }
        self.celestials.laitela.dark_matter -= cost;
        self.celestials.laitela.dimensions[tier].power_de_upgrades += x as u32;
        true
    }

    /// `Laitela.maxAllDMDimensions(maxTier)`: for each unlocked DMD up to
    /// `max_tier` (1-indexed count), bulk-buy every upgrade type up to 2% of
    /// the *initial* Dark Matter, then greedily buy the cheapest single
    /// upgrades while the (initial-snapshot) balance still covers them — as
    /// the original, whose working list keeps advancing its cost/remaining
    /// entries even when the live purchase fails.
    pub fn laitela_max_all_dm_dimensions(&mut self, max_tier: u32) {
        // Working entries: (tier, kind 0/1/2, cost, cost increase, remaining).
        let mut upgrades: Vec<(usize, u8, Decimal, f64, f64)> = Vec::new();
        #[allow(clippy::needless_range_loop)]
        for tier in 0..(max_tier as usize).min(4) {
            if !self.dmd_unlocked(tier) {
                continue;
            }
            upgrades.push((
                tier,
                0,
                self.dmd_raw_interval_cost(tier),
                self.dmd_interval_cost_increase(),
                self.dmd_max_interval_purchases(tier).ceil(),
            ));
            upgrades.push((tier, 1, self.dmd_raw_power_dm_cost(tier), 10.0, f64::MAX));
            upgrades.push((
                tier,
                2,
                self.dmd_raw_power_de_cost(tier),
                POWER_DE_COST_MULTS[tier],
                f64::MAX,
            ));
        }
        let dark_matter = self.celestials.laitela.dark_matter;
        let buy = |game: &mut Self, u: &mut (usize, u8, Decimal, f64, f64), x: f64| {
            match u.1 {
                0 => game.dmd_buy_many_interval(u.0, x),
                1 => game.dmd_buy_many_power_dm(u.0, x),
                _ => game.dmd_buy_many_power_de(u.0, x),
            };
            u.2 *= Decimal::from_float(u.3).pow(&Decimal::from_float(x));
            u.4 -= x;
        };
        // Bulk pass: everything costing less than 2% of the snapshot.
        for u in upgrades.iter_mut() {
            let affordable = (dark_matter * Decimal::from_float(0.02) / u.2)
                .max(&Decimal::ONE)
                .ln()
                / u.3.ln();
            let purchases = affordable.floor().clamp(0.0, u.4);
            buy(self, u, purchases);
        }
        // Greedy pass against the snapshot balance.
        while upgrades.iter().any(|u| u.2 <= dark_matter && u.4 > 0.0) {
            let cheapest = upgrades
                .iter_mut()
                .filter(|u| u.4 > 0.0)
                .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
                .expect("non-empty by the while condition");
            buy(self, cheapest, 1.0);
        }
    }

    /// `ascend()`: only when the interval is maxed; bumps ascension and re-buys
    /// interval upgrades to bring the (now-jumped) interval back down.
    pub fn dmd_ascend(&mut self, tier: usize) -> bool {
        if self.dmd_interval(tier) > INTERVAL_CAP_MS {
            return false;
        }
        self.celestials.laitela.dimensions[tier].ascension_count += 1;
        while self.dmd_buy_interval(tier) {}
        true
    }

    /// The "Max all" button: the faithful `maxAllDMDimensions` over every
    /// tier.
    pub fn laitela_max_all_dmd(&mut self) {
        self.laitela_max_all_dm_dimensions(4);
    }

    /// `DarkMatterDimensions.tick(realDiff)` — top-down DM/DE production.
    pub(crate) fn dmd_tick(&mut self, real_diff_ms: f64) {
        if !self.laitela_unlocked() {
            return;
        }
        for tier in (0..4).rev() {
            if !self.dmd_unlocked(tier) {
                continue;
            }
            let interval = self.dmd_interval(tier);
            self.celestials.laitela.dimensions[tier].time_since_last_update +=
                real_diff_ms;
            let tsu = self.celestials.laitela.dimensions[tier].time_since_last_update;
            if interval < tsu {
                let ticks = (tsu / interval).floor();
                let power_dm = self.dmd_power_dm(tier);
                let power_de = self.dmd_power_de(tier);
                let amount = self.celestials.laitela.dimensions[tier].amount;
                let production_dm = amount * Decimal::from_float(ticks) * power_dm;
                if tier == 0 {
                    self.celestials.laitela.dark_matter += production_dm;
                    self.celestials.laitela.max_dark_matter = self
                        .celestials
                        .laitela
                        .max_dark_matter
                        .max(&self.celestials.laitela.dark_matter);
                } else {
                    self.celestials.laitela.dimensions[tier - 1].amount += production_dm;
                }
                self.celestials.laitela.dark_energy += ticks * power_de;
                self.celestials.laitela.dimensions[tier].time_since_last_update -=
                    interval * ticks;
            }
        }
        // `dim4Generation`: passive DM4 once annihilation is available.
        if self.singularity_milestone_unlocked(sing::DIM4_GENERATION)
            && self.annihilation_unlocked()
        {
            let gain = self.singularity_milestone_effect_or(sing::DIM4_GENERATION, 0.0)
                * real_diff_ms
                / 1000.0;
            self.celestials.laitela.dimensions[3].amount += Decimal::from_float(gain);
        }
    }

    // --- Annihilation -----------------------------------------------------------

    pub fn annihilation_unlocked(&self) -> bool {
        self.imaginary_upgrade_bought(19)
    }

    /// `darkMatterMultGain` — the annihilation reward.
    pub fn dark_matter_mult_gain(&self) -> f64 {
        let ratio =
            self.celestials.laitela.dark_matter / Decimal::from_float(ANNIHILATION_DM);
        let base = (ratio.pos_log10() + 1.0).max(0.0).powf(1.5);
        // Imaginary Upgrade 21: improved by iM.
        let iu21 = if self.imaginary_upgrade_applies(21) {
            ((self.reality.imaginary_machines.pos_log10() - 10.0).powi(3)).max(1.0)
        } else {
            1.0
        };
        base * iu21
    }

    pub fn can_annihilate(&self) -> bool {
        self.annihilation_unlocked()
            && self.celestials.laitela.dark_matter
                >= Decimal::from_float(ANNIHILATION_DM)
    }

    /// `Laitela.annihilate(force)`: bank the mult gain and reset the DMDs.
    pub fn annihilate(&mut self, force: bool) -> bool {
        if !force && !self.can_annihilate() {
            return false;
        }
        self.celestials.laitela.dark_matter_mult += self.dark_matter_mult_gain();
        self.dmd_reset();
        // Achievement 176: annihilate your Dark Matter Dimensions.
        self.check_annihilation_achievements();
        true
    }

    /// `DarkMatterDimensions.reset`: amounts → 1, upgrades/ascension → 0, DM → 0.
    fn dmd_reset(&mut self) {
        for d in self.celestials.laitela.dimensions.iter_mut() {
            d.amount = Decimal::ONE;
            d.interval_upgrades = 0;
            d.power_dm_upgrades = 0;
            d.power_de_upgrades = 0;
            d.time_since_last_update = 0.0;
            d.ascension_count = 0;
        }
        self.celestials.laitela.dark_matter = Decimal::ZERO;
    }

    // --- Continuum --------------------------------------------------------------

    pub fn continuum_unlocked(&self) -> bool {
        self.imaginary_upgrade_bought(15) && !self.pelle_is_disabled("continuum")
    }

    pub fn continuum_active(&self) -> bool {
        self.continuum_unlocked() && !self.celestials.laitela.disable_continuum
    }

    pub fn set_continuum(&mut self, on: bool) {
        self.celestials.laitela.disable_continuum = !on;
        // Enabling Continuum fails Imaginary Upgrade 21's "never enabled"
        // requirement for the rest of the Reality.
        if on {
            self.requirement_checks.reality_no_continuum = false;
        }
    }

    /// `Laitela.matterExtraPurchaseFactor`.
    pub fn matter_extra_purchase_factor(&self) -> f64 {
        let log_max = self.celestials.laitela.max_dark_matter.pos_log10();
        1.0 + 0.5
            * (log_max / 50.0).powf(0.4)
            * (1.0 + self.singularity_milestone_effect_or(sing::CONTINUUM_MULT, 0.0))
    }

    /// The continuum buy-10 value for AD `tier` (0-indexed):
    /// `costScale.getContinuumValue(currencyAmount, 10) ×
    /// matterExtraPurchaseFactor` — including the super-exponential branch
    /// past 1.8e308 (`get_continuum_value`'s quadratic).
    pub(crate) fn ad_continuum_value(&self, tier: usize) -> f64 {
        if !self.dim_available_for_purchase(tier) {
            return 0.0;
        }
        // Enslaved limits dim 8 to 1 purchase; Continuum mirrors that.
        if tier == 7 && self.celestials.enslaved.run {
            return 1.0;
        }
        self.ad_cost_scale_for_tier(tier)
            .get_continuum_value(self.dim_currency_amount(tier), 10.0)
            * self.matter_extra_purchase_factor()
    }

    /// The continuum value for Tickspeed (perSet = 1, base 1000).
    pub(crate) fn tickspeed_continuum_value(&self) -> f64 {
        self.tickspeed_cost_scale()
            .get_continuum_value(self.antimatter, 1.0)
            * self.matter_extra_purchase_factor()
    }

    // --- Entropy / run ----------------------------------------------------------

    /// `entropyGainPerSecond`.
    fn entropy_gain_per_second(&self) -> f64 {
        ((self.antimatter + Decimal::ONE).pos_log10() / 1e11)
            .powi(2)
            .clamp(0.0, 100.0)
            / 200.0
    }

    /// `laitelaRealityTick`: advance entropy; on completion record the time and,
    /// if under 30 s, raise the difficulty tier. Called on real time.
    pub(crate) fn laitela_reality_tick(&mut self, real_diff_ms: f64) {
        if !self.celestials.laitela.run {
            return;
        }
        if self.celestials.laitela.entropy >= 0.0 {
            self.celestials.laitela.entropy +=
                (real_diff_ms / 1000.0) * self.entropy_gain_per_second();
        }
        if self.celestials.laitela.entropy >= 1.0 {
            self.celestials.laitela.entropy = -1.0;
            let secs = self.records.this_reality.real_time_ms / 1000.0;
            self.celestials.laitela.this_completion = secs;
            self.celestials.laitela.fastest_completion =
                self.celestials.laitela.fastest_completion.min(secs);
            self.clear_celestial_runs();
            if secs < 30.0 {
                self.celestials.laitela.difficulty_tier += 1;
                self.celestials.laitela.fastest_completion = 300.0;
            }
        }
    }

    /// Reset entropy when entering Lai'tela's Reality.
    pub(crate) fn laitela_on_enter(&mut self) {
        self.celestials.laitela.entropy = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn laitela_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        // Unlock Lai'tela + all 4 DMDs.
        for id in 15..=18 {
            game.reality.imaginary_upgrade_bits |= 1u32 << id;
            // Seed the DMD amount like the upgrade purchase does.
            game.celestials.laitela.dimensions[id - 15].amount = Decimal::ONE;
        }
        game
    }

    #[test]
    fn continuum_uses_the_quadratic_branch_past_the_threshold() {
        let mut game = laitela_game();
        game.reality.imaginary_upgrade_bits |= 1 << 15; // continuum unlock... (iU15)
        game.celestials.laitela.max_dark_matter = Decimal::new(1.0, 100);
        // Below the scaling threshold the linear branch applies.
        game.antimatter = Decimal::new(1.0, 100);
        let linear = game.ad_continuum_value(0);
        assert!(linear > 0.0);
        // Far past 1.8e308 the quadratic branch flattens growth: the value is
        // well below the linear extrapolation.
        game.antimatter = Decimal::new(1.0, 1000);
        let quad = game.ad_continuum_value(0);
        let factor = game.matter_extra_purchase_factor();
        let linear_extrapolation =
            (1.0 + (1000.0 - 1.0 - 10f64.log10()) / 1e3f64.log10()) * factor;
        assert!(quad > linear);
        assert!(quad < linear_extrapolation);
        // The effective amount follows the continuum purchases.
        assert!(game.continuum_active());
        assert_eq!(
            game.ad_total_amount(0),
            Decimal::from_float((10.0 * quad).floor())
        );
    }

    #[test]
    fn max_all_buys_the_cheap_upgrades() {
        let mut game = laitela_game();
        game.celestials.laitela.dark_matter = Decimal::from_float(1e7);
        game.laitela_max_all_dm_dimensions(4);
        let d = &game.celestials.laitela.dimensions[0];
        assert!(d.interval_upgrades > 0);
        assert!(d.power_dm_upgrades > 0);
        assert!(d.power_de_upgrades > 0);
        assert!(game.celestials.laitela.dark_matter < Decimal::from_float(1e7));
    }

    #[test]
    fn annihilation_and_condense_autobuyers_fire() {
        let mut game = laitela_game();
        game.autobuyers.enabled = true;
        // Annihilation: milestone 4e18 singularities + iU19 + gain over 1.05.
        game.celestials.laitela.singularities = 4e18;
        game.reality.imaginary_upgrade_bits |= 1 << 19;
        game.celestials.laitela.dark_matter = Decimal::new(1.0, 70);
        game.autobuyers.annihilation_active = true;
        let mult_before = game.celestials.laitela.dark_matter_mult;
        game.tick_autobuyers(50.0);
        assert!(game.celestials.laitela.dark_matter_mult > mult_before);
        // Condense: dark energy over cap × autoCondense factor. (The gained
        // singularities vanish in f64 precision next to 4e18, so the Dark
        // Energy reset is the observable.)
        game.autobuyers.singularity_active = true;
        game.celestials.laitela.dark_energy = game.singularity_cap() * 2.0;
        game.tick_autobuyers(50.0);
        assert_eq!(game.celestials.laitela.dark_energy, 0.0);
    }

    #[test]
    fn dmd_tick_produces_dark_matter_and_energy() {
        let mut game = laitela_game();
        // A long tick fires the fast tier-1 interval many times.
        game.dmd_tick(5000.0);
        assert!(game.celestials.laitela.dark_matter > Decimal::ZERO);
        assert!(game.celestials.laitela.dark_energy > 0.0);
    }

    #[test]
    fn annihilation_needs_unlock_and_dm() {
        let mut game = laitela_game();
        assert!(!game.can_annihilate()); // no iU19
        game.reality.imaginary_upgrade_bits |= 1u32 << 19;
        game.celestials.laitela.dark_matter = Decimal::new(1.0, 61);
        assert!(game.can_annihilate());
        let before = game.celestials.laitela.dark_matter_mult;
        assert!(game.annihilate(false));
        assert!(game.celestials.laitela.dark_matter_mult > before);
        // DMDs reset to amount 1.
        assert_eq!(game.celestials.laitela.dimensions[0].amount, Decimal::ONE);
    }

    #[test]
    fn destabilization_raises_difficulty_on_fast_completion() {
        let mut game = laitela_game();
        game.celestials.laitela.run = true;
        // Entropy at the completion threshold (the gain rate only matters at
        // deep-endgame antimatter).
        game.celestials.laitela.entropy = 1.0;
        game.records.this_reality.real_time_ms = 10_000.0; // 10 s
        game.laitela_reality_tick(2000.0);
        assert_eq!(game.celestials.laitela.difficulty_tier, 1);
        assert_eq!(game.laitela_max_allowed_dimension(), 7);
        assert!(!game.celestials.laitela.run); // exited
    }

    #[test]
    fn continuum_value_grows_with_antimatter() {
        let mut game = laitela_game();
        game.antimatter = Decimal::new(1.0, 100);
        game.celestials.laitela.max_dark_matter = Decimal::new(1.0, 20);
        let v = game.ad_continuum_value(0);
        assert!(v > 0.0);
    }
}
