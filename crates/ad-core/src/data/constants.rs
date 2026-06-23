use break_infinity::Decimal;

/// Starting antimatter for a new game (and after galaxy/dim boost resets).
pub const INITIAL_ANTIMATTER: f64 = 10.0;

/// Base costs for each antimatter dimension tier (0-indexed in array).
/// From the original game: 10, 100, 1e4, 1e6, 1e9, 1e13, 1e18, 1e24
pub const AD_BASE_COSTS: [f64; 8] = [1e1, 1e2, 1e4, 1e6, 1e9, 1e13, 1e18, 1e24];

/// Cost multiplier per 10 purchases for each dimension tier.
/// Cost increases every 10th purchase, not every individual purchase.
/// From the original game: 1e3, 1e4, 1e5, 1e6, 1e8, 1e10, 1e12, 1e15
pub const AD_COST_MULTIPLIERS: [f64; 8] = [1e3, 1e4, 1e5, 1e6, 1e8, 1e10, 1e12, 1e15];

/// Initial tickspeed in milliseconds (lower = faster production).
pub const INITIAL_TICKSPEED_MS: f64 = 1000.0;

/// Base cost of a tickspeed upgrade (in antimatter).
pub const TICKSPEED_BASE_COST: f64 = 1000.0;

/// Cost multiplier per tickspeed purchase.
pub const TICKSPEED_COST_MULTIPLIER: f64 = 10.0;

/// Number of 8th dimensions required for the first galaxy.
pub const FIRST_GALAXY_REQUIREMENT: u64 = 80;

/// Additional 8th dimensions required per galaxy after the first.
pub const GALAXY_REQUIREMENT_INCREMENT: u64 = 60;

/// Dimension boost requirements: first 4 boosts require 20 of
/// dims 4-7. After that, each boost requires 15 more 8th dims.
pub const DIM_BOOST_INITIAL_REQUIREMENT: u64 = 20;

/// After unlocking all 8 dims, each subsequent boost requires
/// 15 more 8th dims.
pub const DIM_BOOST_SCALING_REQUIREMENT: u64 = 15;

/// The base multiplier each dimension boost gives.
/// Applied tier-dependently: power^max(0, boosts - tier).
pub const DIM_BOOST_MULTIPLIER: f64 = 2.0;

/// Base multiplier granted per 10 purchases of a dimension.
/// Every 10 bought gives an additional 2x to that dimension.
pub const BUY_TEN_MULTIPLIER: f64 = 2.0;

/// Sacrifice exponent for the pre-infinity formula.
pub const SACRIFICE_EXPONENT: f64 = 2.0;

/// Per-galaxy tickspeed reduction for the pre-3-galaxy linear
/// formula.
pub const GALAXY_TICKSPEED_REDUCTION: f64 = 0.02;

/// Base tickspeed multipliers for the first 3 galaxy counts.
/// 0 galaxies: 1/1.1245, 1 galaxy: 1/1.11888888, 2 galaxies:
/// 1/1.11267177
pub const TICKSPEED_BASE_MULTIPLIERS: [f64; 3] =
    [1.0 / 1.1245, 1.0 / 1.11888888, 1.0 / 1.11267177];

/// Per-galaxy exponential decay factor for 3+ galaxies.
pub const TICKSPEED_GALAXY_DECAY: f64 = 0.965;

/// Base multiplier for the exponential galaxy formula (3+
/// galaxies).
pub const TICKSPEED_GALAXY_BASE: f64 = 0.8;

/// Minimum tickspeed purchase multiplier (floor).
pub const TICKSPEED_MULTIPLIER_MIN: f64 = 0.01;

/// Helper to create a Decimal from a constant f64.
pub fn decimal(value: f64) -> Decimal {
    Decimal::from_float(value)
}
