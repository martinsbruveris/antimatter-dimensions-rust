//! Infinity Upgrades: the 16-cell grid of one-shot upgrades bought with Infinity
//! Points (Feature 2.2).
//!
//! The original stores ownership as a `Set` of string ids (`player.infinityUpgrades`);
//! we use a `u32` bitmask on [`GameState`], one bit per [`InfinityUpgrade`]. The
//! grid is four columns of four; each column is an independent chain purchasable
//! top-to-bottom (an upgrade requires the one directly above it). Ownership persists
//! across a Big Crunch (reset only on Eternity, a later feature).
//!
//! This module owns the upgrade vocabulary (the enum + the static data table), the
//! purchase logic, and the small *effect readers* that the rest of the engine calls
//! (`buy_ten_multiplier`, `dim_boost_power`, `galaxy_strength_effect`,
//! `reset_boost_reduction`, the AD-multiplier contributions, `skip_resets_if_possible`).
//! The effect *application* lives at the original's sites — dimension multiplier,
//! tickspeed, dim-boost/galaxy requirements — see
//! `docs/design/2026-07-03-infinity-upgrades.md`.
//!
//! The bottom row — the `ipMult` rebuyable and the one-time `ipOffline`, both
//! gated by Achievement 41 — is also modelled here: `ip_mult_purchases` /
//! `ip_offline_bought` on [`GameState`], the two-regime cost curve
//! (`InfinityIPMultUpgrade`: ×10 steps to 1e3M, ×1e10 steps to the 1e6M cap),
//! and `buy_max_ip_mult`'s two-phase geometric-series bulk buy. The `ipMult`
//! effect (`2^purchases`) applies in `total_ip_mult` (crunch.rs); `ipOffline`'s
//! IP award applies in the offline catch-up (tick.rs).

use break_infinity::Decimal;

use crate::break_infinity_upgrades::BreakInfinityUpgrade;
use crate::data::constants::{BUY_TEN_MULTIPLIER, DIM_BOOST_MULTIPLIER};
use crate::state::GameState;

/// The 16 grid upgrades, in column-major order (column 0 rows 0..4, column 1
/// rows 0..4, …). The `as usize` discriminant is the bitmask bit index; a
/// prerequisite is the previous variant in the same column (index not divisible
/// by 4). Save round-trip uses [`InfinityUpgrade::save_id`], never the bit index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InfinityUpgrade {
    // Column 0
    TotalTimeMult,
    Dim18Mult,
    Dim36Mult,
    ResetBoost,
    // Column 1
    Buy10Mult,
    Dim27Mult,
    Dim45Mult,
    GalaxyBoost,
    // Column 2
    ThisInfinityTimeMult,
    UnspentIpMult,
    DimboostMult,
    IpGen,
    // Column 3
    SkipReset1,
    SkipReset2,
    SkipReset3,
    SkipResetGalaxy,
}

/// Number of grid upgrades.
pub const INFINITY_UPGRADE_COUNT: usize = 16;

/// All 16 upgrades in bit-index order (used for iteration in the snapshot and
/// save round-trip).
pub const ALL_INFINITY_UPGRADES: [InfinityUpgrade; INFINITY_UPGRADE_COUNT] = [
    InfinityUpgrade::TotalTimeMult,
    InfinityUpgrade::Dim18Mult,
    InfinityUpgrade::Dim36Mult,
    InfinityUpgrade::ResetBoost,
    InfinityUpgrade::Buy10Mult,
    InfinityUpgrade::Dim27Mult,
    InfinityUpgrade::Dim45Mult,
    InfinityUpgrade::GalaxyBoost,
    InfinityUpgrade::ThisInfinityTimeMult,
    InfinityUpgrade::UnspentIpMult,
    InfinityUpgrade::DimboostMult,
    InfinityUpgrade::IpGen,
    InfinityUpgrade::SkipReset1,
    InfinityUpgrade::SkipReset2,
    InfinityUpgrade::SkipReset3,
    InfinityUpgrade::SkipResetGalaxy,
];

/// IP cost of each upgrade, indexed by bit index (`InfinityUpgrade as usize`).
/// Verified against `secret-formula/infinity/infinity-upgrades.js`.
const INFINITY_UPGRADE_COSTS: [Decimal; INFINITY_UPGRADE_COUNT] = [
    Decimal::new_unchecked(1.0, 0), // timeMult
    Decimal::new_unchecked(1.0, 0), // 18Mult
    Decimal::new_unchecked(1.0, 0), // 36Mult
    Decimal::new_unchecked(1.0, 0), // resetBoost
    Decimal::new_unchecked(1.0, 0), // dimMult (buy10)
    Decimal::new_unchecked(1.0, 0), // 27Mult
    Decimal::new_unchecked(1.0, 0), // 45Mult
    Decimal::new_unchecked(2.0, 0), // galaxyBoost
    Decimal::new_unchecked(3.0, 0), // timeMult2 (thisInfinity)
    Decimal::new_unchecked(5.0, 0), // unspentBonus
    Decimal::new_unchecked(7.0, 0), // resetMult (dimboost)
    Decimal::new_unchecked(1.0, 1), // passiveGen (ipGen), 10
    Decimal::new_unchecked(2.0, 1), // skipReset1, 20
    Decimal::new_unchecked(4.0, 1), // skipReset2, 40
    Decimal::new_unchecked(8.0, 1), // skipReset3, 80
    Decimal::new_unchecked(3.0, 2), // skipResetGalaxy, 300
];

/// Original save id of each upgrade, indexed by bit index. These are the strings
/// stored in `player.infinityUpgrades`.
const INFINITY_UPGRADE_SAVE_IDS: [&str; INFINITY_UPGRADE_COUNT] = [
    "timeMult",
    "18Mult",
    "36Mult",
    "resetBoost",
    "dimMult",
    "27Mult",
    "45Mult",
    "galaxyBoost",
    "timeMult2",
    "unspentBonus",
    "resetMult",
    "passiveGen",
    "skipReset1",
    "skipReset2",
    "skipReset3",
    "skipResetGalaxy",
];

