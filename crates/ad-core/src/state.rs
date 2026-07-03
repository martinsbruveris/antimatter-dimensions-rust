use break_infinity::Decimal;

use crate::achievements::ACHIEVEMENT_ROW_COUNT;
use crate::autobuyers::AutobuyerState;
use crate::challenges::NormalChallengeState;
use crate::data::constants::{
    INITIAL_ANTIMATTER, TICKSPEED_BASE_COST, TICKSPEED_COST_MULTIPLIER,
};
use crate::infinity_challenges::InfinityChallengeState;
use crate::infinity_dimensions::InfinityDimension;
use crate::options::Options;
use crate::records::Records;
use crate::replicanti::ReplicantiState;

/// serde default for boolean fields that default to `true` (e.g.
/// `tutorial_active`), since `bool`'s own `Default` is `false`.
#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

/// serde default for `Decimal` fields whose neutral value is `1` (e.g.
/// `chall8_total_sacrifice`, `matter`), since `Decimal`'s own `Default` is `0`.
#[cfg(feature = "serde")]
fn default_decimal_one() -> Decimal {
    Decimal::ONE
}

/// serde default for `chall2_pow` (`1` = full production), since `f64`'s own
/// `Default` is `0`.
#[cfg(feature = "serde")]
fn default_f64_one() -> f64 {
    1.0
}

/// serde default for `chall3_pow` — NC3's 1st-dimension multiplier starts at
/// `0.01` (`player.chall3Pow` default).
#[cfg(feature = "serde")]
fn default_chall3_pow() -> Decimal {
    Decimal::from_float(0.01)
}

/// serde default for `post_c4_tier` (`1`, `player.postC4Tier` default).
#[cfg(feature = "serde")]
fn default_post_c4_tier() -> u8 {
    1
}

/// serde default for `infinity_dimensions` (8 fresh, locked tiers).
#[cfg(feature = "serde")]
fn default_infinity_dimensions() -> [InfinityDimension; 8] {
    std::array::from_fn(InfinityDimension::new)
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
    /// Extra cost-scaling steps beyond `bought / 10` (`data.costBumps`). Normal
    /// Challenge 9 bumps this on other dimensions of equal cost when you buy a
    /// group of 10 or a Tickspeed upgrade; otherwise 0. Folds into the cost
    /// exponent: `base × mult^(bought/10 + cost_bumps)`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub cost_bumps: u64,
}

