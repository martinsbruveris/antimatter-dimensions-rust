//! Break Infinity Upgrades: 9 one-time + 3 rebuyable upgrades bought with Infinity
//! Points, available once Infinity is broken (Feature 2.3).
//!
//! The one-time upgrades share the original's `player.infinityUpgrades` string set
//! with the §2.2 [`InfinityUpgrade`](crate::InfinityUpgrade)s, but are a distinct
//! `u32` bitmask on [`GameState`] (`break_infinity_upgrades`). The rebuyables are
//! purchase counts in `player.infinityRebuyables` (`infinity_rebuyables: [u32; 3]`).
//! Both persist across a Big Crunch.
//!
//! This module owns the upgrade vocabulary + data, the purchase logic, and the
//! effect readers the rest of the engine calls (`break_infinity_upgrade_common_mult`,
//! `break_infinity_galaxy_boost`, `break_infinity_autobuyer_speedup`). A few effects
//! still depend on inputs we don't model yet (IP/min, the cost-scaling knobs); those
//! upgrades are purchasable/persisted but their effect is neutral until their inputs
//! exist. See `docs/design/2026-07-03-break-infinity.md`.

use break_infinity::Decimal;

use crate::state::GameState;

/// The 9 one-time Break Infinity Upgrades. The `as usize` discriminant is the
/// bitmask bit index; save round-trip uses [`BreakInfinityUpgrade::save_id`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BreakInfinityUpgrade {
    /// AD ×`(totalAM.exp + 1)^0.5`.
    TotalAmMult,
    /// AD ×`(AM.exp + 1)^0.5`.
    CurrentAmMult,
    /// Galaxies 50% stronger.
    GalaxyBoost,
    /// AD ×`1 + infinities.log10()·10`.
    InfinitiedMult,
    /// AD ×`max((achievements − 30)^3 / 40, 1)`.
    AchievementMult,
    /// AD ×`clampMin(50 / worstChallengeMinutes, 1)` (capped at 3e4).
    SlowestChallengeMult,
    /// Passively generate Infinities (deferred — generation loop).
    InfinitiedGen,
    /// Unlock the buy-max Dimension Boost autobuyer mode (deferred behaviour).
    AutobuyMaxDimboosts,
    /// Challenge-unlocked autobuyers work twice as fast.
    AutobuyerSpeed,
}

/// Number of one-time Break Infinity Upgrades.
pub const BREAK_INFINITY_UPGRADE_COUNT: usize = 9;

/// All 9 one-time upgrades in bit-index order.
pub const ALL_BREAK_INFINITY_UPGRADES: [BreakInfinityUpgrade;
    BREAK_INFINITY_UPGRADE_COUNT] = [
    BreakInfinityUpgrade::TotalAmMult,
    BreakInfinityUpgrade::CurrentAmMult,
    BreakInfinityUpgrade::GalaxyBoost,
    BreakInfinityUpgrade::InfinitiedMult,
    BreakInfinityUpgrade::AchievementMult,
    BreakInfinityUpgrade::SlowestChallengeMult,
    BreakInfinityUpgrade::InfinitiedGen,
    BreakInfinityUpgrade::AutobuyMaxDimboosts,
    BreakInfinityUpgrade::AutobuyerSpeed,
];

/// IP cost of each one-time upgrade, by bit index. From
/// `secret-formula/infinity/break-infinity-upgrades.js`.
const BREAK_UPGRADE_COSTS: [Decimal; BREAK_INFINITY_UPGRADE_COUNT] = [
    Decimal::new_unchecked(1.0, 4),  // totalMult
    Decimal::new_unchecked(5.0, 4),  // currentMult
    Decimal::new_unchecked(5.0, 11), // postGalaxy
    Decimal::new_unchecked(1.0, 5),  // infinitiedMult
    Decimal::new_unchecked(1.0, 6),  // achievementMult
    Decimal::new_unchecked(1.0, 7),  // challengeMult
    Decimal::new_unchecked(2.0, 7),  // infinitiedGeneration
    Decimal::new_unchecked(5.0, 9),  // autobuyMaxDimboosts
    Decimal::new_unchecked(1.0, 15), // autoBuyerUpgrade
];

/// Original save id of each one-time upgrade (stored in `player.infinityUpgrades`).
const BREAK_UPGRADE_SAVE_IDS: [&str; BREAK_INFINITY_UPGRADE_COUNT] = [
    "totalMult",
    "currentMult",
    "postGalaxy",
    "infinitiedMult",
    "achievementMult",
    "challengeMult",
    "infinitiedGeneration",
    "autobuyMaxDimboosts",
    "autoBuyerUpgrade",
];

impl BreakInfinityUpgrade {
    /// The bitmask bit for this upgrade.
    pub fn bit(self) -> u32 {
        1u32 << (self as u32)
    }