/// Below this best-infinity time (ms) `ipGen` stops generating — the original's
/// `player.records.bestInfinity.time >= 999999999999` cutoff.
const IP_GEN_TOO_SLOW_MS: f64 = 999_999_999_999.0;

/// The `ipMult` rebuyable's `costIncreaseThreshold` (`DC.E3E6`): past it the
/// per-purchase cost ratio steepens from ×10 to ×1e10.
const IP_MULT_COST_INCREASE_THRESHOLD: Decimal = Decimal::new_unchecked(1.0, 3_000_000);

/// The `ipMult` rebuyable's `costCap` (`DC.E6E6`): once the next cost reaches
/// it the upgrade is capped.
const IP_MULT_COST_CAP: Decimal = Decimal::new_unchecked(1.0, 6_000_000);

/// `purchasesAtIncrease = costIncreaseThreshold.log10() - 1`.
const IP_MULT_PURCHASES_AT_INCREASE: u32 = 2_999_999;

/// IP cost of the one-time `ipOffline` upgrade.
const IP_OFFLINE_COST: Decimal = Decimal::new_unchecked(1.0, 3);

impl InfinityUpgrade {
    /// Bitmask bit for this upgrade (its slot in `GameState::infinity_upgrades`).
    pub fn bit(self) -> u32 {
        1 << (self as u32)
    }

    /// IP cost.
    pub fn cost(self) -> Decimal {
        INFINITY_UPGRADE_COSTS[self as usize]
    }

    /// Original save id (the string stored in `player.infinityUpgrades`).
    pub fn save_id(self) -> &'static str {
        INFINITY_UPGRADE_SAVE_IDS[self as usize]
    }

    /// The upgrade directly above this one in its column, which must be owned
    /// before this can be bought. `None` for the top of a column.
    pub fn requires(self) -> Option<InfinityUpgrade> {
        let idx = self as usize;
        if idx.is_multiple_of(4) {
            None
        } else {
            Some(ALL_INFINITY_UPGRADES[idx - 1])
        }
    }

    /// Resolve an original save id to its upgrade, if modelled.
    pub fn from_save_id(id: &str) -> Option<InfinityUpgrade> {
        ALL_INFINITY_UPGRADES
            .iter()
            .copied()
            .find(|u| u.save_id() == id)
    }
}

impl GameState {
    /// Whether `upgrade` is owned.
    pub fn infinity_upgrade_bought(&self, upgrade: InfinityUpgrade) -> bool {
        self.infinity_upgrades & upgrade.bit() != 0
    }

    /// Whether `upgrade` can be bought right now: not already owned, its column
    /// prerequisite owned, and enough Infinity Points. Mirrors the original's
    /// `canBeBought` (Pelle/charge conditions omitted — not modelled).
    pub fn can_buy_infinity_upgrade(&self, upgrade: InfinityUpgrade) -> bool {
        if self.infinity_upgrade_bought(upgrade) {
            return false;
        }
        if let Some(req) = upgrade.requires() {
            if !self.infinity_upgrade_bought(req) {
                return false;
            }
        }
        self.infinity_points >= upgrade.cost()
    }

    /// Buy `upgrade`, spending its IP cost. Returns whether the purchase happened.
    /// Buying a skip-reset upgrade applies it retroactively (the original calls
    /// `skipResetsIfPossible` from `purchase`).
    pub fn buy_infinity_upgrade(&mut self, upgrade: InfinityUpgrade) -> bool {
        if !self.can_buy_infinity_upgrade(upgrade) {
            return false;
        }
        self.infinity_points -= upgrade.cost();
        self.infinity_upgrades |= upgrade.bit();
        // Achievement 41: buy 16 Infinity Upgrades. The original counts
        // `player.infinityUpgrades.size`, which holds both the grid upgrades and
        // the Break Infinity upgrades (a single string set); here they are two
        // bitmasks, so sum their popcounts. Its reward unlocks the bottom row
        // (`ipMult` + `ipOffline`).
        if self.infinity_upgrades.count_ones()
            + self.break_infinity_upgrades.count_ones()
            >= 16
        {
            self.unlock_achievement(41);
        }
        if matches!(
            upgrade,
            InfinityUpgrade::SkipReset1
                | InfinityUpgrade::SkipReset2
                | InfinityUpgrade::SkipReset3
                | InfinityUpgrade::SkipResetGalaxy
        ) {
            self.skip_resets_if_possible();
        }
        true
    }

    // --- The bottom row: the `ipMult` rebuyable + `ipOffline` ---------------

    /// Whether the `ipMult` cost has passed `costIncreaseThreshold` (1e3M) into
    /// the steep ×1e10 regime (`hasIncreasedCost`).
    fn ip_mult_has_increased_cost(&self) -> bool {
        self.ip_mult_purchases >= IP_MULT_PURCHASES_AT_INCREASE
    }

    /// The current per-purchase cost ratio (`costIncrease`): ×10, steepening to
    /// ×1e10 past the threshold.
    fn ip_mult_cost_increase(&self) -> f64 {
        if self.ip_mult_has_increased_cost() {
            1e10
        } else {
            10.0
        }
    }

    /// The next `ipMult` purchase's IP cost (`InfinityIPMultUpgrade.cost`).
    pub fn ip_mult_cost(&self) -> Decimal {
        if self.ip_mult_has_increased_cost() {
            let excess = (self.ip_mult_purchases - IP_MULT_PURCHASES_AT_INCREASE) as f64;
            IP_MULT_COST_INCREASE_THRESHOLD
                * Decimal::from_float(1e10).pow(&Decimal::from_float(excess))
        } else {
            Decimal::pow10((self.ip_mult_purchases + 1) as f64)
        }
    }

