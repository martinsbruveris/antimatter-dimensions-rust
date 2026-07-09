use break_infinity::Decimal;

use crate::data::constants::{
    AD_AUTOBUYER_INTERVALS_MS, AD_AUTOBUYER_REQUIREMENTS, AUTOMATION_TAB_REQUIREMENT,
    BIG_CRUNCH_AUTOBUYER_INTERVAL_MS, DIM_BOOST_AUTOBUYER_INTERVAL_MS,
    GALAXY_AUTOBUYER_INTERVAL_MS, TICKSPEED_AUTOBUYER_INTERVAL_MS,
    TICKSPEED_AUTOBUYER_REQUIREMENT,
};
use crate::state::GameState;
use crate::tab_notifications::TabNotificationId;

/// Autobuyer purchase mode.
///
/// Maps onto the original's `AUTOBUYER_MODE`. For antimatter dimensions,
/// `BuyMax` corresponds to the early-game `BUY_10` mode (the UI shows it as
/// "Buys max"), which fills the current group of ten each tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AutobuyerMode {
    /// Buy a single unit each time the autobuyer fires ("Buys singles").
    BuySingle,
    /// Buy the maximum sensible amount each time ("Buys max").
    BuyMax,
}

/// Goal mode for the post-break Big Crunch autobuyer and the Eternity
/// autobuyer (the original's `AUTO_CRUNCH_MODE` / `AUTO_ETERNITY_MODE`:
/// 0 = amount, 1 = time, 2 = X times highest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PrestigeAutobuyerMode {
    /// Prestige once the pending gain reaches a fixed amount.
    #[default]
    Amount,
    /// Prestige after a fixed number of real-time seconds in the run.
    Time,
    /// Prestige once the pending gain reaches X times the previous highest.
    XHighest,
}

/// Trigger mode for the Reality autobuyer (`AUTO_REALITY_MODE`; the Effarig
/// `RELIC_SHARD` mode is celestial content and out of frontier — a save
/// carrying it loads as `Rm`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AutoRealityMode {
    /// Reality once the pending RM gain reaches the target.
    #[default]
    Rm,
    /// Reality once the pending glyph level reaches the target.
    Glyph,
    /// RM or glyph level.
    Either,
    /// RM and glyph level.
    Both,
    /// Reality after a fixed number of real-time seconds.
    Time,
}

/// The goal settings shared by the Big Crunch autobuyer (post-break) and the
/// Eternity autobuyer (the original stores these on `player.auto.bigCrunch` /
/// `player.auto.eternity` alongside the active flag).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PrestigeGoalSettings {
    pub mode: PrestigeAutobuyerMode,
    /// Fixed prestige-currency threshold (`amount`).
    pub amount: Decimal,
    /// "Dynamic amount": buying prestige-currency multipliers scales `amount`
    /// along (`increaseWithMult`).
    pub increase_with_mult: bool,
    /// Seconds between prestiges (`time`).
    pub time: f64,
    /// Multiplier on the previous highest gain (`xHighest`).
    pub x_highest: Decimal,
}

impl PrestigeGoalSettings {
    pub fn new() -> Self {
        Self {
            mode: PrestigeAutobuyerMode::Amount,
            amount: Decimal::ONE,
            increase_with_mult: true,
            time: 1.0,
            x_highest: Decimal::ONE,
        }
    }
}

impl Default for PrestigeGoalSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// The Eternity autobuyer (`player.auto.eternity`): no interval — its
/// condition is checked every tick. Unlocked by the `autobuyerEternity`
/// milestone (100 Eternities); non-`Amount` modes need Reality Upgrade 13.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EternityAutobuyer {
    /// Off by default (the original's `eternity.isActive: false`).
    pub is_active: bool,
    pub settings: PrestigeGoalSettings,
}

impl EternityAutobuyer {
    pub fn new() -> Self {
        Self {
            is_active: false,
            settings: PrestigeGoalSettings::new(),
        }
    }
}

impl Default for EternityAutobuyer {
    fn default() -> Self {
        Self::new()
    }
}

/// The Reality autobuyer (`player.auto.reality`): no interval. Unlocked by
/// Reality Upgrade 25.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RealityAutobuyer {
    /// Off by default.
    pub is_active: bool,
    pub mode: AutoRealityMode,
    /// Target Reality Machines (`rm`).
    pub rm: Decimal,
    /// Target glyph level (`glyph`; an integer in the original's input).
    pub glyph: u32,
    /// Target real-time seconds (`time`).
    pub time: f64,
}

impl RealityAutobuyer {
    pub fn new() -> Self {
        Self {
            is_active: false,
            mode: AutoRealityMode::Rm,
            rm: Decimal::ONE,
            glyph: 0,
            time: 0.0,
        }
    }
}

impl Default for RealityAutobuyer {
    fn default() -> Self {
        Self::new()
    }
}

/// The glyph level cap (`Glyphs.levelCap`, a constant pre-Ra) clamping the
/// Reality autobuyer's glyph-level target.
pub const GLYPH_LEVEL_CAP: u32 = 1_000_000;

/// A handle to any single autobuyer, for a uniform toggle/upgrade API across the
/// AD, Tickspeed (and later Dim Boost / Galaxy / Big Crunch) autobuyers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutobuyerTarget {
    /// One of the 8 Antimatter Dimension autobuyers (0-indexed tier).
    AdTier(usize),
    /// The Tickspeed autobuyer.
    Tickspeed,
    /// The Dimension Boost autobuyer (unlocked by NC10).
    DimBoost,
    /// The Antimatter Galaxy autobuyer (unlocked by NC11).
    Galaxy,
    /// The Big Crunch (Infinity) autobuyer (unlocked by NC12) — the one whose
    /// maxed interval gates Break Infinity.
    BigCrunch,
}

/// The original's `PRESTIGE_EVENT` ordinals, used by `resetTick` to decide which
/// autobuyer timers a reset clears (`prestigeEvent >= resetTickOn`).
const PRESTIGE_DIMENSION_BOOST: u8 = 0;
const PRESTIGE_ANTIMATTER_GALAXY: u8 = 1;
const PRESTIGE_INFINITY: u8 = 2;
const PRESTIGE_ETERNITY: u8 = 3;
const PRESTIGE_REALITY: u8 = 4;

/// The 100 ms floor an autobuyer's interval can be reduced to. Reaching it is
/// `hasMaxedInterval`; the Big Crunch autobuyer hitting it is what unlocks Break
/// Infinity (Feature 2.3).
pub const AUTOBUYER_MIN_INTERVAL_MS: f64 = 100.0;

/// Each interval upgrade multiplies the interval by this factor (floored at
/// [`AUTOBUYER_MIN_INTERVAL_MS`]) and doubles the IP cost.
const INTERVAL_UPGRADE_FACTOR: f64 = 0.6;

/// State for a single autobuyer.
///
/// Pre-Infinity, an autobuyer is unlocked either by antimatter (`is_bought`, once
/// the requirement is met — no antimatter is spent) or by completing its Normal
/// Challenge (`can_be_upgraded`, which additionally lets its `interval_ms` be
/// reduced with Infinity Points, down to the 100 ms floor). See the interval-
/// upgrade methods on [`GameState`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Autobuyer {
    /// Whether the slow version has been unlocked (JS `data.isBought`).
    pub is_bought: bool,
    /// Per-autobuyer on/off toggle (JS `data.isActive`). Defaults on.
    pub is_active: bool,
    /// Purchase mode (single or max).
    pub mode: AutobuyerMode,
    /// Interval between purchases in milliseconds. Starts at the tier's base and
    /// is reduced by interval upgrades toward [`AUTOBUYER_MIN_INTERVAL_MS`].
    pub interval_ms: f64,
    /// Infinity-Point cost of the next interval upgrade (JS `data.cost`, a plain
    /// number); starts at 1 and doubles per upgrade. Stays small (≤ 2^15 or so,
    /// since the interval floors after ~15 upgrades), so an `f64` matching the
    /// save's number form is exact.
    #[cfg_attr(feature = "serde", serde(default = "default_autobuyer_cost"))]
    pub cost: f64,
    /// "Buys max" bulk multiplier (`data.bulk`): how many groups of ten a single
    /// `BuyMax` fire completes. Only the AD autobuyers use it; it doubles per bulk
    /// upgrade up to [`AD_AUTOBUYER_BULK_CAP`]. Starts at 1.
    #[cfg_attr(feature = "serde", serde(default = "default_autobuyer_bulk"))]
    pub bulk: u32,
    /// Current timer tracking elapsed time since the last purchase.
    pub timer_ms: f64,
}

