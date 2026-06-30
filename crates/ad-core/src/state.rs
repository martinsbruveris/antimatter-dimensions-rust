use break_infinity::Decimal;

use crate::achievements::ACHIEVEMENT_ROW_COUNT;
use crate::autobuyers::AutobuyerState;
use crate::data::constants::{
    INITIAL_ANTIMATTER, TICKSPEED_BASE_COST, TICKSPEED_COST_MULTIPLIER,
};
use crate::options::Options;

/// serde default for boolean fields that default to `true` (e.g.
/// `tutorial_active`), since `bool`'s own `Default` is `false`.
#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

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

    /// Rebuilds tickspeed state for a given purchased count, recomputing the
    /// next-purchase cost from our cost formula. The original save stores only
    /// the purchase count (`player.totalTickBought`), never the cost — it is
    /// derived — so on load we recompute it here rather than trusting a saved
    /// value. Matches the geometric accumulation in [`GameState::buy_tickspeed`]
    /// (`cost = base * multiplier^bought`).
    pub fn with_bought(bought: u64) -> Self {
        let mut state = Self::new();
        state.bought = bought;
        state.cost = Decimal::from_float(TICKSPEED_BASE_COST)
            * state.cost_multiplier.pow(&Decimal::from(bought));
        state
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
    /// Normal-achievement unlock state. Mirrors `player.achievementBits`: 18
    /// rows, one bitmask each (`achievement_bits[row-1]` bit `1 << (col-1)`).
    /// Row 18 (the Pelle achievements) is held only so original saves round-trip;
    /// we never unlock it ourselves. Persists forever, including across a Big
    /// Crunch. See `achievements.rs`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub achievement_bits: [u32; ACHIEVEMENT_ROW_COUNT],
    /// Current tutorial-highlight step. Mirrors `player.tutorialState` (an
    /// index into [`tutorial`](crate::tutorial)'s state machine): which early
    /// element gets the gold glow + `!` icon next. Persisted at the `player`
    /// root.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tutorial_state: u8,
    /// Whether the current tutorial step's highlight is showing. Mirrors
    /// `player.tutorialActive` (default `true`); cleared when the player
    /// performs the highlighted action and re-set on advancing to the next step.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub tutorial_active: bool,
    /// Autobuyer state for dimensions and tickspeed.
    pub autobuyers: AutobuyerState,
    /// Player options (UI/UX preferences). Not pre-Infinity progress, so they
    /// are **not** reset by a Big Crunch; they persist for the whole save.
    #[cfg_attr(feature = "serde", serde(default))]
    pub options: Options,
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
            achievement_bits: [0; ACHIEVEMENT_ROW_COUNT],
            tutorial_state: 0,
            tutorial_active: true,
            autobuyers: AutobuyerState::new(),
            options: Options::new(),
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

    /// Whether a dimension tier (0-indexed) can currently be purchased. Mirrors
    /// `AntimatterDimension.isAvailableForPurchase` pre-Infinity: the tier must
    /// be within the dim-boost unlock band **and** the tier below it must be
    /// owned (you cannot buy the 2nd dimension before owning a 1st). The
    /// original is 1-indexed (`tier > totalBoosts + 4`); for our 0-indexed
    /// `tier` that band is `tier > dim_boosts + 3`.
    pub fn dim_available_for_purchase(&self, tier: usize) -> bool {
        if tier >= 8 || tier > self.dim_boosts as usize + 3 {
            return false;
        }
        tier == 0 || self.dimensions[tier - 1].amount > Decimal::ZERO
    }

    /// Whether a dimension row should be shown, minus the per-row `amount > 0`
    /// term the view adds on top. Mirrors the original row's
    /// `isShown || isUnlocked`, where `isShown = (totalBoosts > 0 &&
    /// totalBoosts + 3 >= tier) || infinityUnlocked` (1-indexed `tier`). The
    /// lookahead reveals the next couple of rows just before they're purchasable
    /// once the first boost is bought.
    pub fn dim_is_shown(&self, tier: usize) -> bool {
        let lookahead =
            self.dim_boosts > 0 && self.dim_boosts as usize + 2 >= tier;
        lookahead || self.infinity_unlocked || self.dim_available_for_purchase(tier)
    }

    /// Returns whether dimensional sacrifice is **visible**. Mirrors the
    /// original's `Sacrifice.isVisible = Achievement(18).isUnlocked`:
    /// achievement 18 unlocks the first time an 8th Antimatter Dimension is
    /// bought and persists forever (including across a Big Crunch), so the
    /// button stays visible once seen. This is the *visibility* gate, distinct
    /// from the *enable* gate [`can_sacrifice`](Self::can_sacrifice).
    pub fn sacrifice_unlocked(&self) -> bool {
        self.achievement_unlocked(18)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_first_dimension_purchasable_at_start() {
        let game = GameState::new();
        assert!(game.dim_available_for_purchase(0));
        // The 2nd–8th cannot be bought before owning the tier below.
        for tier in 1..8 {
            assert!(!game.dim_available_for_purchase(tier));
        }
    }

    #[test]
    fn nth_dimension_purchasable_once_previous_owned() {
        let mut game = GameState::new();
        game.dim_boosts = 4; // unlock band covers all 8 tiers
        for tier in 1..8 {
            assert!(!game.dim_available_for_purchase(tier));
            game.dimensions[tier - 1].amount = Decimal::ONE;
            assert!(game.dim_available_for_purchase(tier));
        }
    }

    #[test]
    fn unlock_band_gates_purchasability() {
        let mut game = GameState::new();
        game.dimensions[3].amount = Decimal::ONE; // own a 4th
        // The 5th (index 4) needs the first dim boost regardless of ownership.
        assert!(!game.dim_available_for_purchase(4));
        game.dim_boosts = 1;
        assert!(game.dim_available_for_purchase(4));
    }

    #[test]
    fn only_first_row_shown_at_start() {
        let game = GameState::new();
        assert!(game.dim_is_shown(0));
        for tier in 1..8 {
            assert!(!game.dim_is_shown(tier));
        }
    }

    #[test]
    fn shown_row_unfolds_with_ownership() {
        let mut game = GameState::new();
        game.dimensions[0].amount = Decimal::ONE; // own a 1st
        // The 2nd row becomes shown because it is now purchasable.
        assert!(game.dim_is_shown(1));
        assert!(!game.dim_is_shown(2));
    }

    #[test]
    fn first_boost_reveals_lookahead_rows() {
        let mut game = GameState::new();
        // Before any boost the 5th row (index 4) is hidden.
        assert!(!game.dim_is_shown(4));
        game.dim_boosts = 1;
        // boosts + 2 >= tier → 3 >= 4 is false; the 5th stays hidden, but the
        // lookahead now reveals up to index 3 even without ownership.
        assert!(game.dim_is_shown(3));
        game.dim_boosts = 2;
        assert!(game.dim_is_shown(4));
    }

    #[test]
    fn sacrifice_visibility_tracks_achievement_18() {
        let mut game = GameState::new();
        assert!(!game.sacrifice_unlocked());
        game.unlock_achievement(18);
        assert!(game.sacrifice_unlocked());
    }
}
