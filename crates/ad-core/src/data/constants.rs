use break_infinity::Decimal;

/// Starting antimatter for a new game (and after galaxy/dim boost resets).
pub const INITIAL_ANTIMATTER: f64 = 10.0;

/// Base costs for each antimatter dimension tier (1-indexed conceptually, 0-indexed in array).
/// From the original game: 10, 100, 1e4, 1e6, 1e9, 1e13, 1e18, 1e24
pub const AD_BASE_COSTS: [f64; 8] = [1e1, 1e2, 1e4, 1e6, 1e9, 1e13, 1e18, 1e24];

/// Cost multiplier per purchase for each dimension tier.
/// From the original game: 1e3, 1e4, 1e5, 1e6, 1e8, 1e10, 1e12, 1e15
pub const AD_COST_MULTIPLIERS: [f64; 8] = [1e3, 1e4, 1e5, 1e6, 1e8, 1e10, 1e12, 1e15];

/// Initial tickspeed in milliseconds (lower = faster production).
pub const INITIAL_TICKSPEED_MS: f64 = 1000.0;

/// Base cost of a tickspeed upgrade (in antimatter).
pub const TICKSPEED_BASE_COST: f64 = 1000.0;

/// Cost multiplier per tickspeed purchase.
pub const TICKSPEED_COST_MULTIPLIER: f64 = 10.0;

/// Base tickspeed multiplier per purchase (fraction of current tickspeed retained).
/// Each purchase multiplies tickspeed by this value (making it faster).
pub const TICKSPEED_MULTIPLIER: f64 = 0.88;

/// The galaxy tickspeed reduction. Each galaxy reduces the tickspeed multiplier by this amount.
/// e.g., with 1 galaxy the multiplier becomes 0.88 - 0.02 = 0.86
pub const GALAXY_TICKSPEED_REDUCTION: f64 = 0.02;

/// Minimum tickspeed multiplier before switching to logarithmic scaling.
pub const TICKSPEED_MULTIPLIER_MIN: f64 = 0.02;

/// Number of 8th dimensions required for the first galaxy.
pub const FIRST_GALAXY_REQUIREMENT: u64 = 80;

/// Additional 8th dimensions required per galaxy after the first.
pub const GALAXY_REQUIREMENT_INCREMENT: u64 = 60;

/// Dimension boost requirements: (dimension_tier_required, amount_required).
/// First 4 boosts unlock dims 5-8 (require 20 of dim 4, 5, 6, 7).
/// After that, each boost requires 20 more 8th dimensions than the last.
pub const DIM_BOOST_INITIAL_REQUIREMENT: u64 = 20;

/// After unlocking all 8 dims, each subsequent boost requires 15 more 8th dims.
pub const DIM_BOOST_SCALING_REQUIREMENT: u64 = 15;

/// The multiplier each dimension boost gives to all dimensions.
pub const DIM_BOOST_MULTIPLIER: f64 = 2.0;

/// Sacrifice is unlocked after the first dimension boost that unlocks the 5th dimension.
/// The sacrifice multiplier formula: max(sacrificed_amount / 10, 1) ^ 0.05
/// (simplified pre-infinity formula)
pub const SACRIFICE_EXPONENT: f64 = 2.0;

/// Minimum sacrifice multiplier (no effect below this).
pub const SACRIFICE_MIN_AMOUNT: f64 = 10.0;

/// Helper to create a Decimal from a constant f64.
pub fn decimal(value: f64) -> Decimal {
    Decimal::from_float(value)
}
