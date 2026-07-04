use break_infinity::Decimal;

use crate::data::constants::BIG_CRUNCH_THRESHOLD;
use crate::records::ThisInfinity;
use crate::state::{DimensionTier, GameState, TickspeedState};
use crate::tab_notifications::TabNotificationId;

impl GameState {
    /// The antimatter goal for the current run — an Infinity Challenge's own goal
    /// while one runs, else the `1e308` Big Crunch threshold (which is also the
    /// Normal Challenge goal). Mirrors `Player.infinityGoal`.
    pub fn infinity_goal(&self) -> Decimal {
        if self.infinity_challenge.current != 0 {
            Self::infinity_challenge_goal(self.infinity_challenge.current)
        } else {
            BIG_CRUNCH_THRESHOLD
        }
    }

    /// Whether the player can perform a Big Crunch: the peak antimatter this
    /// infinity has reached the goal (`Player.canCrunch`). Peak (not current) so a
    /// mid-run Dimension Boost/Galaxy reset doesn't revoke it.
    pub fn can_big_crunch(&self) -> bool {
        self.records.this_infinity.max_am.max(&self.antimatter) >= self.infinity_goal()
    }

    /// The Infinity-Point formula divisor (`Effects.min(308, Achievement(103),
    /// TimeStudy(111))`). TS111 lowers it to 285; Achievement 103 is a later
    /// feature.
    fn ip_gain_divisor(&self) -> f64 {
        if self.time_study_bought(111) {
            285.0
        } else {
            308.0
        }
    }

    /// The global Infinity-Point multiplier (`totalIPMult`): Time Studies
    /// 41/51/141/142/143. The IP-mult Infinity Upgrade and achievements
    /// 85/93/116/125 are later features. Read by
    /// [`GameState::generate_passive_ip`] too.
    pub(crate) fn total_ip_mult(&self) -> Decimal {
        let mut mult = Decimal::ONE;
        // TS41: ×1.2 per galaxy of any kind.
        if self.time_study_bought(41) {
            let galaxies = self.effective_galaxies();
            mult *= Decimal::from_float(1.2).pow(&Decimal::from(galaxies as u64));
        }
        if self.time_study_bought(51) {
            mult *= Decimal::new_unchecked(1.0, 15);
        }
        // The Pace-split IP studies: decaying (Active), flat (Passive), growing
        // (Idle) over the current infinity.
        let infinity_secs = self.records.this_infinity.time_ms / 1000.0;
        if self.time_study_bought(141) {
            let decay = Self::this_infinity_mult(infinity_secs);
            mult *= (Decimal::new_unchecked(1.0, 45) / decay).max(&Decimal::ONE);
        }
        if self.time_study_bought(142) {
            mult *= Decimal::new_unchecked(1.0, 25);
        }
        if self.time_study_bought(143) {
            mult *= Self::this_infinity_mult(infinity_secs);
        }
        mult
    }

    /// Infinity Points a Big Crunch would grant right now. Mirrors
    /// `gainedInfinityPoints`: pre-break the base is `308 / div` (= 1 with
    /// `div = 308`); once Infinity is broken it scales as
    /// `10 ^ (log10(thisInfinity.maxAM) / div - 0.75)`. Times `totalIPMult` (= 1),
    /// floored.
    pub fn gained_infinity_points(&self) -> Decimal {
        let div = self.ip_gain_divisor();
        let base = if self.broke_infinity {
            let exponent = self.records.this_infinity.max_am.log10() / div - 0.75;
            Decimal::pow10(exponent)
        } else {
            Decimal::from_float(308.0 / div)
        };
        (base * self.total_ip_mult()).floor()
    }

