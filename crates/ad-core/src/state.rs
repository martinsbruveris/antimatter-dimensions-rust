use break_infinity::Decimal;

use crate::data::constants::{
    AD_BASE_COSTS, AD_COST_MULTIPLIERS, INITIAL_ANTIMATTER, TICKSPEED_BASE_COST,
    TICKSPEED_COST_MULTIPLIER,
};

/// A single antimatter dimension tier.
#[derive(Debug, Clone)]
pub struct DimensionTier {
    /// Current amount of this dimension (can be fractional due to production).
    pub amount: Decimal,
    /// Number of individual purchases made.
    pub bought: u64,
    /// Current cost to buy the next one.
    pub cost: Decimal,
    /// Cost multiplier applied per purchase.
    pub cost_multiplier: Decimal,
}

impl DimensionTier {
    pub fn new(base_cost: Decimal, cost_multiplier: Decimal) -> Self {
        Self {
            amount: Decimal::ZERO,
            bought: 0,
            cost: base_cost,
            cost_multiplier,
        }
    }
}

/// Tickspeed state: controls how fast dimensions produce.
#[derive(Debug, Clone)]
pub struct TickspeedState {
    /// Number of tickspeed upgrades purchased.
    pub bought: u64,
    /// Current cost to buy the next tickspeed upgrade.
    pub cost: Decimal,
    /// Cost multiplier per purchase.
    pub cost_multiplier: Decimal,
}

impl Default for TickspeedState {
    fn default() -> Self {
        Self::new()
    }
}

impl TickspeedState {
    pub fn new() -> Self {
        Self {
            bought: 0,
            cost: Decimal::from_float(TICKSPEED_BASE_COST),
            cost_multiplier: Decimal::from_float(TICKSPEED_COST_MULTIPLIER),
        }
    }
}

/// Full game state for pre-infinity gameplay.
#[derive(Debug, Clone)]
pub struct GameState {
    /// Current antimatter amount.
    pub antimatter: Decimal,
    /// All 8 antimatter dimension tiers.
    pub dimensions: [DimensionTier; 8],
    /// Tickspeed upgrade state.
    pub tickspeed: TickspeedState,
    /// Number of dimension boosts performed.
    pub dim_boosts: u32,
    /// Number of antimatter galaxies purchased.
    pub galaxies: u32,
    /// Total antimatter sacrificed (cumulative across all sacrifices).
    pub sacrificed: Decimal,
    /// Whether sacrifice is unlocked (requires 5th dimension to be unlocked).
    pub sacrifice_unlocked: bool,
}

impl GameState {
    pub fn new() -> Self {
        let dimensions = std::array::from_fn(|i| {
            DimensionTier::new(
                Decimal::from_float(AD_BASE_COSTS[i]),
                Decimal::from_float(AD_COST_MULTIPLIERS[i]),
            )
        });

        Self {
            antimatter: Decimal::from_float(INITIAL_ANTIMATTER),
            dimensions,
            tickspeed: TickspeedState::new(),
            dim_boosts: 0,
            galaxies: 0,
            sacrificed: Decimal::ZERO,
            sacrifice_unlocked: false,
        }
    }

    /// Returns how many dimension tiers are currently unlocked.
    /// Starts with 4, each dim boost beyond the first 4 doesn't unlock more.
    /// Dim boost 1 unlocks tier 5, boost 2 unlocks tier 6, etc.
    pub fn unlocked_dimensions(&self) -> usize {
        let base = 4;
        let from_boosts = (self.dim_boosts as usize).min(4);
        base + from_boosts
    }

    /// Returns whether a given dimension tier (0-indexed) is unlocked.
    pub fn is_dimension_unlocked(&self, tier: usize) -> bool {
        tier < self.unlocked_dimensions()
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
