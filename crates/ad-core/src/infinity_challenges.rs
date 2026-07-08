//! Infinity Challenges: 8 constrained runs gated by antimatter thresholds that
//! all exceed `1e308` (so they need Break Infinity). Structurally like the Normal
//! Challenges (2.5) but with a per-challenge *goal* rather than the fixed `1e308`.
//! This module owns the run state machine ([`InfinityChallengeState`] on
//! [`GameState`]) and the unlock/start/exit/complete logic; the per-challenge rule
//! modifiers live at their engine sites. See
//! `docs/design/2026-07-03-infinity-challenges.md`.

use break_infinity::Decimal;

use crate::state::GameState;

/// Number of infinity challenges.
pub const INFINITY_CHALLENGE_COUNT: u8 = 8;

/// Antimatter goal of each infinity challenge (reach it, then crunch, to complete),
/// indexed by `id - 1`. From `infinity-challenges.js`.
const IC_GOALS: [Decimal; 8] = [
    Decimal::new_unchecked(1.0, 650),
    Decimal::new_unchecked(1.0, 10500),
    Decimal::new_unchecked(1.0, 5000),
    Decimal::new_unchecked(1.0, 13000),
    Decimal::new_unchecked(1.0, 16500),
    Decimal::new_unchecked(2.0, 22222),
    Decimal::new_unchecked(1.0, 10000),
    Decimal::new_unchecked(1.0, 27000),
];

/// `thisEternity.maxAM` required to unlock each infinity challenge, indexed by
/// `id - 1` (`unlockAM`).
const IC_UNLOCK_AM: [Decimal; 8] = [
    Decimal::new_unchecked(1.0, 2000),
    Decimal::new_unchecked(1.0, 11000),
    Decimal::new_unchecked(1.0, 12000),
    Decimal::new_unchecked(1.0, 14000),
    Decimal::new_unchecked(1.0, 18000),
    Decimal::new_unchecked(1.0, 22500),
    Decimal::new_unchecked(1.0, 23000),
    Decimal::new_unchecked(1.0, 28000),
];

/// Infinity-challenge run state. Mirrors `player.challenge.infinity`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InfinityChallengeState {
    /// Active challenge id (`0` = none, `1..=8` = running that challenge).
    pub current: u8,
    /// Completed-challenge bitmask (bit `1 << id`).
    pub completed: u16,
}

impl GameState {
    /// The goal of infinity challenge `id` (0 for an invalid id).
    pub fn infinity_challenge_goal(id: u8) -> Decimal {
        if (1..=INFINITY_CHALLENGE_COUNT).contains(&id) {
            IC_GOALS[(id - 1) as usize]
        } else {
            Decimal::ZERO
        }
    }

    /// The `thisEternity.maxAM` needed to unlock infinity challenge `id`
    /// (`unlockAM`; 0 for an invalid id).
    pub fn infinity_challenge_unlock_am(id: u8) -> Decimal {
        if (1..=INFINITY_CHALLENGE_COUNT).contains(&id) {
            IC_UNLOCK_AM[(id - 1) as usize]
        } else {
            Decimal::ZERO
        }
    }

    /// Whether infinity challenge `id` is the one currently running.
    pub fn infinity_challenge_running(&self, id: u8) -> bool {
        self.infinity_challenge.current == id
    }

    /// Whether any infinity challenge is currently running.
    pub fn any_infinity_challenge_running(&self) -> bool {
        self.infinity_challenge.current != 0
    }

    /// Whether the player is in *any* antimatter challenge (normal or infinity) —
    /// the original's `Player.isInAntimatterChallenge`, which the Big-Crunch cap
    /// and `skip_resets_if_possible` gate on.
    pub fn in_any_antimatter_challenge(&self) -> bool {
        self.any_challenge_running() || self.any_infinity_challenge_running()
    }

    /// Whether infinity challenge `id` has been completed.
    pub fn infinity_challenge_completed(&self, id: u8) -> bool {
        (1..=INFINITY_CHALLENGE_COUNT).contains(&id)
            && self.infinity_challenge.completed & (1u16 << id) != 0
    }

    /// Whether infinity challenge `id` is unlocked: the peak antimatter this
    /// eternity has reached its `unlockAM`.
    pub fn infinity_challenge_unlocked(&self, id: u8) -> bool {
        if !(1..=INFINITY_CHALLENGE_COUNT).contains(&id) {
            return false;
        }
        self.records.this_eternity.max_am >= IC_UNLOCK_AM[(id - 1) as usize]
    }

    /// Whether any infinity challenge is unlocked (the subtab gate).
    pub fn infinity_challenges_unlocked(&self) -> bool {
        (1..=INFINITY_CHALLENGE_COUNT).any(|id| self.infinity_challenge_unlocked(id))
    }

    /// Whether infinity challenge `id` can be started now: unlocked and not already
    /// the running one.
    pub fn can_start_infinity_challenge(&self, id: u8) -> bool {
        self.infinity_challenge_unlocked(id) && self.infinity_challenge.current != id
    }

