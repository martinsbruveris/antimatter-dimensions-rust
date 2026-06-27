use break_infinity::Decimal;

use crate::data::constants::BIG_CRUNCH_THRESHOLD;
use crate::GameState;

#[allow(clippy::needless_range_loop)]
impl GameState {
    /// Advance the game by `dt_ms` milliseconds.
    /// Production chain: AD[n+1] produces AD[n], AD1 produces antimatter.
    /// All production is scaled by the dimension's multiplier and tickspeed effect.
    pub fn tick(&mut self, dt_ms: f64) {
        // Run autobuyers before production
        self.tick_autobuyers(dt_ms);

        let dt_seconds = dt_ms / 1000.0;
        let dt = Decimal::from_float(dt_seconds);

        // Production flows from higher dimensions to lower.
        // AD8 produces AD7, AD7 produces AD6, ..., AD2 produces AD1, AD1 produces
        // antimatter. We compute all production first to avoid order-of-update issues.

        let unlocked = self.unlocked_dimensions();
        let productions: [Decimal; 8] = std::array::from_fn(|tier| {
            if tier < unlocked {
                self.dimension_production_per_second(tier) * dt
            } else {
                Decimal::ZERO
            }
        });

        // Apply production: each tier produces into the tier below it
        // Tier 0 (AD1) produces antimatter
        self.antimatter += productions[0];

        // Track all-time antimatter produced (monotonic, survives crunches).
        // Counted before the Big Crunch cap so it reflects true production.
        self.total_antimatter += productions[0];

        // Tiers 1-7 produce into the tier below
        for tier in 1..unlocked {
            self.dimensions[tier - 1].amount += productions[tier];
        }

        // Cap antimatter at the Big Crunch threshold. Pre-Infinity, antimatter
        // cannot exceed Number.MAX_VALUE; the player must Crunch to progress.
        // This cap is lifted once breaking Infinity is implemented.
        if self.antimatter > BIG_CRUNCH_THRESHOLD {
            self.antimatter = BIG_CRUNCH_THRESHOLD;
        }
    }

    /// Advance the game by `repeats` discrete steps of `dt_ms` each.
    ///
    /// Used by the dev game-speed control: running N real-sized ticks is more
    /// faithful than a single `dt_ms * N` step, which would lump discrete
    /// per-tick effects (e.g. autobuyers) into one and lose precision.
    pub fn ticks(&mut self, dt_ms: f64, repeats: u32) {
        for _ in 0..repeats {
            self.tick(dt_ms);
        }
    }

    /// Run the simulation for `total_ms` of real time, using `tick_size_ms` per step.
    pub fn simulate(&mut self, total_ms: f64, tick_size_ms: f64) {
        let steps = (total_ms / tick_size_ms) as u64;
        for _ in 0..steps {
            self.tick(tick_size_ms);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GameState;

    #[test]
    fn tick_caps_antimatter_at_big_crunch_threshold() {
        let cap = BIG_CRUNCH_THRESHOLD;
        let mut game = GameState::new();

        // Start just below the cap with strong production so a tick would
        // otherwise push antimatter well past it.
        game.antimatter = cap * Decimal::from_float(0.9);
        game.dimensions[0].amount = Decimal::new(1.0, 400);

        game.tick(1000.0);

        assert!(
            game.antimatter <= cap,
            "antimatter {:?} exceeded the cap {:?}",
            game.antimatter,
            cap
        );
        assert_eq!(game.antimatter, cap);
    }
}