/// Dim Boost autobuyer limit config (`player.auto.dimBoost`): mirrors the
/// original's `limitDimBoosts` / `maxDimBoosts` / `limitUntilGalaxies` /
/// `galaxies` / `buyMaxInterval`. Gates the autobuyer and round-trips through the
/// save.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DimBoostAutobuyerConfig {
    /// Stop boosting once `dim_boosts >= max_dim_boosts` (`limitDimBoosts`).
    pub limit_dim_boosts: bool,
    /// The boost cap (`maxDimBoosts`); a plain number in the save.
    pub max_dim_boosts: f64,
    /// Only boost once `galaxies >= until_galaxies` (`limitUntilGalaxies`).
    pub limit_until_galaxies: bool,
    /// The galaxy threshold (`galaxies`).
    pub until_galaxies: f64,
    /// "Buys max" interval-suspension setting (`buyMaxInterval`), preserved.
    pub buy_max_interval: f64,
}

impl Default for DimBoostAutobuyerConfig {
    fn default() -> Self {
        Self {
            limit_dim_boosts: false,
            max_dim_boosts: 1.0,
            limit_until_galaxies: false,
            until_galaxies: 10.0,
            buy_max_interval: 0.0,
        }
    }
}

/// Antimatter Galaxy autobuyer limit config (`player.auto.galaxy`): mirrors
/// `limitGalaxies` / `maxGalaxies` / `buyMax` / `buyMaxInterval`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GalaxyAutobuyerConfig {
    /// Stop buying once `galaxies >= max_galaxies` (`limitGalaxies`).
    pub limit_galaxies: bool,
    /// The galaxy cap (`maxGalaxies`).
    pub max_galaxies: f64,
    /// "Buys max" toggle (`buyMax`); preserved (inert pre-Reality).
    pub buy_max: bool,
    /// "Buys max" interval-suspension setting (`buyMaxInterval`), preserved.
    pub buy_max_interval: f64,
}

impl Default for GalaxyAutobuyerConfig {
    fn default() -> Self {
        Self {
            limit_galaxies: false,
            max_galaxies: 1.0,
            buy_max: false,
            buy_max_interval: 0.0,
        }
    }
}

/// serde default for [`Autobuyer::cost`] (1 IP), since `f64`'s `Default` is 0.
#[cfg(feature = "serde")]
fn default_autobuyer_cost() -> f64 {
    1.0
}

/// serde default for [`Autobuyer::bulk`] (1 group), matching the game's default
/// and covering the tickspeed/prestige autobuyers, whose saves carry no `bulk`.
#[cfg(feature = "serde")]
fn default_autobuyer_bulk() -> u32 {
    1
}

/// The bulk-multiplier cap for the AD autobuyers (`bulkCap`): upgrades stop
/// doubling `bulk` here. Achievement 61 lifts the *effective* bulk to unlimited.
pub const AD_AUTOBUYER_BULK_CAP: u32 = 512;

/// The effective unlimited bulk granted by Achievement 61 (`1e100` in the
/// original, "to avoid issues with Infinity").
const AD_AUTOBUYER_UNLIMITED_BULK: f64 = 1e100;

impl Autobuyer {
    pub fn new(interval_ms: f64, mode: AutobuyerMode) -> Self {
        Self {
            is_bought: false,
            is_active: true,
            mode,
            interval_ms,
            cost: 1.0,
            bulk: 1,
            timer_ms: 0.0,
        }
    }

    /// Whether the interval is at its 100 ms floor (JS `hasMaxedInterval`).
    pub fn has_maxed_interval(&self) -> bool {
        self.interval_ms <= AUTOBUYER_MIN_INTERVAL_MS
    }

    /// Advance the timer, firing when the accumulated phase reaches
    /// `effective_interval_ms` (the stored interval after the `autobuyerSpeed`
    /// Break Infinity Upgrade's halving) *and* the autobuyer is `ready` — the
    /// caller-supplied form of `canTick` minus the interval test (active,
    /// unlocked, and the action-specific conditions like availability /
    /// affordability / requirement). See [`GameState::tick_autobuyers`].
    ///
    /// This mirrors the original `IntervaledAutobuyerState`, whose `canTick`
    /// compares `realTimePlayed - lastTick >= interval` using the `realTimePlayed`
    /// *before* the game loop advances it, and whose `tick()` sets
    /// `lastTick = realTimePlayed` — resetting the phase to 0 and discarding any
    /// overshoot. Real time always advances, so the phase accrues *every* tick
    /// (even when not `ready`); only a fire resets it. A fire is gated on `ready`
    /// because the original only calls `tick()` when `canTick` holds, so an
    /// autobuyer that is waiting to afford its purchase keeps its elapsed time
    /// rather than restarting each interval. We therefore test the carried phase
    /// *before* adding this tick's `dt`: a fresh timer (phase 0) does not fire on
    /// its first tick unless it already carried a full interval.
    ///
    /// `timer_ms` is the elapsed-time form of the original's
    /// `timeSinceLastTick` (`= realTimePlayed - lastTick`); the save codec
    /// converts between the two on load/store.
    fn advance(&mut self, dt_ms: f64, effective_interval_ms: f64, ready: bool) -> bool {
        let fired = ready && self.timer_ms >= effective_interval_ms;
        if fired {
            // `lastTick = realTimePlayed`: the phase resets to 0, dropping the
            // overshoot (the original does not carry the remainder forward).
            self.timer_ms = 0.0;
        }
        self.timer_ms += dt_ms;
        fired
    }
}

/// Collection of all autobuyer state.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AutobuyerState {
    /// Global toggle (JS `player.auto.autobuyersOn`): when false, no autobuyers
    /// fire. The strategy simulator also flips this off to drive its own buying.
    pub enabled: bool,
    /// Autobuyers for each of the 8 antimatter dimension tiers.
    pub dimensions: [Autobuyer; 8],
    /// Autobuyer for tickspeed upgrades. Pre-Infinity its mode is locked to
    /// `BuySingle` (the "Buys max" toggle requires completing a challenge).
    pub tickspeed: Autobuyer,
    /// Dimension Boost autobuyer (unlocked by completing NC10). No antimatter
    /// path — `is_bought` stays false; runs off `can_be_upgraded`.
    #[cfg_attr(feature = "serde", serde(default = "default_dim_boost_autobuyer"))]
    pub dim_boost: Autobuyer,
    /// Dim Boost autobuyer limit config (`limitDimBoosts` etc.).
    #[cfg_attr(feature = "serde", serde(default))]
    pub dim_boost_config: DimBoostAutobuyerConfig,
    /// Antimatter Galaxy autobuyer (unlocked by completing NC11).
    #[cfg_attr(feature = "serde", serde(default = "default_galaxy_autobuyer"))]
    pub galaxy: Autobuyer,
    /// Galaxy autobuyer limit config (`limitGalaxies` etc.).
    #[cfg_attr(feature = "serde", serde(default))]
    pub galaxy_config: GalaxyAutobuyerConfig,
    /// Big Crunch autobuyer (unlocked by completing NC12). Its maxed interval
    /// gates Break Infinity.
    #[cfg_attr(feature = "serde", serde(default = "default_big_crunch_autobuyer"))]
    pub big_crunch: Autobuyer,
    /// Post-break goal settings for the Big Crunch autobuyer (mode/amount/
    /// time/xHighest). Modes beyond `Amount` need the `bigCrunchModes`
    /// milestone (5 Eternities).
    #[cfg_attr(feature = "serde", serde(default))]
    pub big_crunch_settings: PrestigeGoalSettings,
    /// The Eternity autobuyer (100-Eternities milestone).
    #[cfg_attr(feature = "serde", serde(default))]
    pub eternity: EternityAutobuyer,
    /// The Reality autobuyer (Reality Upgrade 25).
    #[cfg_attr(feature = "serde", serde(default))]
    pub reality: RealityAutobuyer,
}

