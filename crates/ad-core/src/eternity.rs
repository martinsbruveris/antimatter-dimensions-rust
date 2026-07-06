//! Eternity (Feature 4.1): the second prestige layer. When Infinity Points
//! reach `1.8e308` (`Decimal.NUMBER_MAX_VALUE`), the player can Eternity —
//! resetting the whole Infinity layer — for Eternity Points.
//!
//! Mirrors `src/core/eternity.js` (`eternity`, `giveEternityRewards`,
//! `initializeChallengeCompletions`, `initializeResourcesAfterEternity`) and
//! `game.js` (`gainedEternityPoints`, `playerInfinityUpgradesOnReset`). The
//! Eternity-Milestone keeps are in `eternity_milestones.rs`. See
//! `docs/design/2026-07-04-eternity.md`.

use break_infinity::Decimal;

use crate::data::constants::{
    AD_AUTOBUYER_INTERVALS_MS, BIG_CRUNCH_AUTOBUYER_INTERVAL_MS,
    DIM_BOOST_AUTOBUYER_INTERVAL_MS, GALAXY_AUTOBUYER_INTERVAL_MS,
    TICKSPEED_AUTOBUYER_INTERVAL_MS,
};
use crate::infinity_dimensions::InfinityDimension;
use crate::records::{ThisEternity, ThisInfinity};
use crate::replicanti::ReplicantiState;
use crate::state::{DimensionTier, GameState, TickspeedState};
use crate::AutobuyerMode;

/// The Eternity goal outside an Eternity Challenge: `Decimal.NUMBER_MAX_VALUE`
/// Infinity Points.
pub const ETERNITY_GOAL: Decimal = Decimal::NUMBER_MAX_VALUE;

/// `bestInfinity.time` is reset to this (not `Number.MAX_VALUE`) by an Eternity
/// — a quirk of the original's `initializeResourcesAfterEternity`.
const BEST_INFINITY_RESET_MS: f64 = 999_999_999_999.0;

impl GameState {
    /// The Infinity-Point goal for an Eternity (`Player.eternityGoal`): the
    /// running Eternity Challenge's scaled goal, else [`ETERNITY_GOAL`].
    pub fn eternity_goal(&self) -> Decimal {
        if self.any_ec_running() {
            self.ec_current_goal(self.eternity_challenge_current)
        } else {
            ETERNITY_GOAL
        }
    }

    /// Whether the player can Eternity now (`Player.canEternity`): the peak IP
    /// this eternity has reached the goal. Peak, not current, so spending IP
    /// doesn't revoke it.
    pub fn can_eternity(&self) -> bool {
        self.records.this_eternity.max_ip >= self.eternity_goal()
    }

    /// The global Eternity-Point multiplier (`totalEPMult`): the rebuyable
    /// `epMult` Eternity Upgrade and Time Studies 61/121/122/123.
    pub(crate) fn total_ep_mult(&self) -> Decimal {
        let mut mult = self.ep_mult_effect();
        if self.time_study_bought(61) {
            mult *= Decimal::from_float(15.0);
        }
        // The Pace-split EP studies (mutually exclusive columns).
        if self.time_study_bought(121) {
            mult *= Decimal::from_float(self.ts121_effect());
        }
        if self.time_study_bought(122) {
            // The PASS perk (31) improves TS122 to ×50.
            mult *= Decimal::from_float(if self.perk_bought(31) { 50.0 } else { 35.0 });
        }
        if self.time_study_bought(123) {
            // The IDL perk (71): the Idle path starts 15 minutes in.
            let secs = self.records.this_eternity.time_ms / 1000.0
                + if self.perk_bought(71) { 900.0 } else { 0.0 };
            mult *= Decimal::from_float((1.39 * secs).sqrt());
        }
        // The `timeEP` glyph effect (`GlyphEffect.epMult`).
        mult *= Decimal::from_float(self.glyph_effect_time_ep());
        // RU12: EP multiplier from TT and Reality count.
        if self.reality_upgrade_bought(12) {
            let realities = (self.reality.realities as f64).min(1e4);
            if realities >= 1.0 {
                let base = (self.time_theorems - Decimal::from_float(1000.0))
                    .max(&Decimal::from_float(2.0));
                mult *= base
                    .pow(&Decimal::from_float(realities.log2()))
                    .max(&Decimal::ONE);
            }
        }
        mult
    }

