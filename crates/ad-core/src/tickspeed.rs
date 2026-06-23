use break_infinity::Decimal;

use crate::data::constants::{
    GALAXY_TICKSPEED_REDUCTION, INITIAL_TICKSPEED_MS, TICKSPEED_MULTIPLIER,
    TICKSPEED_MULTIPLIER_MIN,
};
use crate::state::GameState;

impl GameState {
    /// Buy one tickspeed upgrade. Returns true if successful.
    pub fn buy_tickspeed(&mut self) -> bool {
        if self.antimatter >= self.tickspeed.cost {
            self.antimatter -= self.tickspeed.cost;
            self.tickspeed.bought += 1;
            self.tickspeed.cost *= self.tickspeed.cost_multiplier;
            true
        } else {
            false
        }
    }

    /// Buy the maximum number of tickspeed upgrades affordable.
    /// Returns the number bought.
    pub fn buy_max_tickspeed(&mut self) -> u64 {
        let mut count = 0u64;
        while self.buy_tickspeed() {
            count += 1;
        }
        count
    }

    /// Compute the current tickspeed in milliseconds.
    /// Formula: INITIAL_TICKSPEED_MS * multiplier^bought
    /// where multiplier = max(TICKSPEED_MULTIPLIER - galaxies * GALAXY_TICKSPEED_REDUCTION,
    ///                        TICKSPEED_MULTIPLIER_MIN)
    pub fn current_tickspeed_ms(&self) -> f64 {
        let multiplier = self.tickspeed_purchase_multiplier();
        INITIAL_TICKSPEED_MS * multiplier.powi(self.tickspeed.bought as i32)
    }

    /// The per-purchase tickspeed multiplier (fraction retained per purchase).
    /// Reduced by galaxies.
    pub fn tickspeed_purchase_multiplier(&self) -> f64 {
        let reduction = self.galaxies as f64 * GALAXY_TICKSPEED_REDUCTION;
        (TICKSPEED_MULTIPLIER - reduction).max(TICKSPEED_MULTIPLIER_MIN)
    }

    /// Compute the effective production multiplier from tickspeed.
    /// Production is inversely proportional to tickspeed interval:
    /// effect = INITIAL_TICKSPEED_MS / current_tickspeed_ms
    /// This means if tickspeed is 500ms (half of 1000ms), production is 2x.
    pub fn tickspeed_effect(&self) -> Decimal {
        let current = self.current_tickspeed_ms();
        if current <= 0.0 {
            return Decimal::from_float(1.0);
        }
        Decimal::from_float(INITIAL_TICKSPEED_MS / current)
    }
}