    /// Whether `ipMult` has hit its cost cap of 1e6M (`isCapped`); a capped
    /// upgrade displays as bought.
    pub fn ip_mult_capped(&self) -> bool {
        self.ip_mult_cost() >= IP_MULT_COST_CAP
    }

    /// `InfinityIPMultUpgrade.canBeBought`: not Doomed, not capped, Achievement
    /// 41 unlocked, and the next purchase affordable.
    pub fn can_buy_ip_mult(&self) -> bool {
        !self.is_doomed()
            && !self.ip_mult_capped()
            && self.infinity_points >= self.ip_mult_cost()
            && self.achievement_unlocked(41)
    }

    /// Buy `amount` `ipMult` purchases at the current scaling
    /// (`InfinityIPMultUpgrade.purchase`). Only ever called with `amount = 1` or
    /// from `buy_max_ip_mult` under conditions that keep the scaling constant
    /// across the whole batch. Without TS181 each purchase also doubles the Big
    /// Crunch autobuyer's dynamic amount.
    fn purchase_ip_mult(&mut self, amount: f64) {
        if !self.can_buy_ip_mult() {
            return;
        }
        if !self.time_study_bought(181) {
            let bump = Decimal::from_float(2.0).pow(&Decimal::from_float(amount));
            self.bump_big_crunch_amount(bump);
        }
        let cost = Decimal::sum_geometric_series(
            amount,
            &self.ip_mult_cost(),
            &Decimal::from_float(self.ip_mult_cost_increase()),
            0.0,
        );
        self.infinity_points -= cost;
        self.ip_mult_purchases += amount as u32;
    }

    /// Buy a single `ipMult` purchase. Returns whether one was bought.
    pub fn buy_ip_mult(&mut self) -> bool {
        if !self.can_buy_ip_mult() {
            return false;
        }
        self.purchase_ip_mult(1.0);
        true
    }

    /// `InfinityIPMultUpgrade.buyMax`: bulk-buy in two phases — up to the
    /// `costIncreaseThreshold` at ×10 scaling, then (deliberately *not* an
    /// `else`) up to the cap at ×1e10 — so a balance spanning the threshold is
    /// processed on both sides in one call.
    pub fn buy_max_ip_mult(&mut self) {
        if !self.can_buy_ip_mult() {
            return;
        }
        if !self.ip_mult_has_increased_cost() {
            // Only IP below the threshold participates in this phase.
            let available = self.infinity_points.min(&IP_MULT_COST_INCREASE_THRESHOLD);
            let purchases = Decimal::afford_geometric_series(
                &available,
                &self.ip_mult_cost(),
                &Decimal::from_float(10.0),
                0.0,
            );
            if purchases <= 0.0 {
                return;
            }
            self.purchase_ip_mult(purchases);
        }
        if self.ip_mult_has_increased_cost() {
            let available = self.infinity_points.min(&IP_MULT_COST_CAP);
            let purchases = Decimal::afford_geometric_series(
                &available,
                &self.ip_mult_cost(),
                &Decimal::from_float(1e10),
                0.0,
            );
            if purchases <= 0.0 {
                return;
            }
            self.purchase_ip_mult(purchases);
        }
    }

    /// The `ipMult` effect for display: `2^purchases`, or the flat `1e1000000`
    /// once cost-capped purchases pass 3.3M (see `total_ip_mult` for the applied
    /// form, which also carries Effarig's cap).
    pub fn ip_mult_effect(&self) -> Decimal {
        if self.ip_mult_purchases >= 3_300_000 {
            Decimal::new_unchecked(1.0, 1_000_000)
        } else {
            Decimal::from_float(2.0).pow(&Decimal::from(self.ip_mult_purchases as u64))
        }
    }

    /// IP cost of the one-time `ipOffline` upgrade.
    pub fn ip_offline_cost(&self) -> Decimal {
        IP_OFFLINE_COST
    }

    /// `ipOffline`'s displayed effect: IP gained per minute while away
    /// (`bestIPMsWithoutMaxAll × 60000 / 2`).
    pub fn ip_offline_effect_per_min(&self) -> Decimal {
        self.records.this_eternity.best_ip_ms_without_max_all
            * Decimal::from_float(30_000.0)
    }

    /// Whether the one-time `ipOffline` upgrade can be bought: Achievement 41
    /// unlocked (`checkRequirement`), not already owned, and affordable.
    pub fn can_buy_ip_offline(&self) -> bool {
        !self.ip_offline_bought
            && self.achievement_unlocked(41)
            && self.infinity_points >= IP_OFFLINE_COST
    }

    /// Buy the `ipOffline` upgrade. Returns whether the purchase happened.
    pub fn buy_ip_offline(&mut self) -> bool {
        if !self.can_buy_ip_offline() {
            return false;
        }
        self.infinity_points -= IP_OFFLINE_COST;
        self.ip_offline_bought = true;
        true
    }

    // --- Effect readers -----------------------------------------------------

    /// The buy-10 base multiplier: `2`, raised to `2 × 1.1 = 2.2` once
    /// `buy10Mult` is owned (original `AntimatterDimensions.buyTenMultiplier`).
    ///
    /// Normal Challenge 7 overrides it to `min(2, 1 + dim_boosts / 5)` and, per the
    /// original, ignores the `buy10Mult` upgrade ("unaffected by any upgrades").
    pub fn buy_ten_multiplier(&self) -> Decimal {
        if self.challenge_running(7) {
            let value = (1.0 + self.dim_boosts as f64 / 5.0).min(2.0);
            return Decimal::from_float(value);
        }
        // EC3's reward adds +0.72/completion to the base before the ×1.1;
        // Achievement 141 adds a further +0.1 (`plusEffectsOf`).
        let ach141 = if self.achievement_unlocked(141) {
            0.1
        } else {
            0.0
        };
        let mut mult =
            Decimal::from_float(BUY_TEN_MULTIPLIER + self.ec3_buy_ten_bonus() + ach141);
        if self.infinity_upgrade_bought(InfinityUpgrade::Buy10Mult) {
            mult *= Decimal::from_float(1.1);
        }
        // Achievement 58 (NC9 in ≤ 3 min): +1% to the buy-10 multiplier.
        if self.achievement_unlocked(58) {
            mult *= Decimal::from_float(1.01);
        }
        // The `powerbuy10` glyph effect multiplies the base; `effarigforgotten`
        // raises the whole multiplier to a power.
        mult *= Decimal::from_float(self.glyph_effect_powerbuy10());
        let forgotten = self.glyph_effect_effarigforgotten();
        if forgotten != 1.0 {
            mult = mult.pow(&Decimal::from_float(forgotten));
        }
        mult
    }

