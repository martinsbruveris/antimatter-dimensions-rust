use break_infinity::Decimal;

use crate::data::constants::{AD_BASE_COSTS, AD_COST_MULTIPLIERS};
use crate::state::GameState;

impl GameState {
    /// Compute the current cost for the next purchase of a
    /// dimension tier (0-indexed). Cost increases every 10
    /// purchases: base_cost * cost_multiplier^(bought / 10).
    pub fn dimension_cost(&self, tier: usize) -> Decimal {
        let purchase_group = self.dimensions[tier].bought / 10;
        Decimal::from_float(AD_BASE_COSTS[tier])
            * Decimal::from_float(AD_COST_MULTIPLIERS[tier])
                .pow(&Decimal::from_float(purchase_group as f64))
    }

    /// Try to buy one of the specified dimension tier
    /// (0-indexed). Returns true if the purchase was
    /// successful.
    pub fn buy_dimension(&mut self, tier: usize) -> bool {
        if !self.dim_available_for_purchase(tier) {
            return false;
        }

        let cost = self.dimension_cost(tier);
        if self.antimatter >= cost {
            self.antimatter -= cost;
            self.dimensions[tier].amount += Decimal::from_float(1.0);
            self.dimensions[tier].bought += 1;
            self.on_buy_dimension(tier);
            true
        } else {
            false
        }
    }

    /// Achievement checks fired after buying one of `tier` (0-indexed). Mirrors
    /// the original's `onBuyDimension` unlocks: "buy an Nth dimension" (11–18),
    /// the exactly-99 eighth-dimension achievement (23), and the over-1e150
    /// first-dimension achievement (28).
    fn on_buy_dimension(&mut self, tier: usize) {
        // 11–18: buy a 1st..8th Antimatter Dimension (tier is 0-indexed).
        self.unlock_achievement(11 + tier as u16);
        // 23: have exactly 99 eighth dimensions (only buying AD8 can reach it).
        if tier == 7 && self.dimensions[7].amount == Decimal::from_float(99.0) {
            self.unlock_achievement(23);
        }
        // 28: buy a 1st dimension while holding over 1e150 of them.
        if tier == 0 && self.dimensions[0].amount.exponent() >= 150 {
            self.unlock_achievement(28);
        }
    }

    /// Buy the maximum number of the specified dimension tier
    /// that can be afforded. Returns the number of dimensions
    /// bought.
    pub fn buy_max_dimension(&mut self, tier: usize) -> u64 {
        let mut count = 0u64;
        while self.buy_dimension(tier) {
            count += 1;
        }
        count
    }

    /// Buy dimensions until the next group of 10 is complete.
    /// Returns the number bought.
    pub fn buy_until_10_dimension(&mut self, tier: usize) -> u64 {
        if !self.dim_available_for_purchase(tier) {
            return 0;
        }
        let remaining = 10 - (self.dimensions[tier].bought % 10);
        let mut count = 0u64;
        for _ in 0..remaining {
            if !self.buy_dimension(tier) {
                break;
            }
            count += 1;
        }
        count
    }

    /// Compute the cost to buy until the next group of 10
    /// for a dimension tier. Since cost only changes every
    /// 10 purchases, all remaining buys in this group cost
    /// the same.
    pub fn dimension_cost_until_10(&self, tier: usize) -> Decimal {
        let remaining = 10 - (self.dimensions[tier].bought % 10);
        self.dimension_cost(tier) * Decimal::from_float(remaining as f64)
    }

    /// Get the antimatter production per second (from AD1).
    pub fn antimatter_per_second(&self) -> Decimal {
        self.dimension_production_per_second(0)
    }

    /// Buy max of all unlocked dimensions and tickspeed.
    pub fn max_all(&mut self) {
        self.buy_max_tickspeed();
        let unlocked = self.unlocked_dimensions();
        for tier in 0..unlocked {
            self.buy_max_dimension(tier);
        }
    }

    /// Compute the production multiplier for a given dimension
    /// tier (0-indexed). Includes:
    /// - Buy-10 multiplier (2x per 10 purchases)
    /// - Tier-dependent dim boost multiplier
    /// - Sacrifice multiplier (only for tier 8 / index 7)
    pub fn dimension_multiplier(&self, tier: usize) -> Decimal {
        let mut mult = Decimal::from_float(1.0);

        // Buy-10 multiplier: base^(bought / 10). Base is 2, or 2.2 with the
        // `buy10Mult` Infinity Upgrade (`buy_ten_multiplier`).
        let buy10_groups = self.dimensions[tier].bought / 10;
        if buy10_groups > 0 {
            mult *= self
                .buy_ten_multiplier()
                .pow(&Decimal::from_float(buy10_groups as f64));
        }

        // Dim boost: power^max(0, boosts - tier)
        // tier is 0-indexed; JS formula is
        // power^(boosts + 1 - js_tier) where js_tier = tier+1
        // so exponent = boosts + 1 - (tier + 1) = boosts - tier.
        // Power is 2, or 2.5 with the `dimboostMult` Infinity Upgrade.
        let exponent = (self.dim_boosts as i64 - tier as i64).max(0);
        if exponent > 0 {
            mult *= self
                .dim_boost_power()
                .pow(&Decimal::from_float(exponent as f64));
        }

        // Sacrifice multiplier applies only to 8th dimension
        if tier == 7 {
            mult *= self.sacrifice_multiplier();
        }

        // Achievement effects. The global achievement power applies to every
        // dimension; achievements 28 / 23 boost the 1st / 8th dimension by 10%.
        mult *= self.achievement_power();
        if tier == 0 && self.achievement_unlocked(28) {
            mult *= Decimal::from_float(1.1);
        }
        if tier == 7 && self.achievement_unlocked(23) {
            mult *= Decimal::from_float(1.1);
        }

        // Infinity Upgrade multipliers: the common (all-tier) time multipliers and
        // the per-tier dim-pair / unspent-IP multipliers.
        mult *= self.infinity_upgrade_common_mult();
        mult *= self.infinity_upgrade_tier_mult(tier);

        // The original clamps the final per-tier multiplier to >= 1
        // (applyNDMultipliers). Pre-Infinity every term was >= 1, but
        // `totalTimeMult` can dip below 1 for a very short total play time, so
        // clamp to stay faithful.
        mult.max(&Decimal::ONE)
    }

    /// Get the per-second production rate for a dimension tier.
    /// Each dimension produces the tier below it (AD8 produces
    /// AD7, ..., AD1 produces antimatter).
    /// Production = amount * multiplier * tickspeed_effect
    pub fn dimension_production_per_second(&self, tier: usize) -> Decimal {
        if tier >= 8 || !self.is_dimension_unlocked(tier) {
            return Decimal::ZERO;
        }

        let amount = self.dimensions[tier].amount;
        let multiplier = self.dimension_multiplier(tier);
        let tickspeed_effect = self.tickspeed_effect();

        amount * multiplier * tickspeed_effect
    }
}