    /// Eternity Points an Eternity would grant right now. Mirrors
    /// `gainedEternityPoints`:
    /// `5 ^ (log10(thisEternity.maxIP + gainedInfinityPoints()) / 308 - 0.7)`
    /// times `totalEPMult`, floored. Note the pending crunch IP counts.
    pub fn gained_eternity_points(&self) -> Decimal {
        let ip = self.records.this_eternity.max_ip + self.gained_infinity_points();
        let exponent = ip.log10() / 308.0 - 0.7;
        let base = Decimal::from_float(5.0).pow(&Decimal::from_float(exponent));
        let mut ep = base * self.total_ep_mult();
        // Celestial run modifiers: Teresa `^0.55`, V `^0.5`.
        if self.celestials.teresa.run {
            ep = ep.pow(&Decimal::from_float(0.55));
        } else if self.celestials.v.run {
            ep = ep.pow(&Decimal::from_float(0.5));
        }
        ep.floor()
    }

    /// Eternities an Eternity would grant (`gainedEternities`): the
    /// `timeetermult` glyph effect times Reality Upgrade 3 (Achievement 113
    /// is out of frontier).
    pub fn gained_eternities(&self) -> Decimal {
        let mut gain = Decimal::from_float(self.glyph_effect_timeetermult());
        // RU3 (Eternal Amplifier): ×3 per purchase.
        gain *= self.reality_rebuyable_effect(3);
        gain
    }

    /// Perform an Eternity: award EP / an Eternity, then reset the whole
    /// Infinity layer. Returns whether it happened.
    pub fn eternity(&mut self) -> bool {
        self.eternity_with_options(false)
    }

    /// The rewarded Eternity, with the `enteringEC` special condition (respec
    /// suppressed) used when starting an Eternity Challenge from the goal.
    pub(crate) fn eternity_with_options(&mut self, entering_ec: bool) -> bool {
        if !self.can_eternity() {
            return false;
        }

        // ETERNITY_RESET_BEFORE requirement checks (RU6/8/10), before the
        // rewards clear the `noEternities` flag.
        self.check_reality_upgrade_reqs_on_eternity_before();

        // Rewards (`giveEternityRewards`), read from the pre-reset state.
        self.records.best_eternity.time_ms = self
            .records
            .best_eternity
            .time_ms
            .min(self.records.this_eternity.time_ms);
        self.records.best_eternity.real_time_ms = self
            .records
            .best_eternity
            .real_time_ms
            .min(self.records.this_eternity.real_time_ms);
        let gained_ep = self.gained_eternity_points();
        let gained_eternities = self.gained_eternities();
        self.eternity_points += gained_ep;
        self.records.this_reality.max_ep =
            self.records.this_reality.max_ep.max(&self.eternity_points);
        self.records.best_reality.best_ep =
            self.records.best_reality.best_ep.max(&self.eternity_points);
        self.eternities += gained_eternities;
        self.eternity_unlocked = true;
        // `player.requirementChecks.reality.noEternities = false` (any
        // rewarded Eternity spoils Reality Upgrade 6/10's requirement).
        self.requirement_checks.reality_no_eternities = false;

        // A running Eternity Challenge banks a completion (which auto-respecs
        // the study tree and consumes the study slot). The banked count feeds
        // the Automator's event log (`AutomatorData.lastECCompletionCount`).
        self.automator.runtime.last_ec_completions = 0;
        if self.any_ec_running() {
            let ec = self.eternity_challenge_current;
            let before = self.eternity_challenge_completions(ec);
            self.complete_running_ec();
            self.automator.runtime.last_ec_completions =
                self.eternity_challenge_completions(ec) - before;
        }

        // TS191: bank 5% of the Infinities on each Eternity (Achievement 131's
        // extra share is a later feature).
        if self.time_study_bought(191) {
            self.infinities_banked +=
                (self.infinities * Decimal::from_float(0.05)).floor();
        }

        // `addEternityTime`: push this run onto the last-10-eternities ring.
        self.records.recent_eternities.pop();
        self.records.recent_eternities.insert(
            0,
            crate::records::RecentEternity {
                time_ms: self.records.this_eternity.time_ms,
                real_time_ms: self.records.this_eternity.real_time_ms,
                ep: gained_ep,
                eternities: gained_eternities,
            },
        );

        self.eternity_full_reset(entering_ec);

        // ETERNITY_RESET_AFTER requirement checks (RU9/12/13/15/25) — the
        // awarded EP persists through the reset.
        self.check_reality_upgrade_reqs_on_eternity_after();

        // The Automator's ETERNITY_RESET_AFTER notification (`prestigeNotify`).
        self.automator_notify_prestige(
            crate::automator::PrestigeLayer::Eternity,
            gained_ep,
        );

        // `giveEternityRewards`: completing Effarig's Eternity stage inside the
        // run unlocks it and forces a reward-free Reality exit.
        self.effarig_on_eternity();
        true
    }

