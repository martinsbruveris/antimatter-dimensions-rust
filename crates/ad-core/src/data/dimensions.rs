use break_infinity::Decimal;

use super::constants::{AD_BASE_COSTS, AD_COST_MULTIPLIERS};

/// Configuration for a single antimatter dimension tier.
#[derive(Debug, Clone)]
pub struct DimensionConfig {
    /// Base cost to buy the first of this dimension.
    pub base_cost: Decimal,
    /// Cost multiplier per 10 purchases.
    pub cost_multiplier: Decimal,
}

/// Static configuration for all 8 antimatter dimension tiers.
pub fn antimatter_dimension_configs() -> [DimensionConfig; 8] {
    std::array::from_fn(|i| DimensionConfig {
        base_cost: Decimal::from_float(AD_BASE_COSTS[i]),
        cost_multiplier: Decimal::from_float(AD_COST_MULTIPLIERS[i]),
    })
}
