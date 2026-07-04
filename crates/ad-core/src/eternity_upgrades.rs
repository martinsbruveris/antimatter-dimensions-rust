//! Eternity Upgrades (Feature 4.6): six one-time EP upgrades plus the
//! rebuyable ×5 EP multiplier.
//!
//! Mirrors `secret-formula/eternity/eternity-upgrades.js` and the
//! `EPMultiplierState` in `src/core/eternity.js`. The one-time upgrades have
//! no prerequisites (a plain 2×3 grid); their effects multiply the Infinity /
//! Time Dimensions at the usual common-multiplier sites. `epMult`'s effect
//! (`5^purchases`) feeds `totalEPMult`; its cost walks `500 × stepMult^count`
//! with the step multiplier jumping at 1e100 / 1.8e308 / 1e1300 EP and a
//! super-exponential branch past 1e4000. See
//! `design-docs/2026-07-04-eternity.md` §6.

use break_infinity::Decimal;

use crate::state::GameState;

/// The six one-time Eternity Upgrades, bit `1 << (id - 1)` in
/// `GameState::eternity_upgrades`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EternityUpgrade {
    /// ID multiplier `unspent EP + 1`.
    IdMultEp = 1,
    /// ID multiplier from Eternities (softcap at 1e5).
    IdMultEternities = 2,
    /// ID multiplier from the summed Infinity Challenge record times.
    IdMultIcRecords = 3,
    /// The Achievement bonus affects Time Dimensions.
    TdMultAchievements = 4,
    /// TDs multiplied by unspent Time Theorems.
    TdMultTheorems = 5,
    /// TDs multiplied by days played.
    TdMultRealTime = 6,
}

pub const ALL_ETERNITY_UPGRADES: [EternityUpgrade; 6] = [
    EternityUpgrade::IdMultEp,
    EternityUpgrade::IdMultEternities,
    EternityUpgrade::IdMultIcRecords,
    EternityUpgrade::TdMultAchievements,
    EternityUpgrade::TdMultTheorems,
    EternityUpgrade::TdMultRealTime,
];

impl EternityUpgrade {
    /// Original numeric id (the `player.eternityUpgrades` Set entries).
    pub fn id(self) -> u8 {
        self as u8
    }

    /// Bit in the `eternity_upgrades` bitmask.
    pub fn bit(self) -> u32 {
        1 << (self as u32 - 1)
    }

    /// From the original id.
    pub fn from_id(id: u8) -> Option<Self> {
        ALL_ETERNITY_UPGRADES
            .get(id.checked_sub(1)? as usize)
            .copied()
    }

    /// EP cost.
    pub fn cost(self) -> Decimal {
        match self {
            EternityUpgrade::IdMultEp => Decimal::from_float(5.0),
            EternityUpgrade::IdMultEternities => Decimal::from_float(10.0),
            EternityUpgrade::IdMultIcRecords => Decimal::from_float(5e4),
            EternityUpgrade::TdMultAchievements => Decimal::new_unchecked(1.0, 16),
            EternityUpgrade::TdMultTheorems => Decimal::new_unchecked(1.0, 40),
            EternityUpgrade::TdMultRealTime => Decimal::new_unchecked(1.0, 50),
        }
    }
}

/// `epMult`'s cost thresholds and the per-purchase step multiplier below each
/// (`EPMultiplierState.costIncreaseThresholds` / `multPerUpgrade`).
const EP_MULT_THRESHOLDS: [Decimal; 4] = [
    Decimal::new_unchecked(1.0, 100),
    Decimal::NUMBER_MAX_VALUE,
    Decimal::new_unchecked(1.0, 1300),
    Decimal::new_unchecked(1.0, 4000),
];
const EP_MULT_STEPS: [f64; 4] = [50.0, 100.0, 500.0, 1000.0];

/// `epMult` cost at `count` purchases (`costAfterCount`).
pub fn ep_mult_cost_at(count: u32) -> Decimal {
    for (i, threshold) in EP_MULT_THRESHOLDS.iter().enumerate() {
        let cost = Decimal::from_float(EP_MULT_STEPS[i])
            .pow(&Decimal::from(count as u64))
            * Decimal::from_float(500.0);
        if cost < *threshold {
            return cost;
        }
    }
    // Past 1e4000: 500 × 1e3^(count + max(count-1334, 0)^1.2).
    let count = count as f64;
    let exponent = count + (count - 1334.0).max(0.0).powf(1.2);
    Decimal::pow10(3.0 * exponent) * Decimal::from_float(500.0)
}

impl GameState {
    /// Whether an Eternity Upgrade is owned.
    pub fn eternity_upgrade_bought(&self, upgrade: EternityUpgrade) -> bool {
        self.eternity_upgrades & upgrade.bit() != 0
    }