/// serde defaults for the three challenge-only autobuyers (so an older serialized
/// `AutobuyerState` still deserializes).
#[cfg(feature = "serde")]
fn default_dim_boost_autobuyer() -> Autobuyer {
    Autobuyer::new(DIM_BOOST_AUTOBUYER_INTERVAL_MS, AutobuyerMode::BuySingle)
}
#[cfg(feature = "serde")]
fn default_galaxy_autobuyer() -> Autobuyer {
    Autobuyer::new(GALAXY_AUTOBUYER_INTERVAL_MS, AutobuyerMode::BuySingle)
}
#[cfg(feature = "serde")]
fn default_big_crunch_autobuyer() -> Autobuyer {
    Autobuyer::new(BIG_CRUNCH_AUTOBUYER_INTERVAL_MS, AutobuyerMode::BuySingle)
}

impl AutobuyerState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            // AD autobuyers default to "Buys max" (BUY_10) per the original.
            dimensions: std::array::from_fn(|tier| {
                Autobuyer::new(AD_AUTOBUYER_INTERVALS_MS[tier], AutobuyerMode::BuyMax)
            }),
            tickspeed: Autobuyer::new(
                TICKSPEED_AUTOBUYER_INTERVAL_MS,
                AutobuyerMode::BuySingle,
            ),
            // The prestige autobuyers have a fixed action (no single/max mode).
            dim_boost: Autobuyer::new(
                DIM_BOOST_AUTOBUYER_INTERVAL_MS,
                AutobuyerMode::BuySingle,
            ),
            dim_boost_config: DimBoostAutobuyerConfig::default(),
            galaxy: Autobuyer::new(
                GALAXY_AUTOBUYER_INTERVAL_MS,
                AutobuyerMode::BuySingle,
            ),
            galaxy_config: GalaxyAutobuyerConfig::default(),
            big_crunch: Autobuyer::new(
                BIG_CRUNCH_AUTOBUYER_INTERVAL_MS,
                AutobuyerMode::BuySingle,
            ),
            big_crunch_settings: PrestigeGoalSettings::new(),
            eternity: EternityAutobuyer::new(),
            reality: RealityAutobuyer::new(),
        }
    }
}