    /// The unrewarded (forced) Eternity reset: what an EC exit performs.
    pub(crate) fn eternity_reset(&mut self) {
        self.eternity_full_reset(false);
    }

    /// The full Eternity reset (`eternity()`'s reset half): the shared layer
    /// reset plus the pieces exclusive to a *real* Eternity — the autobuyer /
    /// Break-Infinity handling and the respec.
    fn eternity_full_reset(&mut self, entering_ec: bool) {
        // A dilated run banks its Tachyon Particles on any Eternity reset
        // (`if (player.dilation.active) rewardTP()`; the reward itself is
        // gated on the Eternity goal), then ends — unless entering an EC.
        if self.dilation.active {
            self.reward_tp();
            if !entering_ec {
                self.dilation.active = false;
            }
        }

        self.eternity_challenge_current = 0;

        // Without the keepAutobuyers milestone, Infinity un-breaks (it can only
        // re-break after the Big Crunch autobuyer interval is maxed again).
        if !self.eternity_milestone_reached(2) {
            self.broke_infinity = false;
        }
        self.reset_autobuyers_on_eternity();
        // The prestige autobuyers' config resets are unconditional parts of
        // `Autobuyers.reset()` (their milestone checks live inside).
        self.reset_prestige_autobuyer_configs();

        self.eternity_reset_core();

        // Respec the study tree if the player ticked the box (`player.respec`);
        // suppressed when the reset enters an Eternity Challenge.
        if !entering_ec && self.respec {
            self.respec_time_studies_now();
            self.respec = false;
        }
    }

    /// The layer reset shared by an Eternity and `startEternityChallenge()`.
    pub(crate) fn eternity_reset_core(&mut self) {
        // `initializeChallengeCompletions`: completions cleared; with the
        // keepAutobuyers milestone all Normal Challenges come back completed
        // (which re-grants the autobuyer rewards).
        self.challenge.completed = 0;
        self.infinity_challenge.completed = 0;
        self.challenge.current = 0;
        self.infinity_challenge.current = 0;
        // (`challenge.eternity.current` is handled by the callers: cleared by a
        // real Eternity, restored by `startEternityChallenge`.)
        if self.eternity_milestone_reached(2) {
            for id in 1..=crate::NORMAL_CHALLENGE_COUNT {
                self.complete_challenge(id);
            }
        }

        // `initializeResourcesAfterEternity`.
        self.sacrificed = Decimal::ZERO;
        self.infinities = Decimal::ZERO;
        self.records.best_infinity.time_ms = BEST_INFINITY_RESET_MS;
        self.records.best_infinity.real_time_ms = BEST_INFINITY_RESET_MS;
        self.records.this_infinity = ThisInfinity::new();
        let keep_infinity_upgrades = self.eternity_milestone_reached(4);
        self.dim_boosts = if keep_infinity_upgrades { 4 } else { 0 };
        self.galaxies = if keep_infinity_upgrades { 1 } else { 0 };
        self.part_infinity_point = 0.0;
        self.infinity_power = Decimal::ZERO;
        self.time_shards = Decimal::ZERO;
        self.total_tick_gained = 0;
        self.eterc8_ids = 50;
        self.eterc8_repl = 40;
        self.records.this_eternity = ThisEternity::new();

        // Infinity Dimensions *full* reset (purchases/costs/unlocks too).
        self.infinity_dimensions = std::array::from_fn(InfinityDimension::new);

        // `Replicanti.reset()`: back to defaults; stays unlocked (amount 1) with
        // the unlockReplicanti milestone.
        let keep_replicanti = self.eternity_milestone_reached(10);
        self.replicanti = ReplicantiState {
            unlocked: keep_replicanti,
            amount: if keep_replicanti {
                Decimal::ONE
            } else {
                Decimal::ZERO
            },
            ..ReplicantiState::new()
        };

        // `resetChallengeStuff` + per-run challenge counters.
        self.reset_challenge_stuff();
        self.post_c4_tier = 1;
        self.ic2_count = 0.0;

        // Antimatter Dimensions + Tickspeed reset.
        self.dimensions = std::array::from_fn(|_| DimensionTier::new());
        self.tickspeed = TickspeedState::new();

        // Time Dimension amounts return to the bought base; purchases persist
        // (`resetTimeDimensions`).
        self.reset_time_dimension_amounts();

        // IP reset — to the starting value (the START perks), which also sets
        // the eternity's IP peak (the original `Currency.infinityPoints
        // .reset()` writes `thisEternity.maxIP = startingValue`).
        self.infinity_points = self.starting_ip();
        self.records.this_eternity.max_ip = self.infinity_points;

        // `playerInfinityUpgradesOnReset` (milestone-aware).
        self.reset_infinity_upgrades_on_eternity();

        // `Player.resetRequirements("eternity")`.
        self.requirement_checks.eternity_no_rg = true;

        self.antimatter = self.starting_antimatter();
    }

