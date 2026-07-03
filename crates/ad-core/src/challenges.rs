//! Normal Challenges: constrained pre-Infinity runs (Feature 2.5).
//!
//! Each challenge modifies the production rules; completing one (by reaching
//! Infinity, i.e. `this_infinity.max_am >= BIG_CRUNCH_THRESHOLD`, while it runs)
//! sets its completed bit and grants a reward — an autobuyer. This module owns the
//! run state machine (`NormalChallengeState` on [`GameState`]), the unlock/start/
//! exit/complete logic, and the reward wiring; the per-challenge *rule modifiers*
//! (NC2–12) are added incrementally at their engine sites. NC1 (the tutorial, no
//! restriction) is the first vertical slice. See
//! `design-docs/2026-07-03-normal-challenges.md`.

use break_infinity::Decimal;

use crate::state::GameState;

/// Number of normal challenges.
pub const NORMAL_CHALLENGE_COUNT: u8 = 12;

/// Infinities required to unlock each challenge, indexed by `id - 1`. Challenges
/// 1–9 are available as soon as the Challenges tab is (0 infinities); 10–12 need
/// 16 (`lockedAt` in the original data).
const CHALLENGE_LOCKED_AT: [u32; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 16, 16];

/// Normal-challenge run state. Mirrors `player.challenge.normal`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NormalChallengeState {
    /// Active challenge id (`0` = none, `1..=12` = running that challenge).
    pub current: u8,
    /// Completed-challenge bitmask (bit `1 << id`).
    pub completed: u16,
}

impl GameState {
    /// Whether normal challenge `id` is the one currently running.
    pub fn challenge_running(&self, id: u8) -> bool {
        self.challenge.current == id
    }

    /// Whether any normal challenge is currently running.
    pub fn any_challenge_running(&self) -> bool {
        self.challenge.current != 0
    }

    /// Whether normal challenge `id` has been completed.
    pub fn challenge_completed(&self, id: u8) -> bool {
        (1..=NORMAL_CHALLENGE_COUNT).contains(&id)
            && self.challenge.completed & (1u16 << id) != 0
    }

    /// Whether the Challenges tab is available — after the first Big Crunch
    /// (`PlayerProgress.infinityUnlocked()`).
    pub fn challenges_unlocked(&self) -> bool {
        self.infinity_unlocked
    }

    /// Whether normal challenge `id` is unlocked (tab open **and** enough
    /// Infinities): mirrors `isUnlocked = infinitiesTotal >= lockedAt`.
    pub fn challenge_unlocked(&self, id: u8) -> bool {
        if !(1..=NORMAL_CHALLENGE_COUNT).contains(&id) || !self.infinity_unlocked {
            return false;
        }
        let required = CHALLENGE_LOCKED_AT[(id - 1) as usize];
        self.infinities >= Decimal::from_float(required as f64)
    }

    /// Whether challenge `id` can be started right now. NC1 is the base game (not
    /// "started"); the target must be unlocked and not already the active one.
    pub fn can_start_challenge(&self, id: u8) -> bool {
        (2..=NORMAL_CHALLENGE_COUNT).contains(&id)
            && self.challenge_unlocked(id)
            && self.challenge.current != id
    }

    /// Start normal challenge `id`: a forced Big Crunch (rewards only if at goal),
    /// then enter the challenge. Mirrors `NormalChallengeState.start` →
    /// `bigCrunchReset(true, true)`.
    pub fn start_challenge(&mut self, id: u8) -> bool {
        if !self.can_start_challenge(id) {
            return false;
        }
        self.big_crunch_reset(true, true);
        self.challenge.current = id;
        true
    }

    /// Exit the current challenge: clear it and force a Big Crunch reset (no
    /// rewards below goal). Mirrors `NormalChallengeState.exit` →
    /// `bigCrunchReset(true, false)`.
    pub fn exit_challenge(&mut self) -> bool {
        if self.challenge.current == 0 {
            return false;
        }
        self.challenge.current = 0;
        self.big_crunch_reset(true, false);
        true
    }

