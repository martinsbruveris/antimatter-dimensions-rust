use break_infinity::Decimal;

use crate::state::GameState;

/// Observed dimension tier — mirrors `DimensionTier` plus
/// computed fields materialised at snapshot time.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObservedDimensionTier {
    /// Current amount (can be fractional due to production).
    pub amount: Decimal,
    /// Number of individual purchases made.
    pub bought: u64,
    /// Current production multiplier for this tier.
    pub multiplier: Decimal,
    /// Production rate per second for this tier.
    pub production_per_second: Decimal,
    /// Cost of the next single-unit purchase of this tier.
    pub cost: Decimal,
    /// Whether this tier is currently unlocked (within the dim-boost band).
    pub unlocked: bool,
    /// Whether this tier can be purchased right now (band **and** the tier
    /// below it owned). Drives the buy gate and the dimmed `not-reached` style.
    pub available_for_purchase: bool,
    /// Whether this tier's row should be shown (progressive reveal): shown by
    /// the reveal/lookahead rules or because the player already owns some.
    pub shown: bool,
}

impl ObservedDimensionTier {
    fn from_game_state(game: &GameState, tier: usize) -> Self {
        Self {
            amount: game.dimensions[tier].amount,
            bought: game.dimensions[tier].bought,
            multiplier: game.dimension_multiplier(tier),
            production_per_second: game.dimension_production_per_second(tier),
            cost: game.dimension_cost(tier),
            unlocked: game.is_dimension_unlocked(tier),
            available_for_purchase: game.dim_available_for_purchase(tier),
            shown: game.dim_is_shown(tier)
                || game.dimensions[tier].amount > Decimal::ZERO,
        }
    }
}

/// Observed tickspeed state — mirrors `TickspeedState` plus
/// the computed tickspeed interval and production effect.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObservedTickspeedState {
    /// Number of tickspeed upgrades purchased.
    pub bought: u64,
    /// Current cost to buy the next tickspeed upgrade.
    pub cost: Decimal,
    /// Cost multiplier per purchase.
    pub cost_multiplier: Decimal,
    /// Current tickspeed interval in milliseconds.
    pub tickspeed_ms: f64,
    /// Production multiplier from tickspeed.
    pub tickspeed_effect: Decimal,
}

impl ObservedTickspeedState {
    fn from_game_state(game: &GameState) -> Self {
        Self {
            bought: game.tickspeed.bought,
            cost: game.tickspeed.cost,
            cost_multiplier: game.tickspeed.cost_multiplier,
            tickspeed_ms: game.current_tickspeed_ms(),
            tickspeed_effect: game.tickspeed_effect(),
        }
    }
}

/// A materialised view of the game state at a point in time.
///
/// Contains all fields from `GameState` plus computed values
/// that are derived from game state but not stored in it.
/// This is what gets passed to Python for analysis.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObservedState {
    /// Current antimatter amount.
    pub antimatter: Decimal,
    /// All 8 antimatter dimension tiers with computed fields.
    pub dimensions: [ObservedDimensionTier; 8],
    /// Tickspeed state with computed fields.
    pub tickspeed: ObservedTickspeedState,
    /// Number of dimension boosts performed.
    pub dim_boosts: u32,
    /// Number of antimatter galaxies purchased.
    pub galaxies: u32,
    /// Total antimatter sacrificed (cumulative).
    pub sacrificed: Decimal,
    /// Running product of all sacrifice boosts.
    pub sacrifice_boost: Decimal,
    /// Whether sacrifice is unlocked.
    pub sacrifice_unlocked: bool,
    /// Number of dimension tiers currently unlocked.
    pub unlocked_dimensions: usize,
    /// Whether an antimatter galaxy can be bought right now.
    pub can_buy_galaxy: bool,
    /// Whether a dimension boost can be bought right now.
    pub can_dim_boost: bool,
    /// Whether a sacrifice can be performed right now.
    pub can_sacrifice: bool,
    /// Gain ratio the next sacrifice would yield (the
    /// `nextBoost` value). 1 when sacrifice is not worthwhile.
    pub next_sacrifice_boost: Decimal,
    /// Sorted ids of unlocked normal achievements (the semantic view of
    /// `GameState::achievement_bits`).
    pub unlocked_achievements: Vec<u16>,
    /// Global achievement-power multiplier applied to every dimension.
    pub achievement_power: Decimal,
}

impl ObservedState {
    /// Construct an observed state by reading all fields from
    /// the game state and computing derived values.
    ///
    /// The computed affordability fields (`can_*`,
    /// `next_sacrifice_boost`, per-tier `cost`) make this a
    /// complete decision input for an external controller: a
    /// driver can choose among legal actions without holding a
    /// mutable borrow of the engine or re-deriving game rules.
    pub fn from_game_state(game: &GameState) -> Self {
        Self {
            antimatter: game.antimatter,
            dimensions: std::array::from_fn(|tier| {
                ObservedDimensionTier::from_game_state(game, tier)
            }),
            tickspeed: ObservedTickspeedState::from_game_state(game),
            dim_boosts: game.dim_boosts,
            galaxies: game.galaxies,
            sacrificed: game.sacrificed,
            sacrifice_boost: game.sacrifice_multiplier(),
            sacrifice_unlocked: game.sacrifice_unlocked(),
            unlocked_dimensions: game.unlocked_dimensions(),
            can_buy_galaxy: game.can_buy_galaxy(),
            can_dim_boost: game.can_dim_boost(),
            can_sacrifice: game.can_sacrifice(),
            next_sacrifice_boost: game.next_sacrifice_boost(),
            unlocked_achievements: game.unlocked_achievement_ids(),
            achievement_power: game.achievement_power(),
        }
    }
}