    /// `playerInfinityUpgradesOnReset`: with the keepBreakUpgrades milestone
    /// (8 eternities) you start with *all* Infinity + Break Infinity Upgrades
    /// and maxed rebuyables; with keepInfinityUpgrades (4) all 16 Infinity
    /// Upgrades only; below that everything is cleared.
    fn reset_infinity_upgrades_on_eternity(&mut self) {
        let all_infinity: u32 = crate::ALL_INFINITY_UPGRADES
            .iter()
            .fold(0, |bits, u| bits | u.bit());
        let all_break: u32 = crate::ALL_BREAK_INFINITY_UPGRADES
            .iter()
            .fold(0, |bits, u| bits | u.bit());
        if self.eternity_milestone_reached(8) {
            self.infinity_upgrades = all_infinity;
            self.break_infinity_upgrades = all_break;
            self.infinity_rebuyables = [8, 7, 10];
        } else if self.eternity_milestone_reached(4) {
            self.infinity_upgrades = all_infinity;
            self.break_infinity_upgrades = 0;
            self.infinity_rebuyables = [0, 0, 0];
        } else {
            self.infinity_upgrades = 0;
            self.break_infinity_upgrades = 0;
            self.infinity_rebuyables = [0, 0, 0];
        }
    }

    /// `Autobuyers.reset()` on ETERNITY_RESET_AFTER: unless the keepAutobuyers
    /// milestone is reached, the AD/Tickspeed autobuyers lose their antimatter
    /// unlock and every upgradeable autobuyer's interval/cost return to base
    /// (the tickspeed autobuyer's mode also resets to singles).
    fn reset_autobuyers_on_eternity(&mut self) {
        if self.eternity_milestone_reached(2) {
            return;
        }
        self.reset_autobuyers_for_prestige();
    }

    /// The unconditional autobuyer reset shared by the Eternity path (behind
    /// the keepAutobuyers milestone) and the Reality reset (always).
    pub(crate) fn reset_autobuyers_for_prestige(&mut self) {
        for (tier, ab) in self.autobuyers.dimensions.iter_mut().enumerate() {
            ab.is_bought = false;
            ab.interval_ms = AD_AUTOBUYER_INTERVALS_MS[tier];
            ab.cost = 1.0;
            ab.timer_ms = 0.0;
        }
        let ts = &mut self.autobuyers.tickspeed;
        ts.is_bought = false;
        ts.mode = AutobuyerMode::BuySingle;
        ts.interval_ms = TICKSPEED_AUTOBUYER_INTERVAL_MS;
        ts.cost = 1.0;
        ts.timer_ms = 0.0;
        for (ab, base) in [
            (
                &mut self.autobuyers.dim_boost,
                DIM_BOOST_AUTOBUYER_INTERVAL_MS,
            ),
            (&mut self.autobuyers.galaxy, GALAXY_AUTOBUYER_INTERVAL_MS),
            (
                &mut self.autobuyers.big_crunch,
                BIG_CRUNCH_AUTOBUYER_INTERVAL_MS,
            ),
        ] {
            ab.interval_ms = base;
            ab.cost = 1.0;
            ab.timer_ms = 0.0;
        }
        self.clear_tab_notification_trigger(
            crate::tab_notifications::TabNotificationId::NewAutobuyer,
        );
    }

