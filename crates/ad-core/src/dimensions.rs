use break_infinity::Decimal;

use crate::data::constants::{
    AD_BASE_COSTS, AD_COST_MULTIPLIERS, BUY_TEN_MULTIPLIER, DIM_BOOST_MULTIPLIER,
};
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
        if tier >= 8 || !self.is_dimension_unlocked(tier) {
            return false;
        }

        let cost = self.dimension_cost(tier);
        if self.antimatter >= cost {
            self.antimatter -= cost;
            self.dimensions[tier].amount += Decimal::from_float(1.0);
            self.dimensions[tier].bought += 1;
            true
        } else {
            false
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

    /// Compute the production multiplier for a given dimension
    /// tier (0-indexed). Includes:
    /// - Buy-10 multiplier (2x per 10 purchases)
    /// - Tier-dependent dim boost multiplier
    /// - Sacrifice multiplier (only for tier 8 / index 7)
    pub fn dimension_multiplier(&self, tier: usize) -> Decimal {
        let mut mult = Decimal::from_float(1.0);

        // Buy-10 multiplier: 2^(bought / 10)
        let buy10_groups = self.dimensions[tier].bought / 10;
        if buy10_groups > 0 {
            let buy10_mult = Decimal::from_float(BUY_TEN_MULTIPLIER)
                .pow(&Decimal::from_float(buy10_groups as f64));
            mult *= buy10_mult;
        }

        // Dim boost: power^max(0, boosts - tier)
        // tier is 0-indexed; JS formula is
        // power^(boosts + 1 - js_tier) where js_tier = tier+1
        // so exponent = boosts + 1 - (tier + 1) = boosts - tier
        let exponent = (self.dim_boosts as i64 - tier as i64).max(0);
        if exponent > 0 {
            let boost_mult = Decimal::from_float(DIM_BOOST_MULTIPLIER)
                .pow(&Decimal::from_float(exponent as f64));
            mult *= boost_mult;
        }

        // Sacrifice multiplier applies only to 8th dimension
        if tier == 7 {
            mult *= self.sacrifice_multiplier();
        }

        mult
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
