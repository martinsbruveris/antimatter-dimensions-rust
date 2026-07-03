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
    /// Current timer tracking elapsed time since the last purchase.
    pub timer_ms: f64,
}

/// serde default for [`Autobuyer::cost`] (1 IP), since `f64`'s `Default` is 0.
#[cfg(feature = "serde")]
fn default_autobuyer_cost() -> f64 {
    1.0
}

impl Autobuyer {
    pub fn new(interval_ms: f64, mode: AutobuyerMode) -> Self {
        Self {
            is_bought: false,
            is_active: true,
            mode,
            interval_ms,
            cost: 1.0,
            timer_ms: 0.0,
        }
    }

    /// Whether the interval is at its 100 ms floor (JS `hasMaxedInterval`).
    pub fn has_maxed_interval(&self) -> bool {
        self.interval_ms <= AUTOBUYER_MIN_INTERVAL_MS
    }

    /// Advance the timer by `dt_ms`, firing when it reaches `effective_interval_ms`
    /// (the stored interval after the `autobuyerSpeed` Break Infinity Upgrade's
    /// halving). Does nothing (and never fires) while inactive. The *unlocked*
    /// check lives in the caller ([`GameState::tick_autobuyers`]) via
    /// `autobuyer_is_unlocked`, since some autobuyers unlock by challenge rather
    /// than the `is_bought` flag.
    fn advance(&mut self, dt_ms: f64, effective_interval_ms: f64) -> bool {
        if !self.is_active {
            return false;
        }

        self.timer_ms += dt_ms;
        if self.timer_ms >= effective_interval_ms {
            self.timer_ms -= effective_interval_ms;
            // Clamp timer to prevent unbounded accumulation if dt is very large.
            if self.timer_ms >= effective_interval_ms {
                self.timer_ms = 0.0;
            }
            true
        } else {
            false
        }
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
    /// Antimatter Galaxy autobuyer (unlocked by completing NC11).
    #[cfg_attr(feature = "serde", serde(default = "default_galaxy_autobuyer"))]
    pub galaxy: Autobuyer,
    /// Big Crunch autobuyer (unlocked by completing NC12). Its maxed interval
    /// gates Break Infinity.
    #[cfg_attr(feature = "serde", serde(default = "default_big_crunch_autobuyer"))]
    pub big_crunch: Autobuyer,
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
            galaxy: Autobuyer::new(
                GALAXY_AUTOBUYER_INTERVAL_MS,
                AutobuyerMode::BuySingle,
            ),
            big_crunch: Autobuyer::new(
                BIG_CRUNCH_AUTOBUYER_INTERVAL_MS,
                AutobuyerMode::BuySingle,
            ),
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
            return;
        }

        // The `autobuyerSpeed` Break Infinity Upgrade halves every autobuyer's
        // effective interval.
        let speedup = self.break_infinity_autobuyer_speedup();

        // Antimatter dimension autobuyers.
        for tier in 0..8 {
            let unlocked = self.autobuyer_is_unlocked(AutobuyerTarget::AdTier(tier));
            let eff = self.autobuyers.dimensions[tier].interval_ms * speedup;
            if unlocked && self.autobuyers.dimensions[tier].advance(dt_ms, eff) {
                match self.autobuyers.dimensions[tier].mode {
                    AutobuyerMode::BuySingle => {
                        self.buy_dimension(tier);
                    }
                    // BUY_10: with default bulk 1 the original fills the
                    // current group of ten once per tick.
                    AutobuyerMode::BuyMax => {
                        self.buy_until_10_dimension(tier);
                    }
                }
            }
        }

        // Tickspeed autobuyer.
        let unlocked = self.autobuyer_is_unlocked(AutobuyerTarget::Tickspeed);
        let eff = self.autobuyers.tickspeed.interval_ms * speedup;
        if unlocked && self.autobuyers.tickspeed.advance(dt_ms, eff) {
            match self.autobuyers.tickspeed.mode {
                AutobuyerMode::BuySingle => {
                    self.buy_tickspeed();
                }
                AutobuyerMode::BuyMax => {
                    self.buy_max_tickspeed();
                }
            }
        }

        // Prestige autobuyers (unlocked by completing NC10/11/12). Each fires its
        // fixed action, which is a no-op when its precondition isn't met — the
        // original gates the tick on the same `canBeBought`/`canCrunch` conditions.
        let unlocked = self.autobuyer_is_unlocked(AutobuyerTarget::DimBoost);
        let eff = self.autobuyers.dim_boost.interval_ms * speedup;
        if unlocked && self.autobuyers.dim_boost.advance(dt_ms, eff) {
            self.buy_dim_boost();
        }

        let unlocked = self.autobuyer_is_unlocked(AutobuyerTarget::Galaxy);
        let eff = self.autobuyers.galaxy.interval_ms * speedup;
        if unlocked && self.autobuyers.galaxy.advance(dt_ms, eff) {
            self.buy_galaxy();
        }

        // Big Crunch: pre-break `willInfinity` is always true, so it crunches as
        // soon as the goal is reached (`big_crunch` no-ops otherwise).
        let unlocked = self.autobuyer_is_unlocked(AutobuyerTarget::BigCrunch);
        let eff = self.autobuyers.big_crunch.interval_ms * speedup;
        if unlocked && self.autobuyers.big_crunch.advance(dt_ms, eff) {
            self.big_crunch();
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
        game.antimatter = BIG_CRUNCH_THRESHOLD; // at the goal
        let inf_before = game.infinities;

        game.tick_autobuyers(150.0);

        assert!(game.infinities > inf_before);
        assert!(game.antimatter < BIG_CRUNCH_THRESHOLD); // reset by the crunch
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
