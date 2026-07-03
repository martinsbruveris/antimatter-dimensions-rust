use break_infinity::Decimal;

use crate::data::constants::{
    GALAXY_TICKSPEED_REDUCTION, INITIAL_TICKSPEED_MS, TICKSPEED_BASE_MULTIPLIERS,
    TICKSPEED_BASE_MULTIPLIERS_C5, TICKSPEED_GALAXY_BASE, TICKSPEED_GALAXY_BASE_C5,
    TICKSPEED_GALAXY_DECAY, TICKSPEED_MULTIPLIER_MIN,
};
use crate::state::GameState;

impl GameState {
    /// Buy one tickspeed upgrade. Returns true if successful.
    pub fn buy_tickspeed(&mut self) -> bool {
        if self.antimatter >= self.tickspeed.cost {
            // Clear the TICKSPEED tutorial highlight on the purchase, like the
            // original's buyTickSpeed (no-op once past that step).
            self.tutorial_turn_off(crate::tutorial::state::TICKSPEED);
            // NC9: buying a Tickspeed upgrade bumps every equal-cost dimension to
            // its next cost step (before the purchase, using the current cost).
            if self.challenge_running(9) {
                self.nc9_bump_same_cost_from_tickspeed();
            }
            self.antimatter -= self.tickspeed.cost;
            self.tickspeed.bought += 1;
            self.tickspeed.cost *= self.tickspeed.cost_multiplier;
            // Normal Challenge 2: buying a Tickspeed upgrade also halts production.
            if self.challenge_running(2) {
                self.chall2_pow = 0.0;
            }
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
    pub fn current_tickspeed_ms(&self) -> f64 {
        let multiplier = self.tickspeed_purchase_multiplier();
        INITIAL_TICKSPEED_MS * multiplier.powi(self.tickspeed.bought as i32)
    }

    /// The per-purchase tickspeed multiplier (fraction of
    /// current tickspeed retained per purchase). Uses two
    /// formulas depending on galaxy count:
    ///
    /// Pre-3 galaxies (linear):
    ///   base_mult[galaxies] - galaxies * 0.02
    ///
    /// 3+ galaxies (exponential):
    ///   0.8 * 0.965^(galaxies - 4)
    pub fn tickspeed_purchase_multiplier(&self) -> f64 {
        let galaxies = self.galaxies as f64;
        // The original's `effects` product scales the per-galaxy term; the only
        // pre-Infinity contributor is the `galaxyBoost` Infinity Upgrade (×2).
        let effects = self.galaxy_strength_effect();

        // Normal Challenge 5 lowers the base multiplier (the tickspeed purchase
        // multiplier starts at ×1.080 instead of ×1.1245).
        let in_c5 = self.challenge_running(5);

        if self.galaxies < 3 {
            let base = if in_c5 {
                TICKSPEED_BASE_MULTIPLIERS_C5[self.galaxies as usize]
            } else {
                TICKSPEED_BASE_MULTIPLIERS[self.galaxies as usize]
            };
            // perGalaxy = 0.02 * effects; reduction = galaxies * perGalaxy.
            let reduction = galaxies * GALAXY_TICKSPEED_REDUCTION * effects;
            (base - reduction).max(TICKSPEED_MULTIPLIER_MIN)
        } else {
            // JS: galaxies -= 2; galaxies *= effects; decay^(galaxies - 2) * base.
            let galaxy_base = if in_c5 {
                TICKSPEED_GALAXY_BASE_C5
            } else {
                TICKSPEED_GALAXY_BASE
            };
            let adjusted = (galaxies - 2.0) * effects;
            (galaxy_base * TICKSPEED_GALAXY_DECAY.powf(adjusted - 2.0))
                .max(TICKSPEED_MULTIPLIER_MIN)
        }
    }

    /// Compute the effective production multiplier from
    /// tickspeed. Production is inversely proportional to
    /// tickspeed interval:
    ///   effect = INITIAL_TICKSPEED_MS / current_tickspeed_ms
    pub fn tickspeed_effect(&self) -> Decimal {
        let current = self.current_tickspeed_ms();
        if current <= 0.0 {
            return Decimal::from_float(1.0);
        }
        Decimal::from_float(INITIAL_TICKSPEED_MS / current)
    }
}
