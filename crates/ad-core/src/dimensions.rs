use break_infinity::Decimal;

use crate::data::constants::{
    DIM_BOOST_MULTIPLIER, SACRIFICE_EXPONENT, SACRIFICE_MIN_AMOUNT,
};
use crate::state::GameState;

impl GameState {
    /// Try to buy one of the specified dimension tier (0-indexed).
    /// Returns true if the purchase was successful.
    pub fn buy_dimension(&mut self, tier: usize) -> bool {
        if tier >= 8 || !self.is_dimension_unlocked(tier) {
            return false;
        }

        if self.antimatter >= self.dimensions[tier].cost {
            self.antimatter -= self.dimensions[tier].cost;
            self.dimensions[tier].amount += Decimal::from_float(1.0);
            self.dimensions[tier].bought += 1;
            // Cost increases by the tier's cost multiplier
            self.dimensions[tier].cost *= self.dimensions[tier].cost_multiplier;
            true
        } else {
            false
        }
    }

    /// Buy the maximum number of the specified dimension tier that can be afforded.
    /// Returns the number of dimensions bought.
    pub fn buy_max_dimension(&mut self, tier: usize) -> u64 {
        let mut count = 0u64;
        while self.buy_dimension(tier) {
            count += 1;
        }
        count
    }

    /// Compute the production multiplier for a given dimension tier.
    /// This includes:
    /// - Dimension boost multiplier (2x per boost)
    /// - Sacrifice multiplier (only for tier 8 / index 7)
    pub fn dimension_multiplier(&self, tier: usize) -> Decimal {
        let mut mult = Decimal::from_float(1.0);

        // Dimension boosts: each boost gives DIM_BOOST_MULTIPLIER to all dimensions
        if self.dim_boosts > 0 {
            let boost_mult =
                Decimal::from_float(DIM_BOOST_MULTIPLIER.powi(self.dim_boosts as i32));
            mult *= boost_mult;
        }

        // Sacrifice multiplier applies only to 8th dimension (index 7)
        if tier == 7 {
            mult *= self.sacrifice_multiplier();
        }

        mult
    }

    /// Compute the current sacrifice multiplier for the 8th dimension.
    /// Formula: max(sacrificed / 10, 1) ^ SACRIFICE_EXPONENT
    /// This is a simplified pre-infinity formula.
    pub fn sacrifice_multiplier(&self) -> Decimal {
        if self.sacrificed <= Decimal::from_float(SACRIFICE_MIN_AMOUNT) {
            return Decimal::from_float(1.0);
        }

        let ratio = self.sacrificed / Decimal::from_float(SACRIFICE_MIN_AMOUNT);
        let exponent = Decimal::from_float(SACRIFICE_EXPONENT);
        ratio.pow(&exponent)
    }

    /// Get the per-second production rate for a dimension tier.
    /// Each dimension produces the tier below it (AD8 produces AD7, ..., AD1 produces
    /// antimatter). Production = amount * multiplier * tickspeed_effect
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