    /// Start infinity challenge `id`: a forced Big Crunch (rewards if at goal),
    /// clear any normal challenge, enter the IC, and **break Infinity** (the goal
    /// is unreachable otherwise). Mirrors `InfinityChallengeState.start`.
    pub fn start_infinity_challenge(&mut self, id: u8) -> bool {
        if !self.can_start_infinity_challenge(id) {
            return false;
        }
        // 115: start an Infinity Challenge inside an Eternity Challenge (checked
        // before the reset clears the running EC — matching the original's
        // ACHIEVEMENT_EVENT_OTHER unlock in `startInfinityChallenge`).
        if self.any_ec_running() {
            self.unlock_achievement(115);
        }
        self.big_crunch_reset(true, true);
        self.challenge.current = 0;
        self.infinity_challenge.current = id;
        self.broke_infinity = true;
        true
    }

    /// Mark infinity challenge `id` completed. Idempotent.
    pub(crate) fn complete_infinity_challenge(&mut self, id: u8) {
        if (1..=INFINITY_CHALLENGE_COUNT).contains(&id) {
            self.infinity_challenge.completed |= 1u16 << id;
            // INFINITY_CHALLENGE_COMPLETED achievements (67, 82).
            self.check_infinity_challenge_completed_achievements();
        }
    }

    // --- Effect readers (per the original's `InfinityChallenge(N)` sites) ------

    /// Whether Tickspeed is neutralised (its production effect → 1): IC3 while
    /// running (`getTickSpeedMultiplier` returns 1). Read in `tickspeed_effect`.
    pub(crate) fn ic3_neutralizes_tickspeed(&self) -> bool {
        self.infinity_challenge_running(3)
    }

    /// The all-tier Antimatter Dimension multiplier from Infinity Challenges,
    /// folded into `antimatterDimensionCommonMultiplier`:
    /// - IC3 static `(1.05 + galaxies·0.005)^totalTickBought` (applied once while
    ///   running and again as the IC3 completion reward — the original multiplies
    ///   both effects);
    /// - IC8 production decay `0.8446303389034288^(time − lastBuyTime)` while
    ///   running;
    /// - divided by IC6's rising `matter` (≥ 1) while running.
    pub(crate) fn infinity_challenge_common_mult(&self) -> Decimal {
        let mut mult = Decimal::ONE;

        if self.infinity_challenge_running(3) || self.infinity_challenge_completed(3) {
            let base = Decimal::from_float(1.05 + self.galaxies as f64 * 0.005)
                .pow(&Decimal::from(self.tickspeed.bought));
            if self.infinity_challenge_running(3) {
                mult *= base;
            }
            if self.infinity_challenge_completed(3) {
                mult *= base;
            }
        }

        if self.infinity_challenge_running(8) {
            let elapsed_ms = (self.records.this_infinity.time_ms
                - self.records.this_infinity.last_buy_time_ms)
                .max(0.0);
            mult *= Decimal::from_float(0.844_630_338_903_428_8)
                .pow(&Decimal::from_float(elapsed_ms));
        }

        if self.infinity_challenge_running(6) {
            mult /= self.matter.max(&Decimal::ONE);
        }

        mult
    }

    /// The power applied to a dimension's *final* multiplier (0-indexed `tier`),
    /// from `applyNDPowers`: IC4 weakens every dimension except the last-bought one
    /// to `^0.25` while running, and (as its reward) raises all to `^1.05` once
    /// completed.
    pub(crate) fn infinity_challenge_mult_power(&self, tier: usize) -> f64 {
        let mut power = 1.0;
        if self.infinity_challenge_running(4) && self.post_c4_tier as usize != tier + 1 {
            power *= 0.25;
        }
        if self.infinity_challenge_completed(4) {
            power *= 1.05;
        }
        power
    }

    /// Extra reduction to the Dimension-Boost and Antimatter-Galaxy requirements
    /// from a completed IC5 (`−1`, in addition to the `resetBoost` upgrade).
    pub(crate) fn ic5_requirement_reduction(&self) -> u64 {
        if self.infinity_challenge_completed(5) {
            1
        } else {
            0
        }
    }