    /// The IP cost.
    pub fn cost(self) -> Decimal {
        BREAK_UPGRADE_COSTS[self as usize]
    }

    /// The original save id.
    pub fn save_id(self) -> &'static str {
        BREAK_UPGRADE_SAVE_IDS[self as usize]
    }

    /// The upgrade with the given save id, if any.
    pub fn from_save_id(id: &str) -> Option<BreakInfinityUpgrade> {
        ALL_BREAK_INFINITY_UPGRADES
            .into_iter()
            .find(|u| u.save_id() == id)
    }
}

/// The 3 rebuyable Break Infinity Upgrades (indices into `infinity_rebuyables`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakInfinityRebuyable {
    /// Reduce post-infinity Tickspeed cost scaling (deferred effect).
    TickspeedCostMult,
    /// Reduce post-infinity Antimatter Dimension cost scaling (deferred effect).
    DimCostMult,
    /// Passive IP generation from best IP/min (deferred effect).
    IpGen,
}

/// All 3 rebuyables in index order.
pub const ALL_BREAK_INFINITY_REBUYABLES: [BreakInfinityRebuyable; 3] = [
    BreakInfinityRebuyable::TickspeedCostMult,
    BreakInfinityRebuyable::DimCostMult,
    BreakInfinityRebuyable::IpGen,
];

impl BreakInfinityRebuyable {
    /// Index into `infinity_rebuyables`.
    pub fn index(self) -> usize {
        self as usize
    }

    /// `(initial_cost, cost_increase, max_upgrades)`.
    fn config(self) -> (f64, f64, u32) {
        match self {
            BreakInfinityRebuyable::TickspeedCostMult => (1e6, 5.0, 8),
            BreakInfinityRebuyable::DimCostMult => (1e7, 5e3, 7),
            BreakInfinityRebuyable::IpGen => (1e7, 10.0, 10),
        }
    }

    /// Maximum number of purchases.
    pub fn max_upgrades(self) -> u32 {
        self.config().2
    }
}

impl GameState {
    // --- One-time upgrades ---------------------------------------------------

    /// Whether one-time Break Infinity Upgrade `upgrade` is owned.
    pub fn break_infinity_upgrade_bought(&self, upgrade: BreakInfinityUpgrade) -> bool {
        self.break_infinity_upgrades & upgrade.bit() != 0
    }

    /// Whether `upgrade` can be bought now: Infinity is broken, it isn't already
    /// owned, and there are enough Infinity Points.
    pub fn can_buy_break_infinity_upgrade(&self, upgrade: BreakInfinityUpgrade) -> bool {
        self.broke_infinity
            && !self.break_infinity_upgrade_bought(upgrade)
            && self.infinity_points >= upgrade.cost()
    }

    /// Buy `upgrade`, spending its IP cost. Returns whether the purchase happened.
    pub fn buy_break_infinity_upgrade(&mut self, upgrade: BreakInfinityUpgrade) -> bool {
        if !self.can_buy_break_infinity_upgrade(upgrade) {
            return false;
        }
        self.infinity_points -= upgrade.cost();
        self.break_infinity_upgrades |= upgrade.bit();
        true
    }

    // --- Rebuyables ----------------------------------------------------------

    /// Current purchase count of rebuyable `upgrade`.
    pub fn break_infinity_rebuyable_count(
        &self,
        upgrade: BreakInfinityRebuyable,
    ) -> u32 {
        self.infinity_rebuyables[upgrade.index()]
    }

    /// The IP cost of the next purchase of rebuyable `upgrade`
    /// (`initial × increase^count`).
    pub fn break_infinity_rebuyable_cost(
        &self,
        upgrade: BreakInfinityRebuyable,
    ) -> Decimal {
        let (initial, increase, _) = upgrade.config();
        let count = self.break_infinity_rebuyable_count(upgrade);
        Decimal::from_float(initial)
            * Decimal::from_float(increase).pow(&Decimal::from(count as u64))
    }

    /// Whether rebuyable `upgrade` can be bought now: broken, below its cap, and
    /// affordable.
    pub fn can_buy_break_infinity_rebuyable(
        &self,
        upgrade: BreakInfinityRebuyable,
    ) -> bool {
        self.broke_infinity
            && self.break_infinity_rebuyable_count(upgrade) < upgrade.max_upgrades()
            && self.infinity_points >= self.break_infinity_rebuyable_cost(upgrade)
    }

    /// Buy one level of rebuyable `upgrade`. Returns whether it happened.
    pub fn buy_break_infinity_rebuyable(
        &mut self,
        upgrade: BreakInfinityRebuyable,
    ) -> bool {
        if !self.can_buy_break_infinity_rebuyable(upgrade) {
            return false;
        }
        let cost = self.break_infinity_rebuyable_cost(upgrade);
        self.infinity_points -= cost;
        self.infinity_rebuyables[upgrade.index()] += 1;
        true
    }

