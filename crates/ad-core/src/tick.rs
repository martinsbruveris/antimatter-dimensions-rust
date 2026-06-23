use break_infinity::Decimal;

use crate::GameState;

#[allow(clippy::needless_range_loop)]
impl GameState {
    /// Advance the game by `dt_ms` milliseconds.
    /// Production chain: AD[n+1] produces AD[n], AD1 produces antimatter.
    /// All production is scaled by the dimension's multiplier and tickspeed effect.
    pub fn tick(&mut self, dt_ms: f64) {
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

        // Tiers 1-7 produce into the tier below
        for tier in 1..unlocked {
            self.dimensions[tier - 1].amount += productions[tier];
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
