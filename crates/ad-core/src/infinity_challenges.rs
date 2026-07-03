//! Infinity Challenges: 8 constrained runs gated by antimatter thresholds that
//! all exceed `1e308` (so they need Break Infinity). Structurally like the Normal
//! Challenges (2.5) but with a per-challenge *goal* rather than the fixed `1e308`.
//! This module owns the run state machine ([`InfinityChallengeState`] on
//! [`GameState`]) and the unlock/start/exit/complete logic; the per-challenge rule
//! modifiers live at their engine sites. See
//! `design-docs/2026-07-03-infinity-challenges.md`.

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
        self.records.max_am_this_eternity >= IC_UNLOCK_AM[(id - 1) as usize]
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
        game.records.max_am_this_eternity = Decimal::new(1.0, 2000);
        assert!(game.infinity_challenge_unlocked(1));
        assert!(!game.infinity_challenge_unlocked(2)); // needs 1e11000
        assert!(game.infinity_challenges_unlocked());
    }

    #[test]
    fn start_infinity_challenge_breaks_infinity_and_enters() {
        let mut game = GameState::new();
        game.records.max_am_this_eternity = Decimal::new(1.0, 2000);
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
        game.records.max_am_this_eternity = Decimal::new(1.0, 12000);
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
    fn ic1_composes_normal_challenge_modifiers() {
        let mut game = GameState::new();
        game.records.max_am_this_eternity = Decimal::new(1.0, 2000);
        game.start_infinity_challenge(1);
        // IC1 runs every Normal Challenge except NC9 and NC12.
        assert!(game.challenge_running(2));
        assert!(game.challenge_running(8));
        assert!(!game.challenge_running(9));
        assert!(!game.challenge_running(12));
    }
}