    // --- Effect readers ------------------------------------------------------

    /// Number of unlocked achievements (the original's `effectiveCount`, minus the
    /// Reality-era secret-achievement bookkeeping we don't model): a straight
    /// popcount of the achievement bits.
    pub fn achievement_count(&self) -> u32 {
        self.achievement_bits.iter().map(|r| r.count_ones()).sum()
    }

    /// The all-tier AD multiplier from the owned Break Infinity Upgrades
    /// (`totalAMMult`, `currentAMMult`, `infinitiedMult`, `achievementMult`). The
    /// other AD-affecting upgrades are deferred and contribute ×1. Applied in
    /// `dimension_multiplier` alongside the Infinity-Upgrade common multiplier.
    pub fn break_infinity_upgrade_common_mult(&self) -> Decimal {
        let mut mult = Decimal::ONE;
        if self.break_infinity_upgrade_bought(BreakInfinityUpgrade::TotalAmMult) {
            let e = self.total_antimatter.exponent() as f64 + 1.0;
            mult *= Decimal::from_float(e.max(0.0).powf(0.5));
        }
        if self.break_infinity_upgrade_bought(BreakInfinityUpgrade::CurrentAmMult) {
            let e = self.antimatter.exponent() as f64 + 1.0;
            mult *= Decimal::from_float(e.max(0.0).powf(0.5));
        }
        if self.break_infinity_upgrade_bought(BreakInfinityUpgrade::InfinitiedMult) {
            // Reads infinitiesTotal (banked included); TS31 raises the whole
            // infinity-count bonus to the 4th power.
            let base =
                Decimal::from_float(1.0 + self.infinities_total().pos_log10() * 10.0);
            mult *= if self.time_study_bought(31) {
                base.pow(&Decimal::from_float(4.0))
            } else {
                base
            };
        }
        if self.break_infinity_upgrade_bought(BreakInfinityUpgrade::AchievementMult) {
            let count = self.achievement_count() as f64;
            let value = ((count - 30.0).powi(3) / 40.0).max(1.0);
            mult *= Decimal::from_float(value);
        }
        mult *= self.slowest_challenge_mult();
        mult
    }

    /// The `slowestChallengeMult` Break Infinity Upgrade effect: an all-tier AD
    /// multiplier `clampMin(50 / worstChallengeMinutes, 1)`, capped at 3e4, where
    /// the worst challenge time is the slowest (max) of the Normal Challenge best
    /// times. Any uncompleted challenge (`f64::MAX`) leaves the worst time huge, so
    /// the effect stays at ×1 until every Normal Challenge is completed.
    fn slowest_challenge_mult(&self) -> Decimal {
        if !self
            .break_infinity_upgrade_bought(BreakInfinityUpgrade::SlowestChallengeMult)
        {
            return Decimal::ONE;
        }
        let worst_ms = self
            .nc_best_times_ms
            .iter()
            .copied()
            .fold(f64::MIN, f64::max);
        let worst_minutes = worst_ms / 60_000.0;
        let raw = (50.0 / worst_minutes).max(1.0);
        Decimal::from_float(raw).min(&Decimal::new_unchecked(3.0, 4))
    }

    /// Extra galaxy-strength factor from the `postGalaxy` Break Infinity Upgrade
    /// (×1.5), else ×1. Folds into the tickspeed formula's per-galaxy effects
    /// product (see `galaxy_strength_effect`).
    pub fn break_infinity_galaxy_boost(&self) -> f64 {
        if self.break_infinity_upgrade_bought(BreakInfinityUpgrade::GalaxyBoost) {
            1.5
        } else {
            1.0
        }
    }