    /// `updatePrestigeRates` (game.js): track the peak IP/min while a crunch is
    /// possible and the peak EP/min while an Eternity is possible. Called once
    /// per tick; the rates back the header prestige buttons.
    pub(crate) fn update_prestige_rates(&mut self) {
        let ip_minutes =
            (self.records.this_infinity.real_time_ms / 60_000.0).max(0.000_5);
        let gained_ip = self.gained_infinity_points();
        let current_ip_min = gained_ip / Decimal::from_float(ip_minutes);
        if current_ip_min > self.records.this_infinity.best_ip_min
            && self.can_big_crunch()
        {
            self.records.this_infinity.best_ip_min = current_ip_min;
            self.records.this_infinity.best_ip_min_val = gained_ip;
        }

        let ep_minutes =
            (self.records.this_eternity.real_time_ms / 60_000.0).max(0.000_5);
        let gained_ep = self.gained_eternity_points();
        let current_ep_min = gained_ep / Decimal::from_float(ep_minutes);
        if current_ep_min > self.records.this_eternity.best_ep_min && self.can_eternity()
        {
            self.records.this_eternity.best_ep_min = current_ep_min;
            self.records.this_eternity.best_ep_min_val = gained_ep;
        }
    }

    /// Whether the milestone requiring `eternities` Eternities is reached
    /// (`EternityMilestoneState.isReached`). The full milestone catalogue lives
    /// in `eternity_milestones.rs` (Feature 4.2).
    pub fn eternity_milestone_reached(&self, eternities: u64) -> bool {
        self.eternities >= Decimal::from(eternities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::constants::BIG_CRUNCH_THRESHOLD;

    /// A state at the Eternity goal with some Infinity-layer progress to reset.
    fn game_at_eternity_goal() -> GameState {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.broke_infinity = true;
        game.infinity_points = ETERNITY_GOAL;
        game.records.this_eternity.max_ip = ETERNITY_GOAL;
        game.records.this_eternity.max_am = Decimal::new(1.0, 5000);
        game.infinities = Decimal::from_float(1e6);
        game.dim_boosts = 20;
        game.galaxies = 5;
        game.replicanti.unlocked = true;
        game.replicanti.amount = Decimal::new(1.0, 100);
        game.replicanti.galaxies = 3;
        game
    }

    #[test]
    fn cannot_eternity_below_goal() {
        let mut game = GameState::new();
        game.infinity_points = Decimal::new(1.0, 300);
        game.records.this_eternity.max_ip = game.infinity_points;
        assert!(!game.can_eternity());
        assert!(!game.eternity());
    }

    #[test]
    fn eternity_at_goal_awards_ep_and_resets() {
        let mut game = game_at_eternity_goal();
        assert!(game.can_eternity());

        // At exactly 1.8e308 max IP the formula gives 5^(log10(maxIP+1)/308-0.7)
        // ≈ 5^0.30086 ≈ 1.6 → floor 1.
        assert_eq!(game.gained_eternity_points(), Decimal::ONE);
        assert!(game.eternity());

        assert_eq!(game.eternity_points, Decimal::ONE);
        assert_eq!(game.eternities, Decimal::ONE);
        assert!(game.eternity_unlocked);

        // The Infinity layer is gone.
        assert_eq!(game.infinity_points, Decimal::ZERO);
        assert_eq!(game.infinities, Decimal::ZERO);
        assert_eq!(game.dim_boosts, 0);
        assert_eq!(game.galaxies, 0);
        assert!(!game.broke_infinity);
        assert!(!game.replicanti.unlocked);
        assert_eq!(game.replicanti.galaxies, 0);
        assert_eq!(game.records.this_eternity.max_am, Decimal::ZERO);
        assert_eq!(game.records.this_eternity.max_ip, Decimal::ZERO);
        assert_eq!(game.records.best_infinity.time_ms, BEST_INFINITY_RESET_MS);
        assert!(!game.can_eternity());

        // EP and the eternities count persist (only Reality resets them).
        assert_eq!(game.eternity_points, Decimal::ONE);
    }

    #[test]
    fn ep_scales_with_ip_past_the_goal() {
        let mut game = game_at_eternity_goal();
        // 5^(616/308 - 0.7) = 5^1.3 ≈ 8.1 → floor 8.
        game.records.this_eternity.max_ip = Decimal::new(1.0, 616);
        assert_eq!(game.gained_eternity_points(), Decimal::from_float(8.0));
    }

    #[test]
    fn eternity_clears_challenges_and_upgrades_pre_milestones() {
        let mut game = game_at_eternity_goal();
        game.challenge.completed = 0b1111_1111_1110;
        game.infinity_challenge.completed = 0b1_1111_1110;
        game.infinity_upgrades = crate::ALL_INFINITY_UPGRADES
            .iter()
            .fold(0, |b, u| b | u.bit());
        game.infinity_rebuyables = [3, 2, 1];
        game.autobuyers.dimensions[0].is_bought = true;
        game.autobuyers.tickspeed.is_bought = true;

        assert!(game.eternity());

        assert_eq!(game.challenge.completed, 0);
        assert_eq!(game.infinity_challenge.completed, 0);
        assert_eq!(game.infinity_upgrades, 0);
        assert_eq!(game.infinity_rebuyables, [0, 0, 0]);
        assert!(!game.autobuyers.dimensions[0].is_bought);
        assert!(!game.autobuyers.tickspeed.is_bought);
    }

    #[test]
    fn milestones_keep_autobuyers_upgrades_and_replicanti() {
        let mut game = game_at_eternity_goal();
        game.eternities = Decimal::from_float(10.0);
        game.autobuyers.dimensions[0].interval_ms = 100.0;

        assert!(game.eternity());

        // keepAutobuyers (2): NCs completed → autobuyers unlocked, break kept.
        assert!(game.challenge_completed(12));
        assert!(game.autobuyers.dimensions[0].is_bought);
        assert_eq!(game.autobuyers.dimensions[0].interval_ms, 100.0);
        assert!(game.broke_infinity);
        // keepInfinityUpgrades (4): all 16 upgrades + 4 boosts + 1 galaxy.
        assert_ne!(game.infinity_upgrades, 0);
        assert_eq!(game.dim_boosts, 4);
        assert_eq!(game.galaxies, 1);
        // keepBreakUpgrades (8): break upgrades + maxed rebuyables.
        assert_ne!(game.break_infinity_upgrades, 0);
        assert_eq!(game.infinity_rebuyables, [8, 7, 10]);
        // unlockReplicanti (10): replicanti start unlocked at amount 1.
        assert!(game.replicanti.unlocked);
        assert_eq!(game.replicanti.amount, Decimal::ONE);
    }

    #[test]
    fn tick_tracks_eternity_time_and_ip_peak() {
        let mut game = GameState::new();
        game.infinity_points = Decimal::new(1.0, 100);
        game.tick(1000.0);
        assert_eq!(game.records.this_eternity.time_ms, 1000.0);
        assert_eq!(game.records.this_eternity.real_time_ms, 1000.0);
        assert_eq!(game.records.this_eternity.max_ip, Decimal::new(1.0, 100));
    }

    #[test]
    fn crunch_ip_counts_toward_ip_peak() {
        // A Big Crunch's IP award raises the eternity IP peak immediately.
        let mut game = GameState::new();
        game.broke_infinity = true;
        game.records.this_infinity.max_am = Decimal::new(1.0, 616);
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert_eq!(game.records.this_eternity.max_ip, game.infinity_points);
    }

    #[test]
    fn spending_ip_does_not_revoke_eternity() {
        let mut game = game_at_eternity_goal();
        game.infinity_points = Decimal::ZERO; // spent it all
        assert!(game.can_eternity());
    }
}
