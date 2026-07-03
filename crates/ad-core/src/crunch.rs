use break_infinity::Decimal;

use crate::data::constants::BIG_CRUNCH_THRESHOLD;
use crate::records::ThisInfinity;
use crate::state::{DimensionTier, GameState, TickspeedState};

impl GameState {
    /// Whether the player can perform a Big Crunch: antimatter has reached the
    /// Big Crunch threshold (where it is capped, see `tick`).
    pub fn can_big_crunch(&self) -> bool {
        self.antimatter >= BIG_CRUNCH_THRESHOLD
    }

    /// The Infinity-Point formula divisor (`Effects.min(308, Achievement(103),
    /// TimeStudy(111))`). Both reducers are post-Infinity content we don't model
    /// yet, so this is a constant 308 for now; kept as a method so those slot in
    /// by lowering the value rather than rewriting `gained_infinity_points`.
    fn ip_gain_divisor(&self) -> f64 {
        308.0
    }

    /// The global Infinity-Point multiplier (`GameCache.totalIPMult`). Every
    /// source (the IP-mult Infinity Upgrade, achievements 85/93/116/125, time
    /// studies, …) is a later feature, so this is 1 for now; kept as a method so
    /// those become multiplicative additions here. Read by
    /// [`GameState::generate_passive_ip`] too.
    pub(crate) fn total_ip_mult(&self) -> Decimal {
        Decimal::ONE
    }

    /// Infinity Points a Big Crunch would grant right now. Mirrors
    /// `gainedInfinityPoints`: pre-Break-Infinity the base is `308 / div` (= 1
    /// with `div = 308`), times `totalIPMult` (= 1), floored. The Break-Infinity
    /// branch (`pow10(maxAM.log10() / div - 0.75)`) arrives with Feature 2.3.
    pub fn gained_infinity_points(&self) -> Decimal {
        let base = Decimal::from_float(308.0 / self.ip_gain_divisor());
        (base * self.total_ip_mult()).floor()
    }

    /// Infinities a Big Crunch would grant right now. Mirrors `gainedInfinities`:
    /// `Effects.max(1, Achievement(87)) × …`, all sources post-Reality, so 1 here.
    pub fn gained_infinities(&self) -> Decimal {
        Decimal::ONE
    }

    /// Perform the first Big Crunch (Infinity): reset all pre-Infinity progress
    /// — antimatter, dimensions, tickspeed, dimension boosts, galaxies, and
    /// sacrifices — back to the start. Autobuyer configuration (unlocks, modes,
    /// toggles) and the all-time `total_antimatter` record are preserved — they
    /// are not pre-Infinity progress, matching the original where the Automation
    /// tab and unlocked autobuyers persist through Infinity. Returns true if the
    /// crunch happened. The `infinity_unlocked` flag and the all-time
    /// `total_antimatter` record are preserved (not pre-Infinity progress).
    ///
    /// Infinity Points are not awarded yet; that comes in a later step.
    pub fn big_crunch(&mut self) -> bool {
        if !self.can_big_crunch() {
            return false;
        }
        self.big_crunch_reset(false, false);
        true
    }