    /// The Dimension-Boost power (`DimBoost.power = Effects.max(2, dimboostMult=2.5,
    /// IC7.reward=4, IC7=10, …)`): `2` base, `2.5` with the `dimboostMult` Infinity
    /// Upgrade, `4` once Infinity Challenge 7 is completed, and `10` while IC7 runs.
    /// Normal Challenge 8 overrides it to `1` (Dimension Boosts give no multiplier).
    pub fn dim_boost_power(&self) -> Decimal {
        if self.challenge_running(8) {
            return Decimal::ONE;
        }
        let mut power = DIM_BOOST_MULTIPLIER;
        if self.infinity_upgrade_bought(InfinityUpgrade::DimboostMult) {
            power = power.max(2.5);
        }
        if self.infinity_challenge_completed(7) {
            power = power.max(4.0);
        }
        if self.infinity_challenge_running(7) {
            power = power.max(10.0);
        }
        // TS81 raises the base to ×10 (`Effects.max`).
        if self.time_study_bought(81) {
            power = power.max(10.0);
        }
        let mut boost = Decimal::from_float(power);
        // TS83: ×1.0004 per free tickspeed upgrade, capped at 1e30.
        if self.time_study_bought(83) {
            boost *= Decimal::from_float(1.0004)
                .pow(&Decimal::from(self.total_tick_gained))
                .min(&Decimal::new_unchecked(1.0, 30));
        }
        // TS231: Dimension Boosts are stronger based on their amount.
        if self.time_study_bought(231) {
            boost *= Decimal::from(self.dim_boosts as u64)
                .pow(&Decimal::from_float(0.3))
                .max(&Decimal::ONE);
        }
        // Achievement 117: the Dimension-Boost → AD multiplier is 1% higher.
        if self.achievement_unlocked(117) {
            boost *= Decimal::from_float(1.01);
        }
        // Achievement 142 (unlock the Automator): Dimension Boosts ×1.5.
        if self.achievement_unlocked(142) {
            boost *= Decimal::from_float(1.5);
        }
        // The `powerdimboost` glyph effect (`GlyphEffect.dimBoostPower`).
        boost *= Decimal::from_float(self.glyph_effect_powerdimboost());
        boost
    }

    /// The galaxy-strength effect that scales the tickspeed formula's per-galaxy
    /// term — the original's `effects` product: `×2` from the Infinity Upgrade
    /// `galaxyBoost` and `×1.5` from the Break Infinity Upgrade `postGalaxy`.
    pub fn galaxy_strength_effect(&self) -> f64 {
        let infinity_boost =
            if self.infinity_upgrade_bought(InfinityUpgrade::GalaxyBoost) {
                2.0
            } else {
                1.0
            };
        // Infinity Challenge 5's reward makes all Galaxies 10% stronger.
        let ic5 = if self.infinity_challenge_completed(5) {
            1.1
        } else {
            1.0
        };
        // Achievements 86 and 178: all Galaxies 1% stronger each.
        let ach86 = if self.achievement_unlocked(86) {
            1.01
        } else {
            1.0
        };
        let ach178 = if self.achievement_unlocked(178) {
            1.01
        } else {
            1.0
        };
        infinity_boost * self.break_infinity_galaxy_boost() * ic5 * ach86 * ach178
    }

    /// Amount subtracted from the Dimension-Boost and Antimatter-Galaxy
    /// requirements: `9` once `resetBoost` is owned, else `0`.
    pub fn reset_boost_reduction(&self) -> u64 {
        if self.infinity_upgrade_bought(InfinityUpgrade::ResetBoost) {
            9
        } else {
            0
        }
    }

    /// `dimInfinityMult = infinities × 0.2 + 1` — the multiplier the
    /// `dim{18,27,36,45}mult` upgrades apply to their tier pairs.
    fn dim_infinity_mult(&self) -> Decimal {
        self.infinities * Decimal::from_float(0.2) + Decimal::ONE
    }

    /// `totalTimeMult` effect value: `(totalTimePlayed_minutes / 2) ^ 0.15`.
    fn total_time_mult_effect(&self) -> Decimal {
        let minutes = self.records.total_time_played_ms / 60_000.0;
        Decimal::from_float((minutes / 2.0).powf(0.15))
    }

    /// `thisInfinityTimeMult` effect value: `max((thisInfinity_minutes / 4) ^ 0.25, 1)`.
    fn this_infinity_time_mult_effect(&self) -> Decimal {
        let minutes = self.records.this_infinity.time_ms / 60_000.0;
        Decimal::from_float((minutes / 4.0).powf(0.25).max(1.0))
    }

    /// `unspentIPMult` effect value: `(infinityPoints / 2) ^ 1.5 + 1`.
    fn unspent_ip_mult_effect(&self) -> Decimal {
        let base = self.infinity_points / Decimal::from_float(2.0);
        base.pow(&Decimal::from_float(1.5)) + Decimal::ONE
    }