    /// Interval multiplier from the `autoBuyerUpgrade` Break Infinity Upgrade
    /// (×0.5 = twice as fast), else ×1. Applied to challenge-unlocked autobuyer
    /// intervals.
    pub fn break_infinity_autobuyer_speedup(&self) -> f64 {
        if self.break_infinity_upgrade_bought(BreakInfinityUpgrade::AutobuyerSpeed) {
            0.5
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn broken_game() -> GameState {
        let mut game = GameState::new();
        game.broke_infinity = true;
        game
    }

    #[test]
    fn upgrades_require_break_infinity() {
        let mut game = GameState::new();
        game.infinity_points = Decimal::new(1.0, 20);
        // Not broken → cannot buy, even with plenty of IP.
        assert!(!game.can_buy_break_infinity_upgrade(BreakInfinityUpgrade::TotalAmMult));
        assert!(!game.buy_break_infinity_upgrade(BreakInfinityUpgrade::TotalAmMult));
        assert!(!game.can_buy_break_infinity_rebuyable(BreakInfinityRebuyable::IpGen));

        game.broke_infinity = true;
        assert!(game.can_buy_break_infinity_upgrade(BreakInfinityUpgrade::TotalAmMult));
    }

    #[test]
    fn buy_one_time_upgrade_spends_ip_once() {
        let mut game = broken_game();
        game.infinity_points = Decimal::new(1.0, 5); // 1e5
        let cost = BreakInfinityUpgrade::TotalAmMult.cost(); // 1e4

        assert!(game.buy_break_infinity_upgrade(BreakInfinityUpgrade::TotalAmMult));
        assert!(game.break_infinity_upgrade_bought(BreakInfinityUpgrade::TotalAmMult));
        assert_eq!(game.infinity_points, Decimal::new(1.0, 5) - cost);
        // One-time: a second purchase is a no-op.
        assert!(!game.buy_break_infinity_upgrade(BreakInfinityUpgrade::TotalAmMult));
    }

    #[test]
    fn rebuyable_cost_scales_and_caps() {
        let mut game = broken_game();
        game.infinity_points = Decimal::new(1.0, 30);
        let r = BreakInfinityRebuyable::TickspeedCostMult;

        // Initial cost 1e6, then ×5 per purchase.
        assert_eq!(game.break_infinity_rebuyable_cost(r), Decimal::new(1.0, 6));
        assert!(game.buy_break_infinity_rebuyable(r));
        assert_eq!(game.break_infinity_rebuyable_count(r), 1);
        assert_eq!(game.break_infinity_rebuyable_cost(r), Decimal::new(5.0, 6));

        // Buy up to the cap (8), then no more.
        game.infinity_points = Decimal::new(1.0, 30);
        while game.buy_break_infinity_rebuyable(r) {}
        assert_eq!(game.break_infinity_rebuyable_count(r), r.max_upgrades());
        assert!(!game.can_buy_break_infinity_rebuyable(r));
    }

    #[test]
    fn galaxy_boost_and_autobuyer_speedup_effects() {
        let mut game = broken_game();
        assert_eq!(game.break_infinity_galaxy_boost(), 1.0);
        assert_eq!(game.break_infinity_autobuyer_speedup(), 1.0);

        game.break_infinity_upgrades |= BreakInfinityUpgrade::GalaxyBoost.bit();
        game.break_infinity_upgrades |= BreakInfinityUpgrade::AutobuyerSpeed.bit();
        assert_eq!(game.break_infinity_galaxy_boost(), 1.5);
        assert_eq!(game.break_infinity_autobuyer_speedup(), 0.5);
    }

    #[test]
    fn slowest_challenge_mult_needs_all_challenges_and_the_upgrade() {
        let mut game = broken_game();
        // Every Normal Challenge completed, slowest at 2.05 min (123_000 ms).
        game.nc_best_times_ms = [10_000.0; 11];
        game.nc_best_times_ms[3] = 123_000.0; // the slowest run
                                              // Without the upgrade the effect is inert (×1).
        assert_eq!(game.break_infinity_upgrade_common_mult(), Decimal::ONE);

        game.break_infinity_upgrades |= BreakInfinityUpgrade::SlowestChallengeMult.bit();
        // effect = clampMin(50 / (123_000 / 60_000), 1) = 50 / 2.05 ≈ 24.390.
        let mult = game.break_infinity_upgrade_common_mult().to_f64();
        assert!((mult / (50.0 / 2.05) - 1.0).abs() < 1e-9, "{mult}");

        // An uncompleted challenge (f64::MAX) drops the effect back to ×1.
        game.nc_best_times_ms[7] = f64::MAX;
        assert_eq!(game.break_infinity_upgrade_common_mult(), Decimal::ONE);
    }

    #[test]
    fn common_mult_reflects_owned_upgrades() {
        let mut game = broken_game();
        assert_eq!(game.break_infinity_upgrade_common_mult(), Decimal::ONE);

        // totalAMMult on 1e134 total antimatter → (134+1)^0.5.
        game.total_antimatter = Decimal::new(1.0, 134);
        game.break_infinity_upgrades |= BreakInfinityUpgrade::TotalAmMult.bit();
        // achievementMult on 40 achievements → (40-30)^3/40 = 25.
        for row in 0..5 {
            game.achievement_bits[row] = 0xFF; // 8 achievements per row × 5 = 40
        }
        assert_eq!(game.achievement_count(), 40);
        game.break_infinity_upgrades |= BreakInfinityUpgrade::AchievementMult.bit();

        let expected = 135.0_f64.powf(0.5) * 25.0;
        let mult = game.break_infinity_upgrade_common_mult().to_f64();
        assert!((mult / expected - 1.0).abs() < 1e-9, "{mult} vs {expected}");
    }
}