    /// The Big-Crunch reset shared by the manual crunch and the challenge
    /// enter/exit paths. Rewards — achievement 21, Infinity Points, Infinities,
    /// challenge completion, and the fastest-infinity record — are granted only
    /// when actually at the goal (`can_big_crunch`); `forced` lets a challenge
    /// enter/exit reset below the goal without rewards. `entering_challenge`
    /// suppresses `skip_resets_if_possible` so a challenge starts fresh (mirrors
    /// `softReset(…, enteringAntimatterChallenge)`).
    ///
    /// Mirrors the original `bigCrunchReset(forced, enteringAntimatterChallenge)`.
    pub(crate) fn big_crunch_reset(&mut self, forced: bool, entering_challenge: bool) {
        let at_goal = self.can_big_crunch();
        if !forced && !at_goal {
            return;
        }

        if at_goal {
            // 21: "To infinity!" — unlocks on the crunch itself (the original's
            // BIG_CRUNCH_BEFORE), so the post-reset starting antimatter already
            // reflects its 100-antimatter reward.
            self.unlock_achievement(21);

            // Award rewards from the pre-reset state (IP reads `thisInfinity.maxAM`
            // once Break Infinity lands; both persist across the crunch).
            self.infinity_points += self.gained_infinity_points();
            self.infinities += self.gained_infinities();

            // Complete the running challenge, or NC1 on the first Infinity performed
            // outside a challenge (mirrors `handleChallengeCompletion`).
            self.handle_challenge_completion();

            // Lower the fastest-infinity record to this run before resetting it
            // (mirrors `bigCrunchUpdateStatistics` + `secondSoftReset`).
            self.records.best_infinity.time_ms = self
                .records
                .best_infinity
                .time_ms
                .min(self.records.this_infinity.time_ms);
            self.records.best_infinity.real_time_ms = self
                .records
                .best_infinity
                .real_time_ms
                .min(self.records.this_infinity.real_time_ms);

            self.infinity_unlocked = true;
        }

        self.antimatter = self.starting_antimatter();
        self.dimensions = std::array::from_fn(|_| DimensionTier::new());
        self.tickspeed = TickspeedState::new();
        self.dim_boosts = 0;
        self.galaxies = 0;
        self.sacrificed = Decimal::ZERO;
        // Reset the per-run challenge accumulators (`secondSoftReset` → `softReset`
        // → `resetChallengeStuff`).
        self.reset_challenge_stuff();
        // Re-apply skip-reset Infinity Upgrades (original `secondSoftReset` →
        // `softReset` → `skipResetsIfPossible`): start the next infinity already at
        // the highest owned skip level (and with a Galaxy for skipResetGalaxy).
        // Suppressed when entering a challenge (you start it fresh).
        if !entering_challenge {
            self.skip_resets_if_possible();
        }
        // Reset the current infinity's records (time/maxAM back to 0); the
        // fastest-infinity record and total time played persist.
        self.records.this_infinity = ThisInfinity::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_crunch_below_threshold() {
        let mut game = GameState::new();
        assert!(!game.can_big_crunch());
        assert!(!game.big_crunch());
    }

    #[test]
    fn crunch_at_threshold_resets_everything() {
        let mut game = GameState::new();

        // Advance some progress, then push antimatter to the threshold.
        game.dim_boosts = 6;
        game.galaxies = 3;
        game.sacrificed = Decimal::from_float(1e10);
        game.tickspeed.bought = 50;
        game.dimensions[0].bought = 30;
        game.dimensions[0].amount = Decimal::from_float(1e20);
        game.antimatter = BIG_CRUNCH_THRESHOLD;

        assert!(game.can_big_crunch());
        assert!(game.big_crunch());

        // Progress is back to a fresh game, except antimatter starts at 100:
        // the crunch unlocks achievement 21 ("To infinity!"), whose reward is a
        // 100-antimatter starting value.
        assert!(game.achievement_unlocked(21));
        assert_eq!(game.antimatter, Decimal::from_float(100.0));
        assert_eq!(game.dim_boosts, 0);
        assert_eq!(game.galaxies, 0);
        assert_eq!(game.sacrificed, Decimal::ZERO);
        assert_eq!(game.tickspeed.bought, 0);
        for tier in 0..8 {
            assert_eq!(game.dimensions[tier].bought, 0);
            assert_eq!(game.dimensions[tier].amount, Decimal::ZERO);
        }

        // No longer able to crunch after resetting.
        assert!(!game.can_big_crunch());

        // Infinity stays unlocked after the crunch (persists across resets).
        assert!(game.infinity_unlocked);
    }

    #[test]
    fn options_persist_through_crunch() {
        let mut game = GameState::new();
        game.options.hotkeys = false;
        game.options.set_update_rate(120);
        game.antimatter = BIG_CRUNCH_THRESHOLD;

        assert!(game.big_crunch());

        // Options are UI prefs, not pre-Infinity progress: unchanged by a crunch.
        assert!(!game.options.hotkeys);
        assert_eq!(game.options.update_rate, 120);
    }

    #[test]
    fn infinity_unlocked_starts_false_and_persists_after_crunch() {
        let mut game = GameState::new();
        assert!(!game.infinity_unlocked);

        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert!(game.infinity_unlocked);

        // A second crunch does not clear it.
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert!(game.infinity_unlocked);
    }

    #[test]
    fn crunch_awards_one_ip_and_one_infinity() {
        let mut game = GameState::new();
        assert_eq!(game.gained_infinity_points(), Decimal::ONE);
        assert_eq!(game.gained_infinities(), Decimal::ONE);

        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());

        assert_eq!(game.infinity_points, Decimal::ONE);
        assert_eq!(game.infinities, Decimal::ONE);
    }

    #[test]
    fn ip_and_infinities_accumulate_across_crunches() {
        let mut game = GameState::new();

        for expected in 1..=3 {
            game.antimatter = BIG_CRUNCH_THRESHOLD;
            assert!(game.big_crunch());
            assert_eq!(game.infinity_points, Decimal::from_float(expected as f64));
            assert_eq!(game.infinities, Decimal::from_float(expected as f64));
        }
    }

    #[test]
    fn crunch_updates_best_and_resets_this_infinity_records() {
        let mut game = GameState::new();

        // Spend 90 s of game time reaching the threshold, then crunch.
        game.dimensions[0].amount = Decimal::new(1.0, 400);
        game.simulate(90_000.0, 1000.0);
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        let this_time = game.records.this_infinity.time_ms;
        assert!(this_time >= 90_000.0);

        assert!(game.big_crunch());

        // Best infinity lowers to this run; this-infinity time/maxAM reset.
        assert_eq!(game.records.best_infinity.time_ms, this_time);
        assert_eq!(game.records.this_infinity.time_ms, 0.0);
        assert_eq!(game.records.this_infinity.max_am, Decimal::ZERO);
        // Total time played persists (not reset by the crunch).
        assert!(game.records.total_time_played_ms >= this_time);

        // A slower second infinity does not worsen the best record.
        game.dimensions[0].amount = Decimal::new(1.0, 400);
        game.simulate(120_000.0, 1000.0);
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert_eq!(game.records.best_infinity.time_ms, this_time);
    }

    #[test]
    fn tick_tracks_time_and_peak_antimatter() {
        let mut game = GameState::new();
        game.dimensions[0].amount = Decimal::new(1.0, 3);

        game.tick(1000.0);

        assert_eq!(game.records.total_time_played_ms, 1000.0);
        assert_eq!(game.records.this_infinity.time_ms, 1000.0);
        // maxAM tracks the peak reached (>= starting antimatter).
        assert!(game.records.this_infinity.max_am >= game.antimatter);
    }

    #[test]
    fn tickspeed_unlocks_with_second_dimension_purchase() {
        let mut game = GameState::new();
        assert!(!game.tickspeed_unlocked());

        game.dimensions[1].bought = 1;
        assert!(game.tickspeed_unlocked());
    }
}
