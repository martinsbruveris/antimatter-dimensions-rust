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
            // IC8: buying a Tickspeed upgrade resets the production-decay timer.
            self.records.this_infinity.last_buy_time_ms =
                self.records.this_infinity.time_ms;
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

    /// Total Tickspeed upgrades: bought plus the free upgrades from Time
    /// Shards (`Tickspeed.totalUpgrades = totalTickBought + totalTickGained`).
    pub fn total_tickspeed_upgrades(&self) -> u64 {
        self.tickspeed.bought + self.total_tick_gained
    }

    /// Compute the current tickspeed in milliseconds:
    /// `INITIAL_TICKSPEED_MS × multiplier^totalUpgrades`. A `Decimal` because
    /// free Tickspeed upgrades push the count far past what `f64` can hold
    /// (`0.8^300000` underflows) — the original's `Tickspeed.current` is a
    /// Decimal too.
    pub fn current_tickspeed_ms(&self) -> Decimal {
        let multiplier = self.tickspeed_purchase_multiplier();
        Decimal::from_float(INITIAL_TICKSPEED_MS)
            * Decimal::from_float(multiplier)
                .pow(&Decimal::from(self.total_tickspeed_upgrades()))
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
        // `effectiveBaseGalaxies`: antimatter galaxies plus Replicanti Galaxies feed
        // the tickspeed formula (the branch cutoff, per-galaxy reduction, and the
        // exponent). The base-multiplier lookup, however, keys off the *antimatter*
        // galaxy count (JS: `player.galaxies === 1/2`), since only 0–2 antimatter
        // galaxies are possible while the effective count is < 3.
        let eff = self.effective_galaxies();
        let galaxies = eff as f64;
        // The original's `effects` product scales the per-galaxy term: the
        // `galaxyBoost` Infinity/Break-Infinity Upgrades plus the galaxy-strength
        // Time Studies — TS212 (from Time Shards, cap ×1.1) and TS232 (from
        // Antimatter Galaxies).
        let mut effects = self.galaxy_strength_effect();
        if self.time_study_bought(212) {
            let log2_shards = self.time_shards.max(&Decimal::from_float(2.0)).ln()
                / std::f64::consts::LN_2;
            effects *= log2_shards.powf(0.005).min(1.1);
        }
        if self.time_study_bought(232) {
            effects *= (1.0 + self.galaxies as f64 / 1000.0).powf(0.2);
        }

        // Normal Challenge 5 lowers the base multiplier (the tickspeed purchase
        // multiplier starts at ×1.080 instead of ×1.1245).
        let in_c5 = self.challenge_running(5);

        if eff < 3 {
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
        // Infinity Challenge 3 neutralises Tickspeed (its production effect → ×1);
        // in exchange it grants a static Antimatter Dimension multiplier.
        if self.ic3_neutralizes_tickspeed() {
            return Decimal::ONE;
        }
        let current = self.current_tickspeed_ms();
        if current <= Decimal::ZERO {
            return Decimal::from_float(1.0);
        }
        Decimal::from_float(INITIAL_TICKSPEED_MS) / current
    }
}