    /// Whether it can be bought now (not owned, affordable).
    pub fn can_buy_eternity_upgrade(&self, upgrade: EternityUpgrade) -> bool {
        !self.eternity_upgrade_bought(upgrade) && self.eternity_points >= upgrade.cost()
    }

    /// Buy an Eternity Upgrade. Returns whether it happened.
    pub fn buy_eternity_upgrade(&mut self, upgrade: EternityUpgrade) -> bool {
        if !self.can_buy_eternity_upgrade(upgrade) {
            return false;
        }
        self.eternity_points -= upgrade.cost();
        self.eternity_upgrades |= upgrade.bit();
        true
    }

    /// The next `epMult` purchase's cost.
    pub fn ep_mult_cost(&self) -> Decimal {
        ep_mult_cost_at(self.epmult_upgrades)
    }

    /// `epMult`'s effect: ×5 per purchase.
    pub fn ep_mult_effect(&self) -> Decimal {
        Decimal::from_float(5.0).pow(&Decimal::from(self.epmult_upgrades as u64))
    }

    /// Buy one `epMult`. Returns whether it happened.
    pub fn buy_ep_mult(&mut self) -> bool {
        let cost = self.ep_mult_cost();
        if self.eternity_points < cost {
            return false;
        }
        self.eternity_points -= cost;
        self.epmult_upgrades += 1;
        true
    }

    /// Buy as many `epMult` as affordable (`buyMax`, via repeated singles —
    /// the count grows logarithmically in EP). Returns the number bought.
    pub fn buy_max_ep_mult(&mut self) -> u64 {
        let mut count = 0;
        while self.buy_ep_mult() {
            count += 1;
        }
        count
    }

    /// Public effect readers for display (the GUI's upgrade tiles).
    pub fn eu2_effect_public(&self) -> Decimal {
        self.eu2_effect()
    }
    pub fn eu3_effect_public(&self) -> Decimal {
        self.eu3_effect()
    }

    /// The Infinity-Dimension multiplier from Eternity Upgrades 1–3
    /// (`idMultEP` / `idMultEternities` / `idMultICRecords`), folded into
    /// `id_common_multiplier`.
    pub(crate) fn eternity_upgrade_id_mult(&self) -> Decimal {
        let mut mult = Decimal::ONE;
        if self.eternity_upgrade_bought(EternityUpgrade::IdMultEp) {
            mult *= self.eternity_points + Decimal::ONE;
        }
        if self.eternity_upgrade_bought(EternityUpgrade::IdMultEternities) {
            mult *= self.eu2_effect();
        }
        if self.eternity_upgrade_bought(EternityUpgrade::IdMultIcRecords) {
            mult *= self.eu3_effect();
        }
        mult
    }

    /// EU2's effect: `(eternities/200 + 1)^log4(2·eternities + 1)`, softcapped
    /// at 1e5 Eternities (the post-cap factor grows much slower).
    pub(crate) fn eu2_effect(&self) -> Decimal {
        let log4 = 4f64.ln();
        let eternities_pre_cap = self.eternities.min(&Decimal::from_float(1e5)).to_f64();
        let base = eternities_pre_cap / 200.0 + 1.0;
        let pow = (eternities_pre_cap * 2.0 + 1.0).ln() / log4;
        let mult_pre_cap = Decimal::from_float(base).pow(&Decimal::from_float(pow));

        let post_cap = (self.eternities - Decimal::from_float(1e5)).max(&Decimal::ZERO);
        let mult1 = post_cap / Decimal::from_float(200.0) + Decimal::ONE;
        let mult2 = (post_cap * Decimal::from_float(2.0) + Decimal::ONE).ln() / log4;
        let mult_post_cap = (mult1 * Decimal::from_float(mult2)).max(&Decimal::ONE);
        mult_post_cap * mult_pre_cap
    }

    /// EU3's effect: `2^(30 / max(sum of IC record times in s, 0.1))`, capped
    /// at `2^(30/0.61)`.
    pub(crate) fn eu3_effect(&self) -> Decimal {
        let sum_seconds: f64 = self
            .ic_best_times_ms
            .iter()
            .map(|&t| if t == f64::MAX { 0.0 } else { t / 1000.0 })
            .sum();
        // An unset record (never completed) contributes Number.MAX_VALUE in the
        // original, zeroing the effect; mirror that by treating any unset entry
        // as an enormous sum.
        if self.ic_best_times_ms.contains(&f64::MAX) {
            return Decimal::ONE;
        }
        let exponent = 30.0 / sum_seconds.max(0.1);
        Decimal::from_float(2.0)
            .pow(&Decimal::from_float(exponent))
            .min(&Decimal::from_float(2.0).pow(&Decimal::from_float(30.0 / 0.61)))
    }

