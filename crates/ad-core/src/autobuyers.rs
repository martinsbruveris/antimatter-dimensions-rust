use crate::state::GameState;

/// Autobuyer purchase mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutobuyerMode {
    /// Buy a single unit each time the autobuyer fires.
    BuySingle,
    /// Buy the maximum affordable amount each time.
    BuyMax,
}

/// State for a single autobuyer.
#[derive(Debug, Clone)]
pub struct Autobuyer {
    /// Whether this autobuyer is enabled.
    pub enabled: bool,
    /// Interval between purchases in milliseconds.
    pub interval_ms: f64,
    /// Current timer tracking elapsed time since last purchase.
    pub timer_ms: f64,
    /// Purchase mode (single or max).
    pub mode: AutobuyerMode,
}

impl Autobuyer {
    pub fn new(interval_ms: f64) -> Self {
        Self {
            enabled: false,
            interval_ms,
            timer_ms: 0.0,
            mode: AutobuyerMode::BuySingle,
        }
    }

    /// Advance the timer by `dt_ms`. Returns true if the autobuyer should fire.
    pub fn advance(&mut self, dt_ms: f64) -> bool {
        if !self.enabled {
            return false;
        }

        self.timer_ms += dt_ms;
        if self.timer_ms >= self.interval_ms {
            self.timer_ms -= self.interval_ms;
            // Clamp timer to prevent unbounded accumulation if dt is very large
            if self.timer_ms >= self.interval_ms {
                self.timer_ms = 0.0;
            }
            true
        } else {
            false
        }
    }
}

/// Initial autobuyer interval for all autobuyers (in milliseconds).
pub const AUTOBUYER_INITIAL_INTERVAL_MS: f64 = 1000.0;

/// Collection of all autobuyer state.
#[derive(Debug, Clone)]
pub struct AutobuyerState {
    /// Autobuyers for each of the 8 antimatter dimension tiers.
    pub dimensions: [Autobuyer; 8],
    /// Autobuyer for tickspeed upgrades.
    pub tickspeed: Autobuyer,
}

impl AutobuyerState {
    pub fn new() -> Self {
        Self {
            dimensions: std::array::from_fn(|_| {
                Autobuyer::new(AUTOBUYER_INITIAL_INTERVAL_MS)
            }),
            tickspeed: Autobuyer::new(AUTOBUYER_INITIAL_INTERVAL_MS),
        }
    }
}

impl Default for AutobuyerState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// Advance all autobuyers by `dt_ms` and execute any triggered purchases.
    pub fn tick_autobuyers(&mut self, dt_ms: f64) {
        // Process dimension autobuyers
        for tier in 0..8 {
            if self.autobuyers.dimensions[tier].advance(dt_ms) {
                match self.autobuyers.dimensions[tier].mode {
                    AutobuyerMode::BuySingle => {
                        self.buy_dimension(tier);
                    }
                    AutobuyerMode::BuyMax => {
                        self.buy_max_dimension(tier);
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