    /// Infinities a Big Crunch would grant right now. Mirrors `gainedInfinities`:
    /// base 1 (Achievement 87 is post-Reality) times TS32's Dimension-Boost
    /// multiplier.
    pub fn gained_infinities(&self) -> Decimal {
        // EC4 disables the Infinity generators (`gainedInfinities` returns 1).
        if self.ec_running(4) {
            return Decimal::ONE;
        }
        let mut gain = Decimal::ONE;
        if self.time_study_bought(32) {
            gain *= Decimal::from((self.dim_boosts as u64).max(1));
        }
        gain
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

    /// Whether the player can Break Infinity now: the Big Crunch autobuyer's
    /// interval is at its 100 ms floor (`BreakInfinityButton.isUnlocked`) and
    /// Infinity is not already broken.
    pub fn can_break_infinity(&self) -> bool {
        !self.broke_infinity && self.break_infinity_unlockable()
    }

    /// Break Infinity: lift the `1e308` antimatter cap and switch to the scaling
    /// IP formula. One-way pre-Eternity. Returns whether it happened.
    pub fn break_infinity(&mut self) -> bool {
        if !self.can_break_infinity() {
            return false;
        }
        self.broke_infinity = true;
        // Breaking Infinity points the player at the Infinity Challenges tab
        // (mirrors game.js `breakInfinity`).
        self.try_trigger_tab_notification(TabNotificationId::IcUnlock);
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
            // The first-ever Infinity badges the tabs it opens up (the original's
            // BIG_CRUNCH_BEFORE event, dispatched only when at the goal; the
            // trigger's condition is "Infinity not yet unlocked").
            self.try_trigger_tab_notification(TabNotificationId::FirstInfinity);

            // 21: "To infinity!" — unlocks on the crunch itself (the original's
            // BIG_CRUNCH_BEFORE), so the post-reset starting antimatter already
            // reflects its 100-antimatter reward.
            self.unlock_achievement(21);

            // Award rewards from the pre-reset state (IP reads `thisInfinity.maxAM`
            // once Break Infinity lands; both persist across the crunch).
            self.infinity_points += self.gained_infinity_points();
            self.infinities += self.gained_infinities();
            // The IP setter tracks the eternity's IP peak (drives the Eternity
            // goal and the EP formula).
            self.records.this_eternity.max_ip =
                self.records.this_eternity.max_ip.max(&self.infinity_points);

            // Record the running Infinity Challenge's fastest completion
            // (`bigCrunchUpdateStatistics` → `challenge.infinity.bestTimes`),
            // then complete the running challenge, or NC1 on the first Infinity
            // performed outside a challenge (mirrors `handleChallengeCompletion`).
            let ic = self.infinity_challenge.current;
            if ic != 0 {
                let slot = &mut self.ic_best_times_ms[(ic - 1) as usize];
                *slot = slot.min(self.records.this_infinity.time_ms);
            }
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
        // Reset Infinity Power and each Infinity Dimension's amount to its bought
        // base (`InfinityDimensions.resetAmount`); purchases/cost/unlock persist.
        self.reset_infinity_dimension_amounts();
        // Replicanti reset (`secondSoftReset` + `bigCrunchResetValues`): amount
        // back to 1 and Replicanti Galaxies to 0 — except Achievement 95 keeps
        // the amount (and 1 RG) and TS33 keeps half the RGs.
        let current_replicanti = self.replicanti.amount;
        let current_rgs = self.replicanti.galaxies;
        if self.replicanti.unlocked {
            self.replicanti.amount = Decimal::ONE;
        }
        let mut remaining_rgs = 0;
        if self.achievement_unlocked(95) {
            self.replicanti.amount = current_replicanti;
            remaining_rgs += current_rgs.min(1);
        }
        if self.time_study_bought(33) {
            remaining_rgs += current_rgs / 2;
        }
        self.replicanti.galaxies = remaining_rgs.min(current_rgs);
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

        // EC4's Infinity limit is checked on each crunch
        // (`bigCrunchCheckUnlocks` → `EternityChallenge(4).tryFail()`).
        self.ec_try_fail(4);

        // Replicanti-affordable badge (the original's BIG_CRUNCH_AFTER event,
        // dispatched at the end of every reset, forced ones included; the
        // trigger's condition is IP >= 1e140).
        self.try_trigger_tab_notification(TabNotificationId::Replicanti);
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

    #[test]
    fn break_infinity_requires_maxed_big_crunch_interval() {
        use crate::AutobuyerTarget;
        let mut game = GameState::new();
        assert!(!game.can_break_infinity());
        assert!(!game.break_infinity());

        // Complete NC12 and upgrade the Big Crunch autobuyer to the 100 ms floor.
        game.complete_challenge(12);
        assert!(!game.can_break_infinity()); // interval still 150 s
        game.infinity_points = Decimal::from_float(1e9);
        for _ in 0..50 {
            game.upgrade_autobuyer_interval(AutobuyerTarget::BigCrunch);
        }
        assert!(game.can_break_infinity());
        assert!(game.break_infinity());
        assert!(game.broke_infinity);
        // One-way: cannot break again.
        assert!(!game.can_break_infinity());
    }

    #[test]
    fn post_break_ip_scales_with_max_am() {
        let mut game = GameState::new();
        game.records.this_infinity.max_am = Decimal::new(1.0, 616);
        // Pre-break the crunch is always worth exactly 1 IP.
        assert_eq!(game.gained_infinity_points(), Decimal::ONE);

        // Post-break at 1e616: 10^(616/308 - 0.75) = 10^1.25 ≈ 17.78 → floor 17.
        game.broke_infinity = true;
        assert_eq!(game.gained_infinity_points(), Decimal::from_float(17.0));
    }

    #[test]
    fn broke_infinity_persists_across_crunch() {
        let mut game = GameState::new();
        game.broke_infinity = true;
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert!(game.broke_infinity);
    }

    #[test]
    fn post_break_lifts_antimatter_cap_except_in_challenges() {
        // Outside a challenge, post-break antimatter grows past 1e308.
        let mut game = GameState::new();
        game.broke_infinity = true;
        game.dimensions[0].amount = Decimal::new(1.0, 400);
        game.antimatter = BIG_CRUNCH_THRESHOLD * Decimal::from_float(0.9);
        game.tick(1000.0);
        assert!(game.antimatter > BIG_CRUNCH_THRESHOLD);

        // Inside a normal challenge the 1e308 cap still holds, even post-break.
        let mut game = GameState::new();
        game.broke_infinity = true;
        game.infinity_unlocked = true;
        game.start_challenge(2);
        game.dimensions[0].amount = Decimal::new(1.0, 400);
        game.antimatter = BIG_CRUNCH_THRESHOLD * Decimal::from_float(0.9);
        game.tick(1000.0);
        assert_eq!(game.antimatter, BIG_CRUNCH_THRESHOLD);
    }
}
