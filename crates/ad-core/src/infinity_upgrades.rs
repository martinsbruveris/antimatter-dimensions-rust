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
//! The bottom row (`ipMult` rebuyable + `ipOffline`, gated by Achievement 41) is a
//! scoped follow-up and is not modelled here.

use break_infinity::Decimal;

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
        // bitmasks, so sum their popcounts. Its reward (two extra upgrades) is
        // unmodelled, so the unlock has no numeric effect.
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
        // EC3's reward adds +0.72/completion to the base before the ×1.1.
        let mut mult =
            Decimal::from_float(BUY_TEN_MULTIPLIER + self.ec3_buy_ten_bonus());
        if self.infinity_upgrade_bought(InfinityUpgrade::Buy10Mult) {
            mult *= Decimal::from_float(1.1);
        }
        // Achievement 58 (NC9 in ≤ 3 min): +1% to the buy-10 multiplier.
        if self.achievement_unlocked(58) {
            mult *= Decimal::from_float(1.01);
        }
        // The `powerbuy10` glyph effect multiplies the base.
        mult *= Decimal::from_float(self.glyph_effect_powerbuy10());
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
        // Achievement 86: all Galaxies 1% stronger.
        let ach86 = if self.achievement_unlocked(86) {
            1.01
        } else {
            1.0
        };
        infinity_boost * self.break_infinity_galaxy_boost() * ic5 * ach86
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
        if !self.infinity_upgrade_bought(InfinityUpgrade::IpGen) {
            return;
        }
        let best = self.records.best_infinity.time_ms;
        if best >= IP_GEN_TOO_SLOW_MS {
            return;
        }
        let gen_period = best * 10.0;
        if gen_period <= 0.0 {
            return;
        }
        self.part_infinity_point += dt_ms / gen_period;
        let whole = self.part_infinity_point.floor();
        if whole >= 1.0 {
            self.part_infinity_point -= whole;
            self.infinity_points += Decimal::from_float(whole) * self.total_ip_mult();
        }
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
}