    /// The Infinity-Upgrade multiplier applied to **every** Antimatter Dimension
    /// (the `totalTimeMult` / `thisInfinityTimeMult` terms of the original's
    /// `antimatterDimensionCommonMultiplier`).
    pub fn infinity_upgrade_common_mult(&self) -> Decimal {
        let mut mult = Decimal::ONE;
        if self.infinity_upgrade_bought(InfinityUpgrade::TotalTimeMult) {
            mult *= self.total_time_mult_effect();
        }
        if self.infinity_upgrade_bought(InfinityUpgrade::ThisInfinityTimeMult) {
            mult *= self.this_infinity_time_mult_effect();
        }
        mult
    }

    /// The Infinity-Upgrade multiplier applied to a single tier (0-indexed): the
    /// `dim{18,27,36,45}mult` for its tier pair, plus `unspentIPMult` on tier 0.
    pub fn infinity_upgrade_tier_mult(&self, tier: usize) -> Decimal {
        let mut mult = Decimal::ONE;

        if let Some(u) = dim_pair_upgrade(tier) {
            if self.infinity_upgrade_bought(u) {
                mult *= self.dim_infinity_mult();
            }
        }

        // unspentIPMult: 1st dimension only.
        if tier == 0 && self.infinity_upgrade_bought(InfinityUpgrade::UnspentIpMult) {
            mult *= self.unspent_ip_mult_effect();
        }

        mult
    }

    /// The numeric effect value of `upgrade` for **display** (the number the
    /// original's `formatEffect` shows). Constant-effect upgrades return their
    /// display target (`buy10Mult` → 2.2, `dimboostMult` → 2.5, `resetBoost` → 9,
    /// `galaxyBoost` → 2, `ipGen` → totalIPMult); skip-resets return 1 (their tile
    /// shows only a description). Independent of whether the upgrade is owned.
    pub fn infinity_upgrade_effect(&self, upgrade: InfinityUpgrade) -> Decimal {
        use InfinityUpgrade::*;
        match upgrade {
            TotalTimeMult => self.total_time_mult_effect(),
            ThisInfinityTimeMult => self.this_infinity_time_mult_effect(),
            UnspentIpMult => self.unspent_ip_mult_effect(),
            Dim18Mult | Dim27Mult | Dim36Mult | Dim45Mult => self.dim_infinity_mult(),
            Buy10Mult => Decimal::from_float(2.2),
            DimboostMult => Decimal::from_float(2.5),
            ResetBoost => Decimal::from_float(9.0),
            GalaxyBoost => Decimal::from_float(2.0),
            IpGen => self.total_ip_mult(),
            SkipReset1 | SkipReset2 | SkipReset3 | SkipResetGalaxy => Decimal::ONE,
        }
    }

    /// Apply the skip-reset upgrades: raise `dim_boosts` to the highest owned
    /// skip level (and give a first Galaxy for `skipResetGalaxy`). Mirrors the
    /// original's `skipResetsIfPossible`; called after each reset and when a skip
    /// upgrade is bought. Only ever raises state.
    ///
    /// No-op while a challenge is running: the original guards on
    /// `Player.isInAntimatterChallenge`, since a challenge run always starts fresh
    /// (skip-resets and the free Galaxy would defeat the challenge's reset).
    pub fn skip_resets_if_possible(&mut self) {
        if self.in_any_antimatter_challenge() {
            return;
        }
        if self.infinity_upgrade_bought(InfinityUpgrade::SkipResetGalaxy)
            && self.dim_boosts < 4
        {
            self.dim_boosts = 4;
            if self.galaxies == 0 {
                self.galaxies = 1;
            }
        } else if self.infinity_upgrade_bought(InfinityUpgrade::SkipReset3)
            && self.dim_boosts < 3
        {
            self.dim_boosts = 3;
        } else if self.infinity_upgrade_bought(InfinityUpgrade::SkipReset2)
            && self.dim_boosts < 2
        {
            self.dim_boosts = 2;
        } else if self.infinity_upgrade_bought(InfinityUpgrade::SkipReset1)
            && self.dim_boosts < 1
        {
            self.dim_boosts = 1;
        }
    }

    /// Passive Infinity-Point generation from `ipGen`, called once per `tick`.
    /// Generates `totalIPMult` (= 1 pre-upgrades) IP every `bestInfinity.time ×
    /// 10` ms, accumulating fractional progress in `part_infinity_point`.
    /// Disabled while the best infinity is slower than the too-slow cutoff or has
    /// never happened. Mirrors `preProductionGenerateIP`.
    pub(crate) fn generate_passive_ip(&mut self, dt_ms: f64) {
        // Source 1: the `ipGen` *Infinity* Upgrade — generate `totalIPMult` IP
        // every `bestInfinity.time × 10` ms, banking fractional progress in
        // `part_infinity_point`. Disabled once the best infinity is too slow.
        if self.infinity_upgrade_bought(InfinityUpgrade::IpGen) {
            let best = self.records.best_infinity.time_ms;
            let gen_period = best * 10.0;
            if best < IP_GEN_TOO_SLOW_MS && gen_period > 0.0 {
                self.part_infinity_point += dt_ms / gen_period;
                let whole = self.part_infinity_point.floor();
                if whole >= 1.0 {
                    self.part_infinity_point -= whole;
                    self.infinity_points +=
                        Decimal::from_float(whole) * self.total_ip_mult();
                }
            }
        }
        // Source 2: the `ipGen` *Break Infinity* Upgrade (rebuyable
        // `infinityRebuyables[2]`) — passively add `bestRunIPPM · upgrades/20` IP
        // per minute. This is `preProductionGenerateIP`'s trailing add, applied
        // independently of source 1.
        let ip_gen = self.infinity_rebuyables[2];
        if ip_gen > 0 {
            let rate = self.best_run_ippm() * Decimal::from_float(ip_gen as f64 / 20.0);
            self.infinity_points += rate * Decimal::from_float(dt_ms / 60_000.0);
        }
    }