impl DimensionTier {
    pub fn new() -> Self {
        Self {
            amount: Decimal::ZERO,
            bought: 0,
            cost_bumps: 0,
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
    /// Extra cost-scaling steps beyond `bought` (`player.chall9TickspeedCostBumps`).
    /// Normal Challenge 9 bumps this when you buy a group of 10 Antimatter
    /// Dimensions of equal cost; otherwise 0. The stored `cost` already reflects
    /// it (invariant: `cost = base × mult^(bought + cost_bumps)`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub cost_bumps: u64,
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
            cost_bumps: 0,
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
        Self::with_bought_and_bumps(bought, 0)
    }

    /// Like [`with_bought`](Self::with_bought) but also applies Normal-Challenge-9
    /// tickspeed cost bumps (`player.chall9TickspeedCostBumps`), which shift the
    /// cost by extra multiplier steps: `cost = base × mult^(bought + cost_bumps)`.
    pub fn with_bought_and_bumps(bought: u64, cost_bumps: u64) -> Self {
        let mut state = Self::new();
        state.bought = bought;
        state.cost_bumps = cost_bumps;
        state.cost = Decimal::from_float(TICKSPEED_BASE_COST)
            * state
                .cost_multiplier
                .pow(&Decimal::from(bought + cost_bumps));
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
    /// Current Infinity Points. Mirrors `Currency.infinityPoints`: gained on a
    /// Big Crunch, cumulative, and **not** reset by subsequent crunches (reset
    /// only on Eternity, a later feature). The currency the Infinity Upgrades
    /// tab spends. See `crunch.rs::gained_infinity_points`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub infinity_points: Decimal,
    /// Number of Infinities performed. Mirrors `Currency.infinities`:
    /// incremented on each Big Crunch and persists across crunches. Shown in the
    /// Statistics tab and feeds later infinity-count multipliers.
    #[cfg_attr(feature = "serde", serde(default))]
    pub infinities: Decimal,
    /// Owned Infinity Upgrades, one bit per [`InfinityUpgrade`](crate::InfinityUpgrade)
    /// (the original's `player.infinityUpgrades` string set as a bitmask).
    /// Persists across a Big Crunch. See `infinity_upgrades.rs`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub infinity_upgrades: u32,
    /// Fractional Infinity-Point accumulator for the `ipGen` upgrade's passive
    /// generation (mirrors `player.partInfinityPoint`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub part_infinity_point: f64,
    /// Normal-challenge run state (active challenge + completed bitmask). Mirrors
    /// `player.challenge.normal`; persists across a Big Crunch. See `challenges.rs`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub challenge: NormalChallengeState,
    /// Infinity-challenge run state (active challenge + completed bitmask). Mirrors
    /// `player.challenge.infinity`; persists across a Big Crunch. See
    /// `infinity_challenges.rs`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub infinity_challenge: InfinityChallengeState,
    /// The most recently bought Antimatter Dimension tier (1-indexed,
    /// `player.postC4Tier`). Under Infinity Challenge 4 only this tier produces
    /// normally. Default 1; set on each AD purchase, reset with challenge state.
    #[cfg_attr(feature = "serde", serde(default = "default_post_c4_tier"))]
    pub post_c4_tier: u8,
    /// Infinity-Challenge-2 auto-sacrifice accumulator in ms (`player.ic2Count`):
    /// while IC2 runs, a Dimensional Sacrifice fires every 400 ms.
    #[cfg_attr(feature = "serde", serde(default))]
    pub ic2_count: f64,
    /// Normal-Challenge-8 accumulated sacrifice boost (`player.chall8TotalSacrifice`).
    /// Under NC8 dimensional sacrifice uses a running product kept across
    /// sacrifice resets rather than the log-based total-boost formula; this holds
    /// it. Starts at 1, advanced on each NC8 sacrifice, and reset by
    /// [`reset_challenge_stuff`](GameState::reset_challenge_stuff). See `sacrifice.rs`.
    #[cfg_attr(feature = "serde", serde(default = "default_decimal_one"))]
    pub chall8_total_sacrifice: Decimal,
    /// Normal-Challenge-2 production factor (`player.chall2Pow`, a plain number in
    /// `[0, 1]`). NC2 halts all Antimatter Dimension production on any AD/tickspeed
    /// purchase (set to 0) and recovers linearly to 1 over 3 minutes. `1` = full
    /// production. Advanced in `tick`, reset by [`reset_challenge_stuff`]
    /// (GameState::reset_challenge_stuff). See `tick.rs` / `dimensions.rs`.
    #[cfg_attr(feature = "serde", serde(default = "default_f64_one"))]
    pub chall2_pow: f64,
    /// Normal-Challenge-3 first-dimension multiplier (`player.chall3Pow`). NC3
    /// weakens AD1 to ×0.01 but grows this multiplier exponentially over time
    /// (×1.00038 per 100 ms), uncapped up to `Number.MAX_VALUE`. Reset to `0.01`
    /// by [`reset_challenge_stuff`](GameState::reset_challenge_stuff) (i.e. after
    /// Dimension Boosts and Galaxies). See `tick.rs` / `dimensions.rs`.
    #[cfg_attr(feature = "serde", serde(default = "default_chall3_pow"))]
    pub chall3_pow: Decimal,
    /// Normal-matter amount (`Currency.matter` / `player.matter`). Rises under
    /// Normal Challenge 11 once a 2nd Antimatter Dimension exists; if it exceeds
    /// antimatter (and you cannot yet Crunch) it annihilates — a Dimension-Boost-
    /// style soft reset with no boost. `reset_challenge_stuff` resets it to `0`
    /// (`Currency.matter.reset()`). See `tick.rs`.
    #[cfg_attr(feature = "serde", serde(default = "default_decimal_one"))]
    pub matter: Decimal,
    /// Whether the player has performed at least one Big Crunch. Mirrors JS
    /// `PlayerProgress.infinityUnlocked()`: set on the first crunch and
    /// **not** reset by subsequent crunches. Gates Infinity-related UI (e.g.
    /// the "Infinity" How To Play entry).
    #[cfg_attr(feature = "serde", serde(default))]
    pub infinity_unlocked: bool,
    /// Whether the player has bought Break Infinity (`player.break`): antimatter
    /// may then exceed `1e308` and the Infinity-Point formula scales with how far
    /// past the cap it goes. A permanent unlock (reset only on Eternity, later);
    /// distinct from [`infinity_unlocked`](Self::infinity_unlocked). See
    /// `crunch.rs` / `tick.rs`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub broke_infinity: bool,
    /// Owned one-time Break Infinity Upgrades, one bit per
    /// [`BreakInfinityUpgrade`](crate::BreakInfinityUpgrade). Shares the original's
    /// `player.infinityUpgrades` string set with the [`InfinityUpgrade`]s but is a
    /// distinct bitmask here. Persists across a Big Crunch. See
    /// `break_infinity_upgrades.rs`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub break_infinity_upgrades: u32,
    /// Purchase counts of the three rebuyable Break Infinity Upgrades
    /// (`player.infinityRebuyables`): `[tickspeedCostMult, dimCostMult, ipGen]`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub infinity_rebuyables: [u32; 3],
    /// The 8 Infinity Dimensions (bought with IP, produce Infinity Power). Their
    /// purchases/cost/unlock persist across a Big Crunch; the `amount` resets. See
    /// `infinity_dimensions.rs`.
    #[cfg_attr(feature = "serde", serde(default = "default_infinity_dimensions"))]
    pub infinity_dimensions: [InfinityDimension; 8],
    /// Infinity Power (`Currency.infinityPower`): produced by the 1st Infinity
    /// Dimension, gives an `^7` multiplier to all Antimatter Dimensions; reset on a
    /// Big Crunch.
    #[cfg_attr(feature = "serde", serde(default))]
    pub infinity_power: Decimal,
    /// Replicanti (`player.replicanti`): self-replicating entities unlocked with
    /// Infinity Points that multiply Infinity Dimensions and can be spent on
    /// Replicanti Galaxies. Persists across a Big Crunch (reset only on Eternity,
    /// later). See `replicanti.rs`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub replicanti: ReplicantiState,
    /// Time and prestige records (total time played, this/best infinity). Mirrors
    /// the modelled slice of `player.records`. Advanced in `tick`; the current
    /// infinity's records reset on a Big Crunch. See [`Records`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub records: Records,
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
            infinity_points: Decimal::ZERO,
            infinities: Decimal::ZERO,
            infinity_upgrades: 0,
            part_infinity_point: 0.0,
            challenge: NormalChallengeState::default(),
            infinity_challenge: InfinityChallengeState::default(),
            post_c4_tier: 1,
            ic2_count: 0.0,
            chall8_total_sacrifice: Decimal::ONE,
            chall2_pow: 1.0,
            chall3_pow: Decimal::from_float(0.01),
            matter: Decimal::ONE,
            infinity_unlocked: false,
            broke_infinity: false,
            break_infinity_upgrades: 0,
            infinity_rebuyables: [0; 3],
            infinity_dimensions: std::array::from_fn(InfinityDimension::new),
            infinity_power: Decimal::ZERO,
            replicanti: ReplicantiState::new(),
            records: Records::new(),
            achievement_bits: [0; ACHIEVEMENT_ROW_COUNT],
            tutorial_state: 0,
            tutorial_active: true,
            autobuyers: AutobuyerState::new(),
            options: Options::new(),
        }
    }

    /// Returns how many dimension tiers are currently unlocked.
    /// Starts with 4, each dim boost beyond the first 4 doesn't unlock more.
    /// Dim boost 1 unlocks tier 5, boost 2 unlocks tier 6, etc. Capped at
    /// [`max_dimensions_unlockable`](Self::max_dimensions_unlockable) (6 under
    /// Normal Challenge 10, otherwise 8).
    pub fn unlocked_dimensions(&self) -> usize {
        let base = 4;
        let from_boosts = (self.dim_boosts as usize).min(4);
        (base + from_boosts).min(self.max_dimensions_unlockable())
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
        // Under Normal Challenge 10 only 6 dimensions exist (`tier < 7 ||
        // !NC10` in `isAvailableForPurchase`); otherwise all 8.
        if tier >= self.max_dimensions_unlockable()
            || tier > self.dim_boosts as usize + 3
        {
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
        let lookahead = self.dim_boosts > 0 && self.dim_boosts as usize + 2 >= tier;
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
