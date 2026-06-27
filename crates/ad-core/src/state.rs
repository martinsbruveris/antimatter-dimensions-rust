use break_infinity::Decimal;

use crate::autobuyers::AutobuyerState;
use crate::data::constants::{
    INITIAL_ANTIMATTER, TICKSPEED_BASE_COST, TICKSPEED_COST_MULTIPLIER,
};

/// A single antimatter dimension tier.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DimensionTier {
    /// Current amount of this dimension (can be fractional due
    /// to production).
    pub amount: Decimal,
    /// Number of individual purchases made.
    pub bought: u64,
}

impl DimensionTier {
    pub fn new() -> Self {
        Self {
            amount: Decimal::ZERO,
            bought: 0,
        }
    }
}

impl Default for DimensionTier {
    fn default() -> Self {
        Self::new()
    }
}

/// Tickspeed state: controls how fast dimensions produce.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameState {
    /// Current antimatter amount.
    pub antimatter: Decimal,
    /// All-time total antimatter ever produced. Mirrors JS
    /// `player.records.totalAntimatter`: monotonic and **not** reset by
    /// a Big Crunch. Gates the Automation tab and autobuyer unlocks.
    pub total_antimatter: Decimal,
    /// All 8 antimatter dimension tiers.
    pub dimensions: [DimensionTier; 8],
    /// Tickspeed upgrade state.
    pub tickspeed: TickspeedState,
    /// Number of dimension boosts performed.
    pub dim_boosts: u32,
    /// Number of antimatter galaxies purchased.
    pub galaxies: u32,
    /// Total antimatter sacrificed (cumulative across all
    /// sacrifices).
    pub sacrificed: Decimal,
    /// Whether the player has performed at least one Big Crunch. Mirrors JS
    /// `PlayerProgress.infinityUnlocked()`: set on the first crunch and
    /// **not** reset by subsequent crunches. Gates Infinity-related UI (e.g.
    /// the "Infinity" How To Play entry).
    #[cfg_attr(feature = "serde", serde(default))]
    pub infinity_unlocked: bool,
    /// Autobuyer state for dimensions and tickspeed.
    pub autobuyers: AutobuyerState,
}

impl GameState {
    pub fn new() -> Self {
        let dimensions = std::array::from_fn(|_| DimensionTier::new());

        Self {
            antimatter: Decimal::from_float(INITIAL_ANTIMATTER),
            total_antimatter: Decimal::from_float(INITIAL_ANTIMATTER),
            dimensions,
            tickspeed: TickspeedState::new(),
            dim_boosts: 0,
            galaxies: 0,
            sacrificed: Decimal::ZERO,
            infinity_unlocked: false,
            autobuyers: AutobuyerState::new(),
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

    /// Returns whether dimensional sacrifice is unlocked.
    /// In JS: requires `DimBoost.purchasedBoosts > 4` (i.e.,
    /// ≥ 5 boosts, which means all 8 dims are unlocked plus
    /// one extra boost).
    pub fn sacrifice_unlocked(&self) -> bool {
        self.dim_boosts >= 5
    }

    /// Returns whether Tickspeed is unlocked. In JS
    /// `Tickspeed.isUnlocked` requires `AntimatterDimension(2).bought > 0`
    /// (the later Eternity/Reality unlock conditions don't exist yet).
    pub fn tickspeed_unlocked(&self) -> bool {
        self.dimensions[1].bought > 0
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
