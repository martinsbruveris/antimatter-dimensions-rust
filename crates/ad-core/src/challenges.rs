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

    /// The number of Antimatter Dimensions unlockable in the current run.
    /// Mirrors `DimBoost.maxDimensionsUnlockable`: Normal Challenge 10 restricts
    /// play to 6 dimensions, otherwise all 8.
    pub fn max_dimensions_unlockable(&self) -> usize {
        if self.challenge_running(10) {
            6
        } else {
            8
        }
    }

    /// The maximum number of Dimension Boosts purchasable in the current run;
    /// `None` = unbounded. Mirrors `DimBoost.maxBoosts`: Normal Challenge 8 caps
    /// it at 5 (the 5th unlocks Sacrifice, and further boosts are pointless since
    /// NC8 zeroes the boost multiplier).
    pub fn max_boosts(&self) -> Option<u32> {
        if self.challenge_running(8) {
            Some(5)
        } else {
            None
        }
    }

    /// Reset the per-run challenge accumulators (`resetChallengeStuff`), run on
    /// every soft reset (Dimension Boost, Antimatter Galaxy) and Big Crunch. For
    /// now just NC8's running sacrifice product; NC2/NC3 powers and NC11 matter
    /// join it in later batches.
    pub(crate) fn reset_challenge_stuff(&mut self) {
        self.chall8_total_sacrifice = Decimal::ONE;
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
    fn nc7_reduces_buy_ten_multiplier() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        // Outside a challenge the buy-10 base is ×2.
        assert_eq!(game.buy_ten_multiplier(), Decimal::from_float(2.0));

        game.start_challenge(7);
        // NC7 with 0 boosts: min(2, 1 + 0/5) = ×1.
        assert_eq!(game.buy_ten_multiplier(), Decimal::from_float(1.0));
        game.dim_boosts = 3;
        // 1 + 3/5 = ×1.6.
        assert_eq!(game.buy_ten_multiplier(), Decimal::from_float(1.6));
        game.dim_boosts = 12;
        // Capped at ×2.
        assert_eq!(game.buy_ten_multiplier(), Decimal::from_float(2.0));
    }

    #[test]
    fn nc5_weakens_tickspeed_multiplier() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        let normal = game.tickspeed_purchase_multiplier();

        game.start_challenge(5);
        // NC5 base 1/1.08 > normal 1/1.1245 → a *larger* retained multiplier at 0
        // galaxies, i.e. weaker tickspeed (the challenge).
        assert!(game.tickspeed_purchase_multiplier() > normal);
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

    // --- NC8: no boost multiplier, no galaxies, capped boosts, stronger sacrifice.

    #[test]
    fn nc8_zeroes_boost_multiplier_and_caps_boosts() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.start_challenge(8);

        // Dimension Boosts give no multiplier (power = 1).
        assert_eq!(game.dim_boost_power(), Decimal::ONE);
        assert_eq!(game.max_boosts(), Some(5));

        // At the 5-boost cap, no further boost is possible even with plenty of AD8.
        game.dim_boosts = 5;
        game.dimensions[7].amount = Decimal::from_float(1e6);
        assert!(!game.can_dim_boost());
    }

    #[test]
    fn nc8_disables_galaxies() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        // Outside a challenge, enough 8th dimensions allow a galaxy.
        game.dimensions[7].amount = Decimal::from_float(1e6);
        assert!(game.can_buy_galaxy());

        game.start_challenge(8);
        game.dimensions[7].amount = Decimal::from_float(1e6);
        assert!(!game.can_buy_galaxy());
    }

    #[test]
    fn nc8_sacrifice_accumulates_running_product_and_full_resets() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.start_challenge(8);
        // Sacrifice needs >= 5 boosts and an 8th dimension.
        game.dim_boosts = 5;
        game.dimensions[0].amount = Decimal::from_float(1e10);
        game.dimensions[0].bought = 40;
        game.dimensions[7].amount = Decimal::from_float(10.0);
        game.dimensions[7].bought = 5;
        game.antimatter = Decimal::from_float(1e50);

        assert_eq!(game.chall8_total_sacrifice, Decimal::ONE);
        let next = game.next_sacrifice_boost();
        assert!(next > Decimal::ONE, "next boost should exceed 1: {next:?}");
        assert!(game.can_sacrifice());
        assert!(game.sacrifice());

        // The running product advances by the boost and drives the 8th-dim mult.
        assert_eq!(game.chall8_total_sacrifice, next);
        assert_eq!(game.sacrifice_multiplier(), next);
        // Everything resets: all dimensions cleared, antimatter back to start.
        for tier in 0..8 {
            assert_eq!(game.dimensions[tier].amount, Decimal::ZERO);
            assert_eq!(game.dimensions[tier].bought, 0);
        }
        assert_eq!(game.antimatter, game.starting_antimatter());
    }

    #[test]
    fn crunch_and_boost_reset_chall8_total_sacrifice() {
        // A Big Crunch resets the running product (resetChallengeStuff).
        let mut game = GameState::new();
        game.chall8_total_sacrifice = Decimal::from_float(1e20);
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert_eq!(game.chall8_total_sacrifice, Decimal::ONE);

        // So does a Dimension Boost.
        game.chall8_total_sacrifice = Decimal::from_float(1e20);
        game.dimensions[3].amount = Decimal::from_float(100.0); // 20 of the 4th
        assert!(game.buy_dim_boost());
        assert_eq!(game.chall8_total_sacrifice, Decimal::ONE);
    }

    // --- NC10: only 6 dimensions, modified boost/galaxy costs, no sacrifice.

    #[test]
    fn nc10_limits_dimensions_to_six() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.infinities = Decimal::from_float(16.0);
        game.start_challenge(10);
        game.dim_boosts = 4; // would normally unlock all 8

        assert_eq!(game.max_dimensions_unlockable(), 6);
        assert_eq!(game.unlocked_dimensions(), 6);
        // The 7th dimension (index 6) is never unlocked or purchasable.
        game.dimensions[5].amount = Decimal::ONE; // own a 6th
        assert!(!game.is_dimension_unlocked(6));
        assert!(!game.dim_available_for_purchase(6));
    }

    #[test]
    fn nc10_modifies_galaxy_cost_and_tier() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.infinities = Decimal::from_float(16.0);
        game.start_challenge(10);

        // Gated on the 6th dimension (index 5), base cost 99, +90 per galaxy.
        assert_eq!(game.galaxy_required_tier(), 5);
        assert_eq!(game.galaxy_requirement(), 99);
        game.galaxies = 2;
        assert_eq!(game.galaxy_requirement(), 99 + 2 * 90);
        game.dimensions[5].amount = Decimal::from_float(1000.0);
        assert!(game.can_buy_galaxy());
    }

    #[test]
    fn nc10_dim_boost_requires_more_sixth_dimensions() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.infinities = Decimal::from_float(16.0);
        game.start_challenge(10);

        // Early boosts unlock the 5th/6th dims at 20; then the 6th-dim cost scales
        // by +20 per boost.
        assert_eq!(game.dim_boost_requirement(), (3, 20)); // 4th dim
        game.dim_boosts = 2;
        assert_eq!(game.dim_boost_requirement(), (5, 20)); // 6th dim
        game.dim_boosts = 3;
        assert_eq!(game.dim_boost_requirement(), (5, 40));
        game.dim_boosts = 4;
        assert_eq!(game.dim_boost_requirement(), (5, 60));
    }

    #[test]
    fn nc10_disables_sacrifice() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.infinities = Decimal::from_float(16.0);
        game.start_challenge(10);
        game.dim_boosts = 6;
        game.dimensions[7].amount = Decimal::from_float(100.0);
        assert!(!game.can_sacrifice());
    }
}
