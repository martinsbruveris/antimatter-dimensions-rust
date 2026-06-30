use break_infinity::Decimal;

use crate::data::constants::BIG_CRUNCH_THRESHOLD;
use crate::GameState;

/// Native offline tick resolution in ms (the original simulates offline at a
/// 50 ms base tick). See `design-docs/2026-06-30-offline-progress.md`.
const OFFLINE_BASE_TICK_MS: f64 = 50.0;

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

    /// Replay `game_ms` of (already speed-scaled) game time as offline progress.
    ///
    /// The interval is spread across `min(game_ms / 50, offline_ticks)` discrete
    /// ticks rather than one big step, so per-tick effects (autobuyers, which
    /// fire at most once per tick) behave. Below `offline_ticks × 50 ms` the
    /// replay runs at the native 50 ms resolution; beyond it the tick count
    /// saturates at `offline_ticks` and each tick stretches.
    ///
    /// `offline_ticks` is the player's resolution setting (default 100_000,
    /// reproducing the original game's offline tick budget). A non-positive
    /// `game_ms` is a no-op. See
    /// `design-docs/2026-06-30-offline-progress.md`.
    pub fn simulate_offline(&mut self, game_ms: f64, offline_ticks: u32) {
        if game_ms <= 0.0 {
            return;
        }
        let ticks = offline_tick_count(game_ms, offline_ticks);
        let tick_size = game_ms / ticks as f64; // >= 50 ms once saturated
        self.ticks(tick_size, ticks);
    }
}

/// The discrete tick budget for replaying `game_ms` of offline time:
/// `min(game_ms / 50, offline_ticks)`, never below 1. Below
/// `offline_ticks × 50 ms` this is the native 50 ms count; past it the budget
/// saturates at `offline_ticks` (so each tick covers more than 50 ms).
fn offline_tick_count(game_ms: f64, offline_ticks: u32) -> u32 {
    let want = (game_ms / OFFLINE_BASE_TICK_MS).floor() as u32;
    want.clamp(1, offline_ticks.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AutobuyerMode, GameState};

    #[test]
    fn offline_tick_count_native_resolution_below_cap() {
        // 50 s of game time wants 1000 ticks at 50 ms; a generous budget leaves
        // that untouched.
        assert_eq!(offline_tick_count(50_000.0, 100_000), 1000);
    }

    #[test]
    fn offline_tick_count_saturates_at_budget() {
        // A long interval saturates at `offline_ticks`; each tick then spans far
        // more than 50 ms.
        let ticks = offline_tick_count(50_000_000.0, 1000);
        assert_eq!(ticks, 1000);
        let tick_size = 50_000_000.0 / ticks as f64;
        assert!(tick_size >= OFFLINE_BASE_TICK_MS);
        assert_eq!(tick_size, 50_000.0);
    }

    #[test]
    fn offline_tick_count_floor_is_one() {
        // Sub-tick intervals and a zero budget both clamp up to a single tick.
        assert_eq!(offline_tick_count(10.0, 1000), 1);
        assert_eq!(offline_tick_count(1_000_000.0, 0), 1);
    }

    #[test]
    fn simulate_offline_non_positive_is_noop() {
        let mut game = GameState::new();
        game.dimensions[1].amount = Decimal::new(1.0, 1);
        let before = game.antimatter;

        game.simulate_offline(0.0, 1000);
        game.simulate_offline(-5_000.0, 1000);

        assert_eq!(game.antimatter, before);
    }

    #[test]
    fn simulate_offline_matches_simulate_at_native_resolution() {
        // When the budget doesn't bind, simulate_offline is exactly the native
        // 50 ms tick loop (`simulate` with a 50 ms step over the same total).
        let mut base = GameState::new();
        base.dimensions[0].amount = Decimal::new(1.0, 1);
        base.dimensions[1].amount = Decimal::new(1.0, 1);

        let mut via_offline = base.clone();
        via_offline.simulate_offline(50_000.0, 100_000);

        let mut via_simulate = base;
        via_simulate.simulate(50_000.0, OFFLINE_BASE_TICK_MS);

        assert_eq!(via_offline.antimatter, via_simulate.antimatter);
        for tier in 0..8 {
            assert_eq!(
                via_offline.dimensions[tier].amount,
                via_simulate.dimensions[tier].amount
            );
        }
    }

    #[test]
    fn offline_ticks_is_a_behaviour_knob_for_autobuyers() {
        // The tick budget governs how often once-per-tick effects fire: more
        // ticks over the same game time → more autobuyer purchases. With ample
        // antimatter, the 1st-dimension autobuyer buys more under a large budget
        // (fine resolution) than a tiny one (coarse).
        let mut base = GameState::new();
        base.antimatter = Decimal::new(1.0, 100);
        base.autobuyers.enabled = true;
        base.autobuyers.dimensions[0].is_bought = true;
        base.autobuyers.dimensions[0].is_active = true;
        base.autobuyers.dimensions[0].mode = AutobuyerMode::BuySingle;

        // 50 s of game time. Large budget → 50 ms ticks (≈100 autobuyer fires at
        // the 500 ms interval); tiny budget → 5 s ticks (one fire each, 10 total).
        let mut fine = base.clone();
        fine.simulate_offline(50_000.0, 100_000);

        let mut coarse = base;
        coarse.simulate_offline(50_000.0, 10);

        assert!(
            fine.dimensions[0].bought > coarse.dimensions[0].bought,
            "fine={} coarse={}",
            fine.dimensions[0].bought,
            coarse.dimensions[0].bought
        );
    }

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