    /// Passive Infinity generation (`game.js`, the `!EternityChallenge(4)` block):
    /// the `infinitiedGen` Break Infinity Upgrade grants `0.5 × dt / max(50,
    /// bestInfinity.time)` Infinities per tick; the whole part is banked and the
    /// fraction carried in `part_infinitied`. The RealityUpgrade-5/7, Ra, glyph,
    /// RU11, and Effarig terms are 1/absent for the pre-Reality saves this covers.
    pub(crate) fn generate_passive_infinities(&mut self, dt_ms: f64) {
        // The whole block (including the `part_infinitied` carry) is skipped in EC4.
        if self.ec_running(4) {
            return;
        }
        let mut inf_gen = 0.0;
        if self.break_infinity_upgrade_bought(BreakInfinityUpgrade::InfinitiedGen) {
            inf_gen += 0.5 * dt_ms / self.records.best_infinity.time_ms.max(50.0);
        }
        inf_gen += self.part_infinitied;
        let whole = inf_gen.floor();
        if whole > 0.0 {
            self.infinities += Decimal::from_float(whole);
        }
        self.part_infinitied = inf_gen - whole;
    }
}

/// The dim-pair `dim{18,27,36,45}mult` upgrade for a 0-indexed tier, if any.
fn dim_pair_upgrade(tier: usize) -> Option<InfinityUpgrade> {
    match tier {
        0 | 7 => Some(InfinityUpgrade::Dim18Mult),
        1 | 6 => Some(InfinityUpgrade::Dim27Mult),
        2 | 5 => Some(InfinityUpgrade::Dim36Mult),
        3 | 4 => Some(InfinityUpgrade::Dim45Mult),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maxed_ip(game: &mut GameState) {
        game.infinity_points = Decimal::from_float(1e9);
    }

    #[test]
    fn column_prerequisites_gate_purchase() {
        let mut game = GameState::new();
        maxed_ip(&mut game);

        // Second cell in a column needs the first.
        assert!(!game.can_buy_infinity_upgrade(InfinityUpgrade::Dim18Mult));
        assert!(game.buy_infinity_upgrade(InfinityUpgrade::TotalTimeMult));
        assert!(game.can_buy_infinity_upgrade(InfinityUpgrade::Dim18Mult));
        assert!(game.buy_infinity_upgrade(InfinityUpgrade::Dim18Mult));
        assert!(game.infinity_upgrade_bought(InfinityUpgrade::Dim18Mult));
    }

    #[test]
    fn purchase_spends_ip_and_is_idempotent() {
        let mut game = GameState::new();
        game.infinity_points = Decimal::from_float(2.0);

        assert!(game.buy_infinity_upgrade(InfinityUpgrade::Buy10Mult)); // cost 1
        assert_eq!(game.infinity_points, Decimal::ONE);
        // Already owned → not buyable again.
        assert!(!game.buy_infinity_upgrade(InfinityUpgrade::Buy10Mult));
    }

    #[test]
    fn cannot_buy_without_enough_ip() {
        let mut game = GameState::new();
        game.infinity_points = Decimal::from_float(1.0);
        // galaxyBoost costs 2 and needs dim45mult; even ignoring the prereq, IP is
        // short.
        assert!(!game.can_buy_infinity_upgrade(InfinityUpgrade::GalaxyBoost));
    }

    #[test]
    fn buy10_and_dimboost_effects() {
        let mut game = GameState::new();
        maxed_ip(&mut game);
        assert_eq!(game.buy_ten_multiplier(), Decimal::from_float(2.0));
        assert_eq!(game.dim_boost_power(), Decimal::from_float(2.0));

        game.buy_infinity_upgrade(InfinityUpgrade::Buy10Mult);
        assert_eq!(game.buy_ten_multiplier(), Decimal::from_float(2.2));

        // dimboostMult is column 2 row 2; buy the chain above it.
        game.buy_infinity_upgrade(InfinityUpgrade::ThisInfinityTimeMult);
        game.buy_infinity_upgrade(InfinityUpgrade::UnspentIpMult);
        game.buy_infinity_upgrade(InfinityUpgrade::DimboostMult);
        assert_eq!(game.dim_boost_power(), Decimal::from_float(2.5));
    }

    #[test]
    fn reset_boost_and_galaxy_strength() {
        let mut game = GameState::new();
        maxed_ip(&mut game);
        assert_eq!(game.reset_boost_reduction(), 0);
        assert_eq!(game.galaxy_strength_effect(), 1.0);

        // resetBoost: column 0 row 3.
        game.buy_infinity_upgrade(InfinityUpgrade::TotalTimeMult);
        game.buy_infinity_upgrade(InfinityUpgrade::Dim18Mult);
        game.buy_infinity_upgrade(InfinityUpgrade::Dim36Mult);
        game.buy_infinity_upgrade(InfinityUpgrade::ResetBoost);
        assert_eq!(game.reset_boost_reduction(), 9);

        // galaxyBoost: column 1 row 3.
        game.buy_infinity_upgrade(InfinityUpgrade::Buy10Mult);
        game.buy_infinity_upgrade(InfinityUpgrade::Dim27Mult);
        game.buy_infinity_upgrade(InfinityUpgrade::Dim45Mult);
        game.buy_infinity_upgrade(InfinityUpgrade::GalaxyBoost);
        assert_eq!(game.galaxy_strength_effect(), 2.0);
    }

    #[test]
    fn dim_infinity_mult_scales_with_infinities() {
        let mut game = GameState::new();
        maxed_ip(&mut game);
        game.infinities = Decimal::from_float(10.0);
        game.buy_infinity_upgrade(InfinityUpgrade::TotalTimeMult);
        game.buy_infinity_upgrade(InfinityUpgrade::Dim18Mult);
        // infinities * 0.2 + 1 = 3, applied to tiers 0 and 7.
        assert_eq!(game.infinity_upgrade_tier_mult(0), Decimal::from_float(3.0));
        assert_eq!(game.infinity_upgrade_tier_mult(7), Decimal::from_float(3.0));
        // Tier 1 (a 27-pair) is unaffected.
        assert_eq!(game.infinity_upgrade_tier_mult(1), Decimal::ONE);
    }

    #[test]
    fn skip_resets_raise_boosts_on_purchase() {
        let mut game = GameState::new();
        maxed_ip(&mut game);
        game.buy_infinity_upgrade(InfinityUpgrade::SkipReset1);
        // Buying skipReset1 immediately grants the first boost.
        assert_eq!(game.dim_boosts, 1);
        game.buy_infinity_upgrade(InfinityUpgrade::SkipReset2);
        assert_eq!(game.dim_boosts, 2);
        game.buy_infinity_upgrade(InfinityUpgrade::SkipReset3);
        game.buy_infinity_upgrade(InfinityUpgrade::SkipResetGalaxy);
        assert_eq!(game.dim_boosts, 4);
        assert_eq!(game.galaxies, 1);
    }

    #[test]
    fn save_id_round_trips() {
        for u in ALL_INFINITY_UPGRADES {
            assert_eq!(InfinityUpgrade::from_save_id(u.save_id()), Some(u));
        }
        assert_eq!(InfinityUpgrade::from_save_id("nonexistent"), None);
    }

    #[test]
    fn buy10_mult_raises_dimension_multiplier() {
        let mut game = GameState::new();
        maxed_ip(&mut game);
        game.dimensions[0].bought = 10; // one buy-10 group
        let before = game.dimension_multiplier(0);
        game.buy_infinity_upgrade(InfinityUpgrade::Buy10Mult);
        // base 2 → 2.2 for the single group.
        assert!(game.dimension_multiplier(0) > before);
    }

    #[test]
    fn reset_boost_reduces_boost_and_galaxy_requirements() {
        let mut game = GameState::new();
        maxed_ip(&mut game);
        let boost_before = game.dim_boost_requirement().1;
        let galaxy_before = game.galaxy_requirement();

        game.buy_infinity_upgrade(InfinityUpgrade::TotalTimeMult);
        game.buy_infinity_upgrade(InfinityUpgrade::Dim18Mult);
        game.buy_infinity_upgrade(InfinityUpgrade::Dim36Mult);
        game.buy_infinity_upgrade(InfinityUpgrade::ResetBoost);

        assert_eq!(game.dim_boost_requirement().1, boost_before - 9);
        assert_eq!(game.galaxy_requirement(), galaxy_before - 9);
    }

    #[test]
    fn galaxy_boost_strengthens_tickspeed_reduction() {
        let mut game = GameState::new();
        maxed_ip(&mut game);
        game.galaxies = 2;
        let before = game.tickspeed_purchase_multiplier();

        game.buy_infinity_upgrade(InfinityUpgrade::Buy10Mult);
        game.buy_infinity_upgrade(InfinityUpgrade::Dim27Mult);
        game.buy_infinity_upgrade(InfinityUpgrade::Dim45Mult);
        game.buy_infinity_upgrade(InfinityUpgrade::GalaxyBoost);

        // Doubling the per-galaxy reduction lowers the retained multiplier.
        assert!(game.tickspeed_purchase_multiplier() < before);
    }

    #[test]
    fn skip_reset_galaxy_restores_boosts_and_galaxy_on_crunch() {
        let mut game = GameState::new();
        maxed_ip(&mut game);
        game.buy_infinity_upgrade(InfinityUpgrade::SkipReset1);
        game.buy_infinity_upgrade(InfinityUpgrade::SkipReset2);
        game.buy_infinity_upgrade(InfinityUpgrade::SkipReset3);
        game.buy_infinity_upgrade(InfinityUpgrade::SkipResetGalaxy);

        game.antimatter = crate::data::constants::BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        // Start the next infinity already boosted, with a Galaxy.
        assert_eq!(game.dim_boosts, 4);
        assert_eq!(game.galaxies, 1);
    }

    #[test]
    fn passive_ip_generation_accrues_and_respects_cutoff() {
        let mut game = GameState::new();
        maxed_ip(&mut game);
        // Buy the column 2 chain up to ipGen.
        game.buy_infinity_upgrade(InfinityUpgrade::ThisInfinityTimeMult);
        game.buy_infinity_upgrade(InfinityUpgrade::UnspentIpMult);
        game.buy_infinity_upgrade(InfinityUpgrade::DimboostMult);
        game.buy_infinity_upgrade(InfinityUpgrade::IpGen);

        // No best infinity yet → no generation.
        let before = game.infinity_points;
        game.generate_passive_ip(1_000_000.0);
        assert_eq!(game.infinity_points, before);

        // A 1000 ms best infinity → gen period 10_000 ms → 3 IP over 30_000 ms.
        game.records.best_infinity.time_ms = 1_000.0;
        let before = game.infinity_points;
        game.generate_passive_ip(30_000.0);
        assert_eq!(game.infinity_points, before + Decimal::from_float(3.0));
    }

    /// Unlock Achievement 41 (the bottom-row gate) directly.
    fn with_ach41(game: &mut GameState) {
        game.unlock_achievement(41);
    }

    #[test]
    fn ip_mult_gated_on_achievement_41() {
        let mut game = GameState::new();
        game.infinity_points = Decimal::new(1.0, 10);
        assert!(!game.can_buy_ip_mult());
        with_ach41(&mut game);
        assert!(game.can_buy_ip_mult());
    }

    #[test]
    fn ip_mult_cost_curve_and_single_purchase() {
        let mut game = GameState::new();
        with_ach41(&mut game);
        // Cost starts at 10 and ×10 per purchase.
        assert_eq!(game.ip_mult_cost(), Decimal::from_float(10.0));
        game.infinity_points = Decimal::from_float(10.0);
        assert!(game.buy_ip_mult());
        assert_eq!(game.ip_mult_purchases, 1);
        assert_eq!(game.infinity_points, Decimal::ZERO);
        assert_eq!(game.ip_mult_cost(), Decimal::from_float(100.0));
        assert_eq!(game.ip_mult_effect(), Decimal::from_float(2.0));
    }

    #[test]
    fn ip_mult_buy_max_spends_the_geometric_series() {
        let mut game = GameState::new();
        with_ach41(&mut game);
        // 10 + 100 + 1000 = 1110 buys exactly 3 with 1500 IP.
        game.infinity_points = Decimal::from_float(1500.0);
        game.buy_max_ip_mult();
        assert_eq!(game.ip_mult_purchases, 3);
        let left = game.infinity_points.to_f64();
        assert!((left - 390.0).abs() < 1e-6, "left={left}");
    }

    #[test]
    fn ip_mult_steepens_past_the_threshold_and_caps() {
        let mut game = GameState::new();
        with_ach41(&mut game);
        // Just below the threshold the ratio is still ×10.
        game.ip_mult_purchases = 2_999_998;
        assert_eq!(game.ip_mult_cost(), Decimal::new_unchecked(1.0, 2_999_999));
        // At `purchasesAtIncrease` the cost jumps to the 1e3M threshold and the
        // ratio steepens to ×1e10.
        game.ip_mult_purchases = 2_999_999;
        assert_eq!(game.ip_mult_cost(), Decimal::new_unchecked(1.0, 3_000_000));
        game.ip_mult_purchases = 3_000_000;
        assert_eq!(game.ip_mult_cost(), Decimal::new_unchecked(1.0, 3_000_010));
        // The cap: cost ≥ 1e6M ⇒ no further purchases even with infinite IP.
        game.ip_mult_purchases = 3_299_999;
        assert!(game.ip_mult_capped());
        game.infinity_points = Decimal::MAX_VALUE;
        assert!(!game.can_buy_ip_mult());
    }

    #[test]
    fn ip_mult_buy_max_crosses_the_threshold_in_one_call() {
        let mut game = GameState::new();
        with_ach41(&mut game);
        // Enough IP to clear the whole ×10 regime and buy into the ×1e10 one.
        game.infinity_points = Decimal::new_unchecked(1.0, 3_000_025);
        game.buy_max_ip_mult();
        assert!(
            game.ip_mult_purchases > 2_999_999,
            "purchases={}",
            game.ip_mult_purchases
        );
        assert!(game.ip_mult_purchases < 3_300_000);
    }

    #[test]
    fn ip_mult_purchase_bumps_crunch_autobuyer_amount() {
        let mut game = GameState::new();
        with_ach41(&mut game);
        // Unlock the Big Crunch autobuyer + dynamic amount.
        for id in 1..=12 {
            game.complete_challenge(id);
        }
        game.autobuyers.big_crunch_settings.increase_with_mult = true;
        game.autobuyers.big_crunch_settings.amount = Decimal::from_float(3.0);
        game.infinity_points = Decimal::from_float(10.0);
        assert!(game.buy_ip_mult());
        // ×2 per purchase (TS181 not owned).
        assert_eq!(
            game.autobuyers.big_crunch_settings.amount,
            Decimal::from_float(6.0)
        );
    }

    #[test]
    fn ip_offline_purchase_and_offline_award() {
        let mut game = GameState::new();
        game.infinity_points = Decimal::from_float(2000.0);
        assert!(!game.buy_ip_offline()); // needs Achievement 41
        with_ach41(&mut game);
        assert!(game.buy_ip_offline());
        assert!(game.ip_offline_bought);
        assert_eq!(game.infinity_points, Decimal::from_float(1000.0));
        assert!(!game.buy_ip_offline()); // one-time

        // The offline award: bestIPMsWithoutMaxAll × ms / 2.
        game.records.this_eternity.best_ip_ms_without_max_all = Decimal::from_float(4.0);
        let before = game.infinity_points;
        game.offline_currency_gain(1000.0);
        assert_eq!(game.infinity_points, before + Decimal::from_float(2000.0));
    }

    #[test]
    fn keep_infinity_upgrades_milestone_grants_ip_offline() {
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(4.0);
        game.player_infinity_upgrades_on_reset();
        assert!(game.ip_offline_bought);
        assert_eq!(game.infinity_upgrades.count_ones(), 16);

        // Below the milestone everything (including ipOffline) clears.
        game.eternities = Decimal::ZERO;
        game.player_infinity_upgrades_on_reset();
        assert!(!game.ip_offline_bought);
        assert_eq!(game.infinity_upgrades, 0);
    }

    #[test]
    fn best_run_ippm_drives_the_break_infinity_ipgen_passive() {
        let mut game = GameState::new();
        // One recent infinity: 1e12 IP over 120_000 ms (2 min) → 5e11 IP/min.
        game.records.recent_infinities[0] = crate::records::RecentInfinity {
            time_ms: 120_000.0,
            real_time_ms: 120_000.0,
            ip: Decimal::new(1.0, 12),
            infinities: Decimal::ONE,
        };
        assert_eq!(game.best_run_ippm(), Decimal::new(5.0, 11));

        // 4 ipGen upgrades → effect = bestRunIPPM · 4/20 = 1e11 IP/min. Over one
        // minute the passive add is exactly that. (The `ipGen` Infinity Upgrade is
        // not owned, so source 1 stays inert.)
        game.infinity_rebuyables[2] = 4;
        let before = game.infinity_points;
        game.generate_passive_ip(60_000.0);
        assert_eq!(game.infinity_points, before + Decimal::new(1.0, 11));

        // With no upgrades the passive term is inert.
        game.infinity_rebuyables[2] = 0;
        let before = game.infinity_points;
        game.generate_passive_ip(60_000.0);
        assert_eq!(game.infinity_points, before);
    }
}