    /// Mark challenge `id` completed and grant its reward once. Idempotent.
    pub(crate) fn complete_challenge(&mut self, id: u8) {
        if !(1..=NORMAL_CHALLENGE_COUNT).contains(&id) {
            return;
        }
        let already = self.challenge_completed(id);
        self.challenge.completed |= 1u16 << id;
        if !already {
            self.on_challenge_reward(id);
        }
    }

    /// Called from the Big-Crunch reward path. Completes the running challenge, or
    /// NC1 on the first-ever Infinity performed outside a challenge, then exits the
    /// challenge (no `retryChallenge` modelled). Mirrors `handleChallengeCompletion`.
    pub(crate) fn handle_challenge_completion(&mut self) {
        let current = self.challenge.current;
        if current == 0 {
            if !self.challenge_completed(1) {
                self.complete_challenge(1);
            }
            return;
        }
        self.complete_challenge(current);
        self.challenge.current = 0;
    }

    /// Grant a challenge's reward: unlock the corresponding autobuyer. NC1–8 →
    /// the 1st–8th Antimatter Dimension autobuyers, NC9 → Tickspeed. The
    /// Dim-Boost / Galaxy / Big-Crunch autobuyers (NC10–12) are not modelled yet
    /// (Feature 2.6), so those rewards are currently no-ops.
    fn on_challenge_reward(&mut self, id: u8) {
        match id {
            1..=8 => self.autobuyers.dimensions[(id - 1) as usize].is_bought = true,
            9 => self.autobuyers.tickspeed.is_bought = true,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::constants::BIG_CRUNCH_THRESHOLD;

    #[test]
    fn first_infinity_completes_nc1_and_unlocks_first_autobuyer() {
        let mut game = GameState::new();
        assert!(!game.challenge_completed(1));
        assert!(!game.autobuyers.dimensions[0].is_bought);

        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());

        assert!(game.challenge_completed(1));
        assert!(game.autobuyers.dimensions[0].is_bought);
    }

    #[test]
    fn challenge_unlock_respects_infinities_threshold() {
        let mut game = GameState::new();
        // Tab not open before the first infinity.
        assert!(!game.challenge_unlocked(2));

        game.infinity_unlocked = true;
        // C2–9 are available immediately; C10–12 need 16 infinities.
        assert!(game.challenge_unlocked(2));
        assert!(!game.challenge_unlocked(10));
        game.infinities = Decimal::from_float(16.0);
        assert!(game.challenge_unlocked(10));
        assert!(game.challenge_unlocked(12));
    }

    #[test]
    fn start_and_exit_challenge() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        // NC1 cannot be started; NC2 can once unlocked.
        assert!(!game.can_start_challenge(1));
        assert!(game.start_challenge(2));
        assert!(game.challenge_running(2));
        assert!(game.any_challenge_running());

        assert!(game.exit_challenge());
        assert!(!game.any_challenge_running());
    }

    #[test]
    fn completing_a_challenge_by_crunching_exits_and_rewards() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.start_challenge(3);
        assert!(game.challenge_running(3));
        assert!(!game.autobuyers.dimensions[2].is_bought);

        // Reach the goal and crunch → completes NC3, unlocks the 3rd AD autobuyer,
        // and exits the challenge.
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());

        assert!(game.challenge_completed(3));
        assert!(game.autobuyers.dimensions[2].is_bought);
        assert!(!game.any_challenge_running());
    }

    #[test]
    fn starting_a_challenge_below_goal_does_not_reward() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.infinities = Decimal::from_float(5.0);
        let infinities_before = game.infinities;

        // Not at goal: starting NC2 forces a reset but grants no IP/infinities.
        assert!(game.start_challenge(2));
        assert_eq!(game.infinities, infinities_before);
        assert!(!game.challenge_completed(2));
    }
}
