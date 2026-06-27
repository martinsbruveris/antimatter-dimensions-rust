use break_infinity::Decimal;

use crate::data::constants::{
    AD_AUTOBUYER_INTERVALS_MS, AD_AUTOBUYER_REQUIREMENTS, AUTOMATION_TAB_REQUIREMENT,
    TICKSPEED_AUTOBUYER_INTERVAL_MS, TICKSPEED_AUTOBUYER_REQUIREMENT,
};
use crate::state::GameState;

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

/// State for a single autobuyer.
///
/// Pre-Infinity, an autobuyer must first be unlocked (`is_bought`) by clicking
/// its purchase box once the antimatter requirement is met — unlocking costs no
/// antimatter. `interval_ms` is fixed pre-Infinity (interval upgrades cost
/// Infinity Points and are not modelled yet).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Autobuyer {
    /// Whether the slow version has been unlocked (JS `data.isBought`).
    pub is_bought: bool,
    /// Per-autobuyer on/off toggle (JS `data.isActive`). Defaults on.
    pub is_active: bool,
    /// Purchase mode (single or max).
    pub mode: AutobuyerMode,
    /// Interval between purchases in milliseconds.
    pub interval_ms: f64,
    /// Current timer tracking elapsed time since the last purchase.
    pub timer_ms: f64,
}

impl Autobuyer {
    pub fn new(interval_ms: f64, mode: AutobuyerMode) -> Self {
        Self {
            is_bought: false,
            is_active: true,
            mode,
            interval_ms,
            timer_ms: 0.0,
        }
    }

    /// Advance the timer by `dt_ms`. Returns true if the autobuyer should fire
    /// this step. Does nothing (and never fires) unless the autobuyer is both
    /// unlocked and active.
    fn advance(&mut self, dt_ms: f64) -> bool {
        if !self.is_bought || !self.is_active {
            return false;
        }

        self.timer_ms += dt_ms;
        if self.timer_ms >= self.interval_ms {
            self.timer_ms -= self.interval_ms;
            // Clamp timer to prevent unbounded accumulation if dt is very large.
            if self.timer_ms >= self.interval_ms {
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

    /// Advance all autobuyers by `dt_ms` and execute any triggered purchases.
    /// Does nothing if autobuyers are globally disabled.
    pub fn tick_autobuyers(&mut self, dt_ms: f64) {
        if !self.autobuyers.enabled {
            return;
        }

        // Process dimension autobuyers
        for tier in 0..8 {
            if self.autobuyers.dimensions[tier].advance(dt_ms) {
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

        // Process tickspeed autobuyer
        if self.autobuyers.tickspeed.advance(dt_ms) {
            match self.autobuyers.tickspeed.mode {
                AutobuyerMode::BuySingle => {
                    self.buy_tickspeed();
                }
                AutobuyerMode::BuyMax => {
                    self.buy_max_tickspeed();
                }
            }
        }
    }
}