    /// The Time-Dimension multiplier from Eternity Upgrades 4–6, folded into
    /// `td_common_multiplier`.
    pub(crate) fn eternity_upgrade_td_mult(&self) -> Decimal {
        let mut mult = Decimal::ONE;
        if self.eternity_upgrade_bought(EternityUpgrade::TdMultAchievements) {
            mult *= self.achievement_power();
        }
        if self.eternity_upgrade_bought(EternityUpgrade::TdMultTheorems) {
            mult *= self.time_theorems.max(&Decimal::ONE);
        }
        if self.eternity_upgrade_bought(EternityUpgrade::TdMultRealTime) {
            let days = self.records.total_time_played_ms / 86_400_000.0;
            mult *= Decimal::from_float(days.max(1.0));
        }
        mult
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrades_cost_ep_and_apply_to_ids_and_tds() {
        let mut game = GameState::new();
        game.eternity_points = Decimal::from_float(20.0);
        assert!(game.buy_eternity_upgrade(EternityUpgrade::IdMultEp));
        assert_eq!(game.eternity_points, Decimal::from_float(15.0));
        assert!(!game.buy_eternity_upgrade(EternityUpgrade::IdMultEp)); // owned
                                                                        // EU1: ID mult = unspent EP + 1.
        assert_eq!(game.eternity_upgrade_id_mult(), Decimal::from_float(16.0));

        // EU5: TDs × unspent TT.
        game.eternity_points = Decimal::new_unchecked(1.0, 41);
        assert!(game.buy_eternity_upgrade(EternityUpgrade::TdMultTheorems));
        game.time_theorems = Decimal::from_float(123.0);
        assert_eq!(game.eternity_upgrade_td_mult(), Decimal::from_float(123.0));
    }

    #[test]
    fn eu2_softcaps_at_1e5_eternities() {
        let mut game = GameState::new();
        game.eternity_upgrades = EternityUpgrade::IdMultEternities.bit();
        game.eternities = Decimal::from_float(200.0);
        let low = game.eu2_effect();
        assert!(low > Decimal::ONE);
        game.eternities = Decimal::from_float(1e5);
        let at_cap = game.eu2_effect();
        game.eternities = Decimal::from_float(2e5);
        let past_cap = game.eu2_effect();
        assert!(past_cap > at_cap);
        // Past the cap growth is far below the pre-cap curve's continuation.
        let ratio = (past_cap / at_cap).log10();
        assert!(ratio < at_cap.log10());
    }

    #[test]
    fn eu3_needs_all_ic_records() {
        let mut game = GameState::new();
        game.eternity_upgrades = EternityUpgrade::IdMultIcRecords.bit();
        // Unset records zero the effect.
        assert_eq!(game.eu3_effect(), Decimal::ONE);
        game.ic_best_times_ms = [3000.0; 8]; // 24 s total
        let effect = game.eu3_effect();
        // 2^(30/24) = 2^1.25 ≈ 2.378.
        assert!((effect.to_f64() - 2f64.powf(1.25)).abs() < 1e-9);
    }

    #[test]
    fn ep_mult_cost_walks_the_thresholds() {
        // Below 1e100: 500 × 50^n.
        assert_eq!(ep_mult_cost_at(0), Decimal::from_float(500.0));
        assert_eq!(ep_mult_cost_at(1), Decimal::from_float(25_000.0));
        // 500 × 50^58 ≈ 3.5e100 crosses the first threshold → ×100 step.
        let jumped = ep_mult_cost_at(58);
        assert_eq!(
            jumped,
            Decimal::from_float(100.0).pow(&Decimal::from(58u64))
                * Decimal::from_float(500.0)
        );
    }

    #[test]
    fn ep_mult_multiplies_ep_gain() {
        let mut game = GameState::new();
        game.eternity_points = Decimal::from_float(600.0);
        assert!(game.buy_ep_mult());
        assert_eq!(game.epmult_upgrades, 1);
        assert_eq!(game.ep_mult_effect(), Decimal::from_float(5.0));
        assert!(game.eternity_points < Decimal::from_float(600.0));

        // The EP formula picks it up via totalEPMult: at the goal the raw base
        // is ≈1.62, so ×5 floors to 8 (vs 1 without the upgrade).
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        assert_eq!(game.gained_eternity_points(), Decimal::from_float(8.0));
        game.epmult_upgrades = 0;
        assert_eq!(game.gained_eternity_points(), Decimal::ONE);
    }

    #[test]
    fn ic_completion_records_best_time() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.broke_infinity = true;
        game.records.this_eternity.max_am = Decimal::new(1.0, 3000);
        assert!(game.start_infinity_challenge(1));
        game.tick(5_000.0);
        game.antimatter = GameState::infinity_challenge_goal(1);
        game.records.this_infinity.max_am = game.antimatter;
        assert!(game.big_crunch());
        assert!(game.infinity_challenge_completed(1));
        assert!(game.ic_best_times_ms[0] >= 5_000.0);
        assert!(game.ic_best_times_ms[0] < f64::MAX);
    }
}