impl Default for AutobuyerState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// Whether the Automation tab (containing the Autobuyers subtab) is
    /// unlocked: all-time total antimatter has reached 1e40.
    pub fn autobuyer_tab_unlocked(&self) -> bool {
        self.total_antimatter >= AUTOMATION_TAB_REQUIREMENT
    }

    /// Antimatter requirement to unlock the AD autobuyer for `tier` (0-indexed).
    pub fn ad_autobuyer_requirement(tier: usize) -> Decimal {
        AD_AUTOBUYER_REQUIREMENTS[tier]
    }

    /// Whether the requirement to unlock the AD autobuyer for `tier` is met.
    pub fn can_unlock_ad_autobuyer(&self, tier: usize) -> bool {
        self.total_antimatter >= AD_AUTOBUYER_REQUIREMENTS[tier]
    }

    /// Unlock the AD autobuyer for `tier`. Costs no antimatter; only succeeds
    /// once the requirement is met. Returns true if it became unlocked.
    pub fn unlock_ad_autobuyer(&mut self, tier: usize) -> bool {
        if tier >= 8 || !self.can_unlock_ad_autobuyer(tier) {
            return false;
        }
        self.autobuyers.dimensions[tier].is_bought = true;
        // Buying the unlock acknowledges the "new autobuyer" badge (mirrors
        // AntimatterDimensionAutobuyerState.purchase); the per-tick check
        // re-badges if another unlock is already affordable.
        self.clear_tab_notification_trigger(TabNotificationId::NewAutobuyer);
        true
    }

    /// Toggle the AD autobuyer for `tier` on/off (its `is_active` flag).
    pub fn toggle_ad_autobuyer(&mut self, tier: usize) {
        if tier < 8 {
            let ab = &mut self.autobuyers.dimensions[tier];
            ab.is_active = !ab.is_active;
        }
    }

    /// The effective "Buys max" bulk for the AD autobuyer of `tier` (`this.bulk`):
    /// the stored `bulk` clamped to [`AD_AUTOBUYER_BULK_CAP`], or unlimited once
    /// Achievement 61 (`hasUnlimitedBulk`) is earned.
    pub fn ad_autobuyer_effective_bulk(&self, tier: usize) -> f64 {
        if self.achievement_unlocked(61) {
            AD_AUTOBUYER_UNLIMITED_BULK
        } else {
            self.autobuyers.dimensions[tier]
                .bulk
                .min(AD_AUTOBUYER_BULK_CAP) as f64
        }
    }

    /// Toggle the AD autobuyer for `tier` between "Buys singles" and "Buys max".
    pub fn toggle_ad_autobuyer_mode(&mut self, tier: usize) {
        if tier < 8 {
            let ab = &mut self.autobuyers.dimensions[tier];
            ab.mode = match ab.mode {
                AutobuyerMode::BuySingle => AutobuyerMode::BuyMax,
                AutobuyerMode::BuyMax => AutobuyerMode::BuySingle,
            };
        }
    }

    /// Antimatter requirement to unlock the tickspeed autobuyer.
    pub fn tickspeed_autobuyer_requirement() -> Decimal {
        TICKSPEED_AUTOBUYER_REQUIREMENT
    }

    /// Whether the requirement to unlock the tickspeed autobuyer is met.
    pub fn can_unlock_tickspeed_autobuyer(&self) -> bool {
        self.total_antimatter >= TICKSPEED_AUTOBUYER_REQUIREMENT
    }

    /// Unlock the tickspeed autobuyer (no antimatter cost). Returns true if it
    /// became unlocked.
    pub fn unlock_tickspeed_autobuyer(&mut self) -> bool {
        if !self.can_unlock_tickspeed_autobuyer() {
            return false;
        }
        self.autobuyers.tickspeed.is_bought = true;
        // See unlock_ad_autobuyer (mirrors TickspeedAutobuyerState.purchase).
        self.clear_tab_notification_trigger(TabNotificationId::NewAutobuyer);
        true
    }

    /// Toggle the tickspeed autobuyer on/off.
    pub fn toggle_tickspeed_autobuyer(&mut self) {
        self.autobuyers.tickspeed.is_active = !self.autobuyers.tickspeed.is_active;
    }

    /// Toggle the global autobuyers on/off switch (the hotkey/checkbox).
    pub fn toggle_autobuyers(&mut self) {
        self.autobuyers.enabled = !self.autobuyers.enabled;
    }

    /// Set the `is_active` flag on every *unlocked* autobuyer (the "Enable/
    /// Disable all autobuyers" button — JS only affects `Autobuyers.unlocked`).
    pub fn set_all_autobuyers_active(&mut self, active: bool) {
        for ab in self.autobuyers.dimensions.iter_mut() {
            if ab.is_bought {
                ab.is_active = active;
            }
        }
        if self.autobuyers.tickspeed.is_bought {
            self.autobuyers.tickspeed.is_active = active;
        }
    }

    /// The [`Autobuyer`] addressed by `target`.
    pub fn autobuyer(&self, target: AutobuyerTarget) -> &Autobuyer {
        match target {
            AutobuyerTarget::AdTier(tier) => &self.autobuyers.dimensions[tier],
            AutobuyerTarget::Tickspeed => &self.autobuyers.tickspeed,
            AutobuyerTarget::DimBoost => &self.autobuyers.dim_boost,
            AutobuyerTarget::Galaxy => &self.autobuyers.galaxy,
            AutobuyerTarget::BigCrunch => &self.autobuyers.big_crunch,
        }
    }

    fn autobuyer_mut(&mut self, target: AutobuyerTarget) -> &mut Autobuyer {
        match target {
            AutobuyerTarget::AdTier(tier) => &mut self.autobuyers.dimensions[tier],
            AutobuyerTarget::Tickspeed => &mut self.autobuyers.tickspeed,
            AutobuyerTarget::DimBoost => &mut self.autobuyers.dim_boost,
            AutobuyerTarget::Galaxy => &mut self.autobuyers.galaxy,
            AutobuyerTarget::BigCrunch => &mut self.autobuyers.big_crunch,
        }
    }

    /// The Normal Challenge whose completion makes `target` interval-upgradeable
    /// (`canBeUpgraded`): AD tier `n` → NC`n`, Tickspeed → NC9, Dim Boost → NC10,
    /// Galaxy → NC11, Big Crunch → NC12.
    pub fn autobuyer_challenge(target: AutobuyerTarget) -> u8 {
        match target {
            AutobuyerTarget::AdTier(tier) => tier as u8 + 1,
            AutobuyerTarget::Tickspeed => 9,
            AutobuyerTarget::DimBoost => 10,
            AutobuyerTarget::Galaxy => 11,
            AutobuyerTarget::BigCrunch => 12,
        }
    }

    /// Toggle the `is_active` flag of `target` (used by the prestige autobuyers'
    /// on/off checkbox; the AD/Tickspeed ones have their own tier-keyed toggles).
    pub fn toggle_autobuyer_active(&mut self, target: AutobuyerTarget) {
        let ab = self.autobuyer_mut(target);
        ab.is_active = !ab.is_active;
    }

    /// Whether `target`'s interval can be reduced with Infinity Points — i.e. its
    /// Normal Challenge is completed (JS `canBeUpgraded`).
    pub fn autobuyer_can_be_upgraded(&self, target: AutobuyerTarget) -> bool {
        self.challenge_completed(Self::autobuyer_challenge(target))
    }

    /// Whether `target` runs at all (JS `isUnlocked`): unlocked by antimatter
    /// (`is_bought`) or by completing its challenge (`can_be_upgraded`).
    pub fn autobuyer_is_unlocked(&self, target: AutobuyerTarget) -> bool {
        self.autobuyer(target).is_bought || self.autobuyer_can_be_upgraded(target)
    }

    /// Whether `target`'s interval is already at the 100 ms floor.
    pub fn autobuyer_has_maxed_interval(&self, target: AutobuyerTarget) -> bool {
        self.autobuyer(target).has_maxed_interval()
    }

    /// Reduce `target`'s interval one step, spending its Infinity-Point cost.
    /// Requires the autobuyer to be challenge-upgradeable, not already at the
    /// floor, and affordable. Mirrors `upgradeInterval`: `cost ×2`,
    /// `interval = max(interval × 0.6, 100)`. Returns whether it happened.
    ///
    /// (Achievements 52/53 fire here in the original but have their own unlock
    /// conditions; they are wired with the achievements integration, not forced.)
    pub fn upgrade_autobuyer_interval(&mut self, target: AutobuyerTarget) -> bool {
        if !self.autobuyer_can_be_upgraded(target)
            || self.autobuyer_has_maxed_interval(target)
        {
            return false;
        }
        let cost = Decimal::from_float(self.autobuyer(target).cost);
        if self.infinity_points < cost {
            return false;
        }
        self.infinity_points -= cost;
        let ab = self.autobuyer_mut(target);
        ab.cost *= 2.0;
        ab.interval_ms =
            (ab.interval_ms * INTERVAL_UPGRADE_FACTOR).max(AUTOBUYER_MIN_INTERVAL_MS);
        // Maxing the Big Crunch interval unlocks Break Infinity: badge its tab
        // (mirrors BigCrunchAutobuyerState.upgradeInterval; the trigger's own
        // condition checks the interval actually reached the floor).
        if target == AutobuyerTarget::BigCrunch {
            self.try_trigger_tab_notification(TabNotificationId::BreakInfinity);
        }
        true
    }

    /// Advance all autobuyers by `dt_ms` and execute any triggered purchases.
    /// Does nothing if autobuyers are globally disabled.
    pub fn tick_autobuyers(&mut self, dt_ms: f64) {
        if !self.autobuyers.enabled {
            // Globally-off autobuyers never fire, but the original's
            // `timeSinceLastTick = realTimePlayed - lastTick` keeps growing with
            // real time regardless (it is derived, not stored). Our timers are the
            // elapsed-time form, so accrue every one to keep the stored `lastTick`
            // (`realTimePlayed - timer_ms`) fixed — otherwise it drifts one tick per
            // frame while disabled.
            for ab in &mut self.autobuyers.dimensions {
                ab.timer_ms += dt_ms;
            }
            self.autobuyers.tickspeed.timer_ms += dt_ms;
            self.autobuyers.dim_boost.timer_ms += dt_ms;
            self.autobuyers.galaxy.timer_ms += dt_ms;
            self.autobuyers.big_crunch.timer_ms += dt_ms;
            return;
        }

        // The `autobuyerSpeed` Break Infinity Upgrade halves every autobuyer's
        // effective interval.
        let speedup = self.break_infinity_autobuyer_speedup();

        // Each interval autobuyer's `ready` is its `canTick` minus the interval
        // test: active + unlocked + the action-specific readiness the original
        // gates on (so the phase keeps accruing while it waits to afford — it does
        // not restart each interval). `advance` resets the phase only on a fire.

        // Antimatter dimension autobuyers: `isAvailableForPurchase &&
        // isAffordable` (the single-cost affordability, even in "Buys max").
        for tier in 0..8 {
            let ready = self.autobuyers.dimensions[tier].is_active
                && self.autobuyer_is_unlocked(AutobuyerTarget::AdTier(tier))
                && self.dim_available_for_purchase(tier)
                && self.dim_single_affordable(tier);
            let eff = self.autobuyers.dimensions[tier].interval_ms * speedup;
            let mode = self.autobuyers.dimensions[tier].mode;
            if self.autobuyers.dimensions[tier].advance(dt_ms, eff, ready) {
                match mode {
                    AutobuyerMode::BuySingle => {
                        self.buy_dimension(tier);
                    }
                    // BUY_10: complete up to `bulk` groups of ten, but only when a
                    // whole group is affordable (`buyMaxDimension` bails on
                    // `!isAffordableUntil10`) — never a partial group.
                    AutobuyerMode::BuyMax => {
                        let bulk = self.ad_autobuyer_effective_bulk(tier);
                        self.buy_max_dimension_bulk(tier, bulk);
                    }
                }
            }
        }

        // Tickspeed autobuyer: `isAvailableForPurchase && isAffordable`.
        let ready = self.autobuyers.tickspeed.is_active
            && self.autobuyer_is_unlocked(AutobuyerTarget::Tickspeed)
            && self.tickspeed_available()
            && self.tickspeed_affordable();
        let eff = self.autobuyers.tickspeed.interval_ms * speedup;
        let mode = self.autobuyers.tickspeed.mode;
        if self.autobuyers.tickspeed.advance(dt_ms, eff, ready) {
            match mode {
                AutobuyerMode::BuySingle => {
                    self.buy_tickspeed();
                }
                AutobuyerMode::BuyMax => {
                    self.buy_max_tickspeed();
                }
            }
        }

        // Prestige autobuyers (unlocked by completing NC10/11/12). Their readiness
        // is exactly the buy/reset condition, so the phase resets only when the
        // action can actually happen (matching the original's `canTick`).
        // Dim Boost limit gate (`DimBoostAutobuyerState.tick`, non-buyMax path):
        // boost only when under the boost cap, or once the wait-for-galaxies
        // threshold is met. `isBuyMaxUnlocked` is a post-Reality perk we don't
        // model, so only this branch applies.
        let db_cfg = self.autobuyers.dim_boost_config.clone();
        let limit_condition =
            !db_cfg.limit_dim_boosts || (self.dim_boosts as f64) < db_cfg.max_dim_boosts;
        let galaxy_condition = db_cfg.limit_until_galaxies
            && (self.galaxies as f64) >= db_cfg.until_galaxies;
        let ready = self.autobuyers.dim_boost.is_active
            && self.autobuyer_is_unlocked(AutobuyerTarget::DimBoost)
            && self.can_dim_boost()
            && (limit_condition || galaxy_condition);
        let eff = self.autobuyers.dim_boost.interval_ms * speedup;
        if self.autobuyers.dim_boost.advance(dt_ms, eff, ready) && self.buy_dim_boost() {
            self.reset_autobuyer_ticks(PRESTIGE_DIMENSION_BOOST, dt_ms);
        }

        // Galaxy limit gate (`GalaxyAutobuyerState.tick`: the cap passed to
        // `requestGalaxyReset` stops it at `maxGalaxies`).
        let galaxy_limit_ok = !self.autobuyers.galaxy_config.limit_galaxies
            || (self.galaxies as f64) < self.autobuyers.galaxy_config.max_galaxies;
        let ready = self.autobuyers.galaxy.is_active
            && self.autobuyer_is_unlocked(AutobuyerTarget::Galaxy)
            && self.can_buy_galaxy()
            && galaxy_limit_ok;
        let eff = self.autobuyers.galaxy.interval_ms * speedup;
        if self.autobuyers.galaxy.advance(dt_ms, eff, ready) && self.buy_galaxy() {
            self.reset_autobuyer_ticks(PRESTIGE_ANTIMATTER_GALAXY, dt_ms);
        }

        // Big Crunch: `canTick` is `Player.canCrunch` (at the goal), so the phase
        // resets whenever a crunch is possible; the crunch *itself* additionally
        // needs the configured goal mode (`willInfinity`, always true pre-break).
        let ready = self.autobuyers.big_crunch.is_active
            && self.autobuyer_is_unlocked(AutobuyerTarget::BigCrunch)
            && self.can_big_crunch();
        let eff = self.autobuyers.big_crunch.interval_ms * speedup;
        if self.autobuyers.big_crunch.advance(dt_ms, eff, ready)
            && self.will_auto_crunch()
            && self.big_crunch()
        {
            self.reset_autobuyer_ticks(PRESTIGE_INFINITY, dt_ms);
        }

        // Eternity and Reality autobuyers: no interval — their conditions are
        // checked every tick (plain `AutobuyerState`s in the original). The
        // prestige calls gate themselves on availability.
        if self.eternity_autobuyer_unlocked()
            && self.autobuyers.eternity.is_active
            && self.will_auto_eternity()
            && self.eternity()
        {
            self.reset_autobuyer_ticks(PRESTIGE_ETERNITY, dt_ms);
        }

        if self.reality_autobuyer_unlocked()
            && self.autobuyers.reality.is_active
            && self.will_auto_reality()
            && self.auto_reality()
        {
            self.reset_autobuyer_ticks(PRESTIGE_REALITY, dt_ms);
        }
    }

    /// `Autobuyers.resetTick(prestigeEvent)`: on a prestige reset the original sets
    /// each qualifying autobuyer's `lastTick` to 0 (so its whole run counts as
    /// elapsed and it can fire immediately). We store the timer as elapsed time and
    /// re-derive `lastTick = realTimePlayed - timer_ms` on save; `realTimePlayed` is
    /// incremented *after* the autobuyer pass this tick, so target the post-tick
    /// value (`+ dt_ms`) to keep the derived `lastTick` exactly 0. Pre-Reality
    /// `resetTickOn`: AD / Tickspeed = `DIMENSION_BOOST` (0), Dim Boost =
    /// `ANTIMATTER_GALAXY` (1), Galaxy = `INFINITY` (2), Big Crunch = `ETERNITY` (3).
    fn reset_autobuyer_ticks(&mut self, event: u8, dt_ms: f64) {
        let rt = self.records.real_time_played_ms + dt_ms;
        for ab in &mut self.autobuyers.dimensions {
            ab.timer_ms = rt;
        }
        self.autobuyers.tickspeed.timer_ms = rt;
        if event >= PRESTIGE_ANTIMATTER_GALAXY {
            self.autobuyers.dim_boost.timer_ms = rt;
        }
        if event >= PRESTIGE_INFINITY {
            self.autobuyers.galaxy.timer_ms = rt;
        }
        if event >= PRESTIGE_ETERNITY {
            self.autobuyers.big_crunch.timer_ms = rt;
        }
    }

    /// Whether Break Infinity (Feature 2.3) is unlockable: the Big Crunch
    /// autobuyer's interval is at its 100 ms floor
    /// (`BreakInfinityButton.isUnlocked = Autobuyer.bigCrunch.hasMaxedInterval`).
    /// Reaching the floor requires the interval to have been upgraded, which needs
    /// NC12 completed, so no separate unlock check is necessary.
    pub fn break_infinity_unlockable(&self) -> bool {
        self.autobuyer_has_maxed_interval(AutobuyerTarget::BigCrunch)
    }

    // --- Prestige autobuyer goal modes (Big Crunch / Eternity / Reality) ---------

    /// Whether the Big Crunch autobuyer offers the Time / X-highest modes
    /// (`hasAdditionalModes`): the `bigCrunchModes` milestone, 5 Eternities.
    pub fn big_crunch_autobuyer_has_modes(&self) -> bool {
        self.eternity_milestone_reached(5)
    }

    /// Whether the Eternity autobuyer runs at all (`Autobuyer.eternity
    /// .isUnlocked`): the `autobuyerEternity` milestone, 100 Eternities.
    pub fn eternity_autobuyer_unlocked(&self) -> bool {
        self.eternity_milestone_reached(100)
    }

    /// Whether the Eternity autobuyer offers the Time / X-highest modes:
    /// Reality Upgrade 13.
    pub fn eternity_autobuyer_has_modes(&self) -> bool {
        self.reality_upgrade_bought(13)
    }

    /// Whether the Reality autobuyer runs at all: Reality Upgrade 25.
    pub fn reality_autobuyer_unlocked(&self) -> bool {
        self.reality_upgrade_bought(25)
    }

    /// `BigCrunchAutobuyerState.willInfinity`: pre-break (or inside an
    /// antimatter challenge) the autobuyer always crunches at the goal;
    /// post-break the configured mode decides.
    pub(crate) fn will_auto_crunch(&self) -> bool {
        let in_antimatter_challenge =
            self.challenge.current != 0 || self.infinity_challenge.current != 0;
        if !self.broke_infinity || in_antimatter_challenge {
            return true;
        }
        let s = &self.autobuyers.big_crunch_settings;
        match s.mode {
            PrestigeAutobuyerMode::Amount => self.gained_infinity_points() >= s.amount,
            PrestigeAutobuyerMode::Time => {
                self.records.this_infinity.real_time_ms / 1000.0 > s.time
            }
            // `highestPrevPrestige` for a crunch is this eternity's IP peak.
            PrestigeAutobuyerMode::XHighest => {
                self.gained_infinity_points()
                    >= self.records.this_eternity.max_ip * s.x_highest
            }
        }
    }

    /// `EternityAutobuyerState.willEternity`. Inside an Eternity Challenge the
    /// autobuyer eternities as soon as no further completions are reachable
    /// (without the ECB perk that is "as soon as the EC can be completed",
    /// since `eternity()` itself gates on the goal).
    pub(crate) fn will_auto_eternity(&self) -> bool {
        if self.any_ec_running() {
            if !self.perk_bought(73) {
                return true;
            }
            let id = self.eternity_challenge_current;
            return self.ec_pending_total_completions(id)
                >= crate::eternity_challenges::EC_MAX_COMPLETIONS;
        }
        let s = &self.autobuyers.eternity.settings;
        match s.mode {
            PrestigeAutobuyerMode::Amount => self.gained_eternity_points() >= s.amount,
            PrestigeAutobuyerMode::Time => {
                self.records.this_eternity.real_time_ms / 1000.0 > s.time
            }
            // `highestPrevPrestige` for an eternity is this reality's EP peak.
            PrestigeAutobuyerMode::XHighest => {
                self.gained_eternity_points()
                    >= self.records.this_reality.max_ep * s.x_highest
            }
        }
    }

    /// The Reality autobuyer's trigger (`RealityAutobuyerState.tick`, minus
    /// the Effarig glyph-filter branch — celestial content). The amplification
    /// factor is 1 at our frontier.
    pub(crate) fn will_auto_reality(&self) -> bool {
        if !self.is_reality_available() {
            return false;
        }
        let ab = &self.autobuyers.reality;
        let rm_proc = || self.gained_reality_machines() >= ab.rm;
        let glyph_proc =
            || self.gained_glyph_level().actual_level >= ab.glyph.min(GLYPH_LEVEL_CAP);
        match ab.mode {
            AutoRealityMode::Rm => rm_proc(),
            AutoRealityMode::Glyph => glyph_proc(),
            AutoRealityMode::Either => rm_proc() || glyph_proc(),
            AutoRealityMode::Both => rm_proc() && glyph_proc(),
            AutoRealityMode::Time => {
                self.records.this_reality.real_time_ms / 1000.0 > ab.time
            }
        }
    }

    /// Toggle the Eternity autobuyer's active flag.
    pub fn toggle_eternity_autobuyer(&mut self) {
        self.autobuyers.eternity.is_active = !self.autobuyers.eternity.is_active;
    }

    /// Toggle the Reality autobuyer's active flag.
    pub fn toggle_reality_autobuyer(&mut self) {
        self.autobuyers.reality.is_active = !self.autobuyers.reality.is_active;
    }

    /// Set the Big Crunch autobuyer's goal mode. Non-`Amount` modes require
    /// the `bigCrunchModes` milestone.
    pub fn set_big_crunch_autobuyer_mode(
        &mut self,
        mode: PrestigeAutobuyerMode,
    ) -> bool {
        if mode != PrestigeAutobuyerMode::Amount
            && !self.big_crunch_autobuyer_has_modes()
        {
            return false;
        }
        self.autobuyers.big_crunch_settings.mode = mode;
        true
    }

    /// Set the Eternity autobuyer's goal mode. Non-`Amount` modes require
    /// Reality Upgrade 13.
    pub fn set_eternity_autobuyer_mode(&mut self, mode: PrestigeAutobuyerMode) -> bool {
        if mode != PrestigeAutobuyerMode::Amount && !self.eternity_autobuyer_has_modes()
        {
            return false;
        }
        self.autobuyers.eternity.settings.mode = mode;
        true
    }

    /// Set the Reality autobuyer's trigger mode.
    pub fn set_reality_autobuyer_mode(&mut self, mode: AutoRealityMode) {
        self.autobuyers.reality.mode = mode;
    }

    /// Toggle the Big Crunch autobuyer's "Dynamic amount" checkbox.
    pub fn toggle_big_crunch_dynamic_amount(&mut self) {
        let s = &mut self.autobuyers.big_crunch_settings;
        s.increase_with_mult = !s.increase_with_mult;
    }

    /// Toggle the Eternity autobuyer's "Dynamic amount" checkbox.
    pub fn toggle_eternity_dynamic_amount(&mut self) {
        let s = &mut self.autobuyers.eternity.settings;
        s.increase_with_mult = !s.increase_with_mult;
    }

    /// Set the value for the Big Crunch autobuyer's *current* mode (the single
    /// input box under the mode selector).
    pub fn set_big_crunch_autobuyer_value(&mut self, value: Decimal) {
        let s = &mut self.autobuyers.big_crunch_settings;
        match s.mode {
            PrestigeAutobuyerMode::Amount => s.amount = value,
            PrestigeAutobuyerMode::Time => s.time = value.to_f64(),
            PrestigeAutobuyerMode::XHighest => s.x_highest = value,
        }
    }

    /// Set the value for the Eternity autobuyer's current mode.
    pub fn set_eternity_autobuyer_value(&mut self, value: Decimal) {
        let s = &mut self.autobuyers.eternity.settings;
        match s.mode {
            PrestigeAutobuyerMode::Amount => s.amount = value,
            PrestigeAutobuyerMode::Time => s.time = value.to_f64(),
            PrestigeAutobuyerMode::XHighest => s.x_highest = value,
        }
    }

    /// Set the Reality autobuyer's RM target.
    pub fn set_reality_autobuyer_rm(&mut self, rm: Decimal) {
        self.autobuyers.reality.rm = rm;
    }

    /// Set the Reality autobuyer's glyph-level target.
    pub fn set_reality_autobuyer_glyph(&mut self, glyph: u32) {
        self.autobuyers.reality.glyph = glyph;
    }

    /// Set the Reality autobuyer's time target (seconds).
    pub fn set_reality_autobuyer_time(&mut self, time: f64) {
        self.autobuyers.reality.time = time;
    }

    /// "Dynamic amount" (`bumpAmount`): prestige-currency multiplier purchases
    /// scale the fixed-amount goal along. The Big Crunch bump fires from
    /// Achievements 85/93 (×4 IP) — the `ipMult` rebuyable is a deferred
    /// Break-Infinity feature; the Eternity bump fires from the `epMult`
    /// Eternity Upgrade (×5 each).
    pub(crate) fn bump_big_crunch_amount(&mut self, mult: Decimal) {
        if self.autobuyer_is_unlocked(AutobuyerTarget::BigCrunch)
            && self.autobuyers.big_crunch_settings.increase_with_mult
        {
            let amount = self.autobuyers.big_crunch_settings.amount * mult;
            self.autobuyers.big_crunch_settings.amount = amount;
        }
    }

    /// See [`Self::bump_big_crunch_amount`].
    pub(crate) fn bump_eternity_amount(&mut self, mult: Decimal) {
        if self.eternity_autobuyer_unlocked()
            && self.autobuyers.eternity.settings.increase_with_mult
        {
            let amount = self.autobuyers.eternity.settings.amount * mult;
            self.autobuyers.eternity.settings.amount = amount;
        }
    }

    /// The prestige autobuyers' config half of `Autobuyers.reset()` (runs on
    /// every Eternity and Reality reset, *after* rewards/eternities settle):
    /// the Big Crunch mode reverts to `Amount` without the `bigCrunchModes`
    /// milestone, and the Eternity autobuyer deactivates without its
    /// milestone. The Reality autobuyer has no reset (RU25 persists).
    pub(crate) fn reset_prestige_autobuyer_configs(&mut self) {
        if !self.big_crunch_autobuyer_has_modes() {
            self.autobuyers.big_crunch_settings.mode = PrestigeAutobuyerMode::Amount;
        }
        if !self.eternity_autobuyer_unlocked() {
            self.autobuyers.eternity.is_active = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::constants::BIG_CRUNCH_THRESHOLD;

    /// Complete NC1 (the tutorial) via the first Big Crunch, so the 1st AD
    /// autobuyer becomes interval-upgradeable.
    fn game_with_nc1_done() -> GameState {
        let mut game = GameState::new();
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        game.big_crunch();
        assert!(game.challenge_completed(1));
        game
    }

    #[test]
    fn effective_bulk_clamps_to_cap_and_unlimited_via_ach61() {
        let mut game = GameState::new();
        // Default bulk is 1.
        assert_eq!(game.ad_autobuyer_effective_bulk(0), 1.0);

        // Clamped to the 512 cap.
        game.autobuyers.dimensions[0].bulk = 4096;
        assert_eq!(game.ad_autobuyer_effective_bulk(0), 512.0);

        // Achievement 61 lifts the effective bulk to unlimited (1e100).
        game.unlock_achievement(61);
        assert_eq!(game.ad_autobuyer_effective_bulk(0), 1e100);
    }

    #[test]
    fn buy_max_autobuyer_uses_bulk_multiplier() {
        // The AD1 autobuyer in "Buys max" completes `bulk` groups of ten per fire.
        let mut game = GameState::new();
        game.antimatter = Decimal::from_float(1e12);
        game.autobuyers.dimensions[0].is_bought = true;
        game.autobuyers.dimensions[0].is_active = true;
        game.autobuyers.dimensions[0].mode = AutobuyerMode::BuyMax;
        game.autobuyers.dimensions[0].bulk = 3;
        game.autobuyers.dimensions[0].interval_ms = 100.0;
        // Arm the timer: a full interval of phase has accumulated, so it fires
        // this tick (a fresh timer would wait an interval first).
        game.autobuyers.dimensions[0].timer_ms = 100.0;

        game.tick_autobuyers(150.0);
        // Three groups of ten in one fire.
        assert_eq!(game.dimensions[0].bought, 30);
    }

    #[test]
    fn interval_upgrade_requires_challenge_completion() {
        let mut game = GameState::new();
        let target = AutobuyerTarget::AdTier(0);
        game.infinity_points = Decimal::from_float(100.0);

        // NC1 not completed → not upgradeable even with IP.
        assert!(!game.autobuyer_can_be_upgraded(target));
        assert!(!game.upgrade_autobuyer_interval(target));
        assert_eq!(game.autobuyer(target).interval_ms, 500.0);

        // After completing NC1 the interval can be reduced: ×0.6, cost ×2.
        let mut game = game_with_nc1_done();
        game.infinity_points = Decimal::from_float(100.0);
        assert!(game.autobuyer_can_be_upgraded(target));
        assert!(game.upgrade_autobuyer_interval(target));
        assert!((game.autobuyer(target).interval_ms - 300.0).abs() < 1e-9);
        assert_eq!(game.autobuyer(target).cost, 2.0);
        assert_eq!(game.infinity_points, Decimal::from_float(99.0));
    }

    #[test]
    fn interval_upgrade_floors_at_100ms() {
        let target = AutobuyerTarget::AdTier(0);
        let mut game = game_with_nc1_done();
        game.infinity_points = Decimal::from_float(1e9);

        for _ in 0..50 {
            game.upgrade_autobuyer_interval(target);
        }
        assert!(game.autobuyer_has_maxed_interval(target));
        assert_eq!(
            game.autobuyer(target).interval_ms,
            AUTOBUYER_MIN_INTERVAL_MS
        );

        // Further upgrades are no-ops (no IP spent).
        let ip = game.infinity_points;
        assert!(!game.upgrade_autobuyer_interval(target));
        assert_eq!(game.infinity_points, ip);
    }

    #[test]
    fn interval_upgrade_needs_infinity_points() {
        let target = AutobuyerTarget::AdTier(0);
        let mut game = game_with_nc1_done();
        // The crunch awarded exactly 1 IP; the first upgrade (cost 1) spends it.
        assert_eq!(game.infinity_points, Decimal::ONE);
        assert!(game.upgrade_autobuyer_interval(target));
        assert_eq!(game.infinity_points, Decimal::ZERO);
        // The next upgrade costs 2 but there is no IP left.
        assert!(!game.upgrade_autobuyer_interval(target));
    }

    #[test]
    fn autobuyer_unlocked_by_antimatter_or_challenge() {
        let mut game = GameState::new();
        let target = AutobuyerTarget::AdTier(2); // 3rd AD → NC3
        assert!(!game.autobuyer_is_unlocked(target));

        // Antimatter unlock alone runs the autobuyer but does not allow upgrades.
        game.autobuyers.dimensions[2].is_bought = true;
        assert!(game.autobuyer_is_unlocked(target));
        assert!(!game.autobuyer_can_be_upgraded(target));

        // Challenge completion alone also unlocks it, and enables upgrades.
        game.autobuyers.dimensions[2].is_bought = false;
        game.complete_challenge(3);
        assert!(game.autobuyer_is_unlocked(target));
        assert!(game.autobuyer_can_be_upgraded(target));
    }

    #[test]
    fn prestige_autobuyers_unlock_by_challenge_completion() {
        let mut game = GameState::new();
        for t in [
            AutobuyerTarget::DimBoost,
            AutobuyerTarget::Galaxy,
            AutobuyerTarget::BigCrunch,
        ] {
            assert!(!game.autobuyer_is_unlocked(t));
        }

        game.complete_challenge(10);
        game.complete_challenge(11);
        game.complete_challenge(12);

        assert!(game.autobuyer_is_unlocked(AutobuyerTarget::DimBoost));
        assert!(game.autobuyer_is_unlocked(AutobuyerTarget::Galaxy));
        assert!(game.autobuyer_is_unlocked(AutobuyerTarget::BigCrunch));
    }

    #[test]
    fn dim_boost_autobuyer_boosts_when_possible() {
        let mut game = GameState::new();
        game.complete_challenge(10); // unlock the Dim Boost autobuyer
        game.autobuyers.dim_boost.interval_ms = 100.0;
        // Arm the timer so it fires this tick (see the AD-autobuyer test above).
        game.autobuyers.dim_boost.timer_ms = 100.0;
        // A satisfiable boost: 20 of the 4th dimension.
        game.dimensions[3].amount = Decimal::from_float(20.0);
        assert!(game.can_dim_boost());

        game.tick_autobuyers(150.0);
        assert_eq!(game.dim_boosts, 1);
    }

    #[test]
    fn big_crunch_autobuyer_crunches_when_unlocked_and_at_goal() {
        let mut game = GameState::new();
        game.complete_challenge(12); // unlock the Big Crunch autobuyer
        game.autobuyers.big_crunch.interval_ms = 100.0;
        // Arm the timer so it fires this tick (see the AD-autobuyer test above).
        game.autobuyers.big_crunch.timer_ms = 100.0;
        game.antimatter = BIG_CRUNCH_THRESHOLD; // at the goal
        let inf_before = game.infinities;

        game.tick_autobuyers(150.0);

        assert!(game.infinities > inf_before);
        assert!(game.antimatter < BIG_CRUNCH_THRESHOLD); // reset by the crunch
    }

    /// A post-break state where a crunch is available and gains a known IP.
    fn game_post_break_at_goal() -> GameState {
        let mut game = GameState::new();
        game.complete_challenge(12);
        game.autobuyers.big_crunch.interval_ms = 100.0;
        game.infinity_unlocked = true;
        game.broke_infinity = true;
        game.antimatter = Decimal::new(1.0, 400);
        game.records.this_infinity.max_am = game.antimatter;
        game
    }

    #[test]
    fn crunch_amount_mode_waits_for_threshold() {
        let mut game = game_post_break_at_goal();
        assert!(game.eternity_milestone_reached(0)); // sanity: 0 eternities
        game.eternities = Decimal::from_float(5.0); // bigCrunchModes milestone
        assert!(game.big_crunch_autobuyer_has_modes());
        let pending = game.gained_infinity_points();
        assert!(pending > Decimal::ONE);

        // Threshold above the pending gain: no crunch.
        game.autobuyers.big_crunch_settings.amount = pending * Decimal::from_float(10.0);
        let inf_before = game.infinities;
        game.tick_autobuyers(150.0);
        assert_eq!(game.infinities, inf_before);

        // Threshold at/below the pending gain: crunches.
        game.autobuyers.big_crunch_settings.amount = pending;
        game.tick_autobuyers(150.0);
        assert!(game.infinities > inf_before);
    }

    #[test]
    fn crunch_time_mode_waits_for_infinity_age() {
        let mut game = game_post_break_at_goal();
        game.eternities = Decimal::from_float(5.0);
        game.autobuyers.big_crunch_settings.mode = PrestigeAutobuyerMode::Time;
        game.autobuyers.big_crunch_settings.time = 10.0;

        game.records.this_infinity.real_time_ms = 5_000.0;
        let inf_before = game.infinities;
        game.tick_autobuyers(150.0);
        assert_eq!(game.infinities, inf_before);

        game.records.this_infinity.real_time_ms = 11_000.0;
        game.tick_autobuyers(150.0);
        assert!(game.infinities > inf_before);
    }

    #[test]
    fn crunch_x_highest_mode_compares_against_eternity_peak() {
        let mut game = game_post_break_at_goal();
        game.eternities = Decimal::from_float(5.0);
        game.autobuyers.big_crunch_settings.mode = PrestigeAutobuyerMode::XHighest;
        game.autobuyers.big_crunch_settings.x_highest = Decimal::from_float(2.0);
        let pending = game.gained_infinity_points();

        // Peak too high: pending < 2 × peak.
        game.records.this_eternity.max_ip = pending;
        let inf_before = game.infinities;
        game.tick_autobuyers(150.0);
        assert_eq!(game.infinities, inf_before);

        // Low peak: pending ≥ 2 × peak → crunch.
        game.records.this_eternity.max_ip = pending / Decimal::from_float(4.0);
        game.tick_autobuyers(150.0);
        assert!(game.infinities > inf_before);
    }

    #[test]
    fn crunch_mode_resets_without_milestone_on_eternity() {
        let mut game = game_post_break_at_goal();
        game.eternities = Decimal::from_float(5.0);
        game.autobuyers.big_crunch_settings.mode = PrestigeAutobuyerMode::Time;

        // An Eternity keeps the mode while the milestone holds (5 + 1 ≥ 5)...
        // Slow eternities (> 250 ms) so achievement 113 doesn't ×2 the gain.
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        game.records.this_eternity.time_ms = 60_000.0;
        assert!(game.eternity());
        assert_eq!(
            game.autobuyers.big_crunch_settings.mode,
            PrestigeAutobuyerMode::Time
        );

        // ...but reverts to Amount when eternities fall below it.
        game.autobuyers.big_crunch_settings.mode = PrestigeAutobuyerMode::XHighest;
        game.eternities = Decimal::from_float(3.0);
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        game.records.this_eternity.time_ms = 60_000.0;
        assert!(game.eternity());
        assert_eq!(
            game.autobuyers.big_crunch_settings.mode,
            PrestigeAutobuyerMode::Amount
        );
    }

    #[test]
    fn eternity_autobuyer_needs_milestone_and_goal() {
        let mut game = GameState::new();
        game.eternity_unlocked = true;
        game.autobuyers.eternity.is_active = true;
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        game.autobuyers.eternity.settings.amount = Decimal::ZERO;

        // Below 100 eternities the autobuyer is locked.
        game.eternities = Decimal::from_float(99.0);
        let before = game.eternities;
        game.tick_autobuyers(50.0);
        assert_eq!(game.eternities, before);

        // At the milestone it fires (amount 0 ≤ pending EP). Slow eternity
        // (> 250 ms) so achievement 113 doesn't ×2 the gain.
        game.eternities = Decimal::from_float(100.0);
        game.records.this_eternity.time_ms = 60_000.0;
        game.tick_autobuyers(50.0);
        // The eternity reset takes eternities to 100 + gained (1).
        assert_eq!(game.eternities, Decimal::from_float(101.0));
    }

    #[test]
    fn eternity_autobuyer_amount_mode_waits_for_ep() {
        let mut game = GameState::new();
        game.eternity_unlocked = true;
        game.eternities = Decimal::from_float(100.0);
        game.autobuyers.eternity.is_active = true;
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        let pending = game.gained_eternity_points();

        game.autobuyers.eternity.settings.amount = pending * Decimal::from_float(100.0);
        let before = game.eternities;
        game.tick_autobuyers(50.0);
        assert_eq!(game.eternities, before);

        game.autobuyers.eternity.settings.amount = pending;
        game.tick_autobuyers(50.0);
        assert!(game.eternities > before);
    }

    #[test]
    fn eternity_autobuyer_deactivates_on_reset_without_milestone() {
        let mut game = GameState::new();
        game.eternity_unlocked = true;
        game.eternities = Decimal::from_float(50.0);
        game.autobuyers.eternity.is_active = true;
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        assert!(game.eternity());
        assert!(!game.autobuyers.eternity.is_active);
    }

    #[test]
    fn ep_mult_purchase_bumps_dynamic_eternity_amount() {
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(100.0); // autobuyer unlocked
        game.eternity_points = Decimal::from_float(1e10);
        assert!(game.autobuyers.eternity.settings.increase_with_mult);
        assert!(game.buy_ep_mult());
        assert_eq!(
            game.autobuyers.eternity.settings.amount,
            Decimal::from_float(5.0)
        );

        // With the checkbox off the amount stays put.
        game.autobuyers.eternity.settings.increase_with_mult = false;
        assert!(game.buy_ep_mult());
        assert_eq!(
            game.autobuyers.eternity.settings.amount,
            Decimal::from_float(5.0)
        );
    }

    #[test]
    fn mode_setters_gate_on_unlocks() {
        let mut game = GameState::new();
        // No milestone / RU13: only Amount is settable.
        assert!(!game.set_big_crunch_autobuyer_mode(PrestigeAutobuyerMode::Time));
        assert!(!game.set_eternity_autobuyer_mode(PrestigeAutobuyerMode::XHighest));
        assert!(game.set_big_crunch_autobuyer_mode(PrestigeAutobuyerMode::Amount));

        game.eternities = Decimal::from_float(5.0);
        assert!(game.set_big_crunch_autobuyer_mode(PrestigeAutobuyerMode::Time));
        game.reality.upgrade_bits |= 1 << 13;
        assert!(game.set_eternity_autobuyer_mode(PrestigeAutobuyerMode::XHighest));
    }

    #[test]
    fn reality_autobuyer_rm_mode_triggers_at_target() {
        let mut game = crate::reality::tests::game_at_reality_goal();
        game.reality.realities = 1; // past the first-reality special case
        game.reality.upgrade_bits |= 1 << 25; // RU25 unlocks the autobuyer
        game.autobuyers.reality.is_active = true;
        let pending_rm = game.gained_reality_machines();
        assert!(pending_rm >= Decimal::ONE);

        // Target above the pending gain: nothing happens.
        game.autobuyers.reality.rm = pending_rm * Decimal::from_float(10.0);
        game.tick_autobuyers(50.0);
        assert_eq!(game.reality.realities, 1);

        // Target at the pending gain: an automatic Reality fires.
        game.autobuyers.reality.rm = pending_rm;
        game.tick_autobuyers(50.0);
        assert_eq!(game.reality.realities, 2);
        assert!(game.reality.machines >= pending_rm);
    }

    #[test]
    fn reality_autobuyer_needs_ru25() {
        let mut game = crate::reality::tests::game_at_reality_goal();
        game.reality.realities = 1;
        game.autobuyers.reality.is_active = true;
        game.autobuyers.reality.rm = Decimal::ONE;
        game.tick_autobuyers(50.0);
        assert_eq!(game.reality.realities, 1); // locked without RU25
    }

    #[test]
    fn reality_autobuyer_time_mode() {
        let mut game = crate::reality::tests::game_at_reality_goal();
        game.reality.realities = 1;
        game.reality.upgrade_bits |= 1 << 25;
        game.autobuyers.reality.is_active = true;
        game.autobuyers.reality.mode = AutoRealityMode::Time;
        game.autobuyers.reality.time = 60.0;

        game.records.this_reality.real_time_ms = 30_000.0;
        game.tick_autobuyers(50.0);
        assert_eq!(game.reality.realities, 1);

        game.records.this_reality.real_time_ms = 61_000.0;
        game.tick_autobuyers(50.0);
        assert_eq!(game.reality.realities, 2);
    }

    #[test]
    fn break_infinity_unlockable_requires_nc12_and_maxed_interval() {
        let mut game = GameState::new();
        assert!(!game.break_infinity_unlockable());

        // NC12 completed, but the interval is still the 150 s base.
        game.complete_challenge(12);
        assert!(!game.break_infinity_unlockable());

        // Max the Big Crunch interval to the 100 ms floor.
        game.infinity_points = Decimal::from_float(1e9);
        for _ in 0..50 {
            game.upgrade_autobuyer_interval(AutobuyerTarget::BigCrunch);
        }
        assert!(game.autobuyer_has_maxed_interval(AutobuyerTarget::BigCrunch));
        assert!(game.break_infinity_unlockable());
    }
}