    /// IC5 `multiplyIC5Costs`, triggered by completing a group of 10 of dimension
    /// `src_tier` (0-indexed): buying AD1–4 raises the cost of every strictly
    /// *cheaper* dimension, buying AD5–8 raises every strictly *pricier* one.
    pub(crate) fn ic5_bump_costs_from_dimension(&mut self, src_tier: usize) {
        let src_cost = self.dimension_cost(src_tier);
        let src_is_low = src_tier <= 3; // 1-indexed tiers 1..4
        for t in 0..8 {
            if t == src_tier {
                continue;
            }
            let cost = self.dimension_cost(t);
            let bump = if src_is_low {
                cost < src_cost
            } else {
                cost > src_cost
            };
            if bump {
                self.dimensions[t].cost_bumps += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::constants::BIG_CRUNCH_THRESHOLD;

    #[test]
    fn ic_unlocks_at_max_am_this_eternity() {
        let mut game = GameState::new();
        assert!(!game.infinity_challenge_unlocked(1));
        // IC1 unlocks at 1e2000 peak antimatter this eternity.
        game.records.this_eternity.max_am = Decimal::new(1.0, 2000);
        assert!(game.infinity_challenge_unlocked(1));
        assert!(!game.infinity_challenge_unlocked(2)); // needs 1e11000
        assert!(game.infinity_challenges_unlocked());
    }

    #[test]
    fn start_infinity_challenge_breaks_infinity_and_enters() {
        let mut game = GameState::new();
        game.records.this_eternity.max_am = Decimal::new(1.0, 2000);
        assert!(!game.broke_infinity);
        assert!(game.start_infinity_challenge(1));
        assert!(game.infinity_challenge_running(1));
        assert!(game.broke_infinity);
        assert_eq!(game.challenge.current, 0);
    }

    #[test]
    fn crunching_at_ic_goal_completes_it() {
        let mut game = GameState::new();
        // IC3 unlocks at 1e12000 peak AM this eternity; its goal is the lower 1e5000.
        game.records.this_eternity.max_am = Decimal::new(1.0, 12000);
        game.start_infinity_challenge(3);
        assert!(game.infinity_challenge_running(3));

        // Below the IC goal a crunch does nothing.
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(!game.can_big_crunch());

        // At the IC goal, a crunch completes IC3 and exits.
        game.antimatter = Decimal::new(1.0, 5000);
        assert!(game.can_big_crunch());
        assert!(game.big_crunch());
        assert!(game.infinity_challenge_completed(3));
        assert!(!game.any_infinity_challenge_running());
    }

    #[test]
    fn retry_challenge_re_enters_ic_after_crunch() {
        let mut game = GameState::new();
        game.records.this_eternity.max_am = Decimal::new(1.0, 12000);
        game.options.retry_challenge = true;
        game.start_infinity_challenge(3);
        assert!(game.infinity_challenge_running(3));

        // Crunching at the IC goal completes it, but with retry on the IC stays
        // active (re-entered) rather than exiting.
        game.antimatter = Decimal::new(1.0, 5000);
        assert!(game.big_crunch());
        assert!(game.infinity_challenge_completed(3));
        assert!(game.infinity_challenge_running(3));
        assert!(game.any_infinity_challenge_running());
    }

    #[test]
    fn ic1_composes_normal_challenge_modifiers() {
        let mut game = GameState::new();
        game.records.this_eternity.max_am = Decimal::new(1.0, 2000);
        game.start_infinity_challenge(1);
        // IC1 runs every Normal Challenge except NC9 and NC12.
        assert!(game.challenge_running(2));
        assert!(game.challenge_running(8));
        assert!(!game.challenge_running(9));
        assert!(!game.challenge_running(12));
    }

    #[test]
    fn ic3_neutralizes_tickspeed_and_grants_static_mult() {
        let mut game = GameState::new();
        game.records.this_eternity.max_am = Decimal::new(1.0, 12000);
        game.start_infinity_challenge(3);
        game.tickspeed.bought = 10;
        game.galaxies = 2;
        // Tickspeed's production effect is neutralised to ×1.
        assert_eq!(game.tickspeed_effect(), Decimal::ONE);
        // The static AD multiplier is (1.05 + 2×0.005)^10 = 1.06^10.
        let expected = 1.06_f64.powi(10);
        let mult = game.infinity_challenge_common_mult().to_f64();
        assert!((mult / expected - 1.0).abs() < 1e-9, "{mult} vs {expected}");
    }

    #[test]
    fn ic7_disables_galaxies_and_boosts_dim_boost_power() {
        let mut game = GameState::new();
        game.records.this_eternity.max_am = Decimal::new(1.0, 23000);
        game.start_infinity_challenge(7);
        game.dimensions[7].amount = Decimal::from_float(1e9);
        assert!(!game.can_buy_galaxy());
        // Base Dim-Boost power raised to ×10 while running.
        assert_eq!(game.dim_boost_power(), Decimal::from_float(10.0));
        // Completed (not running): floored at ×4.
        game.exit_challenge();
        game.complete_infinity_challenge(7);
        assert_eq!(game.dim_boost_power(), Decimal::from_float(4.0));
    }

    #[test]
    fn ic4_weakens_all_but_the_latest_dimension() {
        let mut game = GameState::new();
        game.records.this_eternity.max_am = Decimal::new(1.0, 14000);
        game.start_infinity_challenge(4);
        game.post_c4_tier = 3; // the 3rd AD (index 2) is the last bought
        assert_eq!(game.infinity_challenge_mult_power(2), 1.0);
        assert_eq!(game.infinity_challenge_mult_power(0), 0.25);
        assert_eq!(game.infinity_challenge_mult_power(7), 0.25);
    }
}
