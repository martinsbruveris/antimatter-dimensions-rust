//! Normal Challenges: constrained pre-Infinity runs (Feature 2.5).
//!
//! Each challenge modifies the production rules; completing one (by reaching
//! Infinity, i.e. `this_infinity.max_am >= BIG_CRUNCH_THRESHOLD`, while it runs)
//! sets its completed bit and grants a reward — an autobuyer. This module owns the
//! run state machine (`NormalChallengeState` on [`GameState`]), the unlock/start/
//! exit/complete logic, and the reward wiring; the per-challenge *rule modifiers*
//! (NC2–12) are added incrementally at their engine sites. NC1 (the tutorial, no
//! restriction) is the first vertical slice. See
//! `docs/design/2026-07-03-normal-challenges.md`.

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
    /// Whether normal challenge `id` is the one currently running — directly, or
    /// as part of Infinity Challenge 1 (which runs every Normal Challenge except
    /// NC9 and NC12 simultaneously, mirroring `NormalChallenge(id).isRunning`).
    pub fn challenge_running(&self, id: u8) -> bool {
        if self.challenge.current == id {
            return true;
        }
        id != 9 && id != 12 && self.infinity_challenge_running(1)
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
    /// every soft reset (Dimension Boost, Antimatter Galaxy) and Big Crunch:
    /// NC2's production factor back to full, NC3's 1st-dim multiplier back to its
    /// weak `0.01` start, NC8's running sacrifice product to `1`, and normal
    /// matter to `0` (`Currency.matter.reset()`).
    pub(crate) fn reset_challenge_stuff(&mut self) {
        self.chall2_pow = 1.0;
        self.chall3_pow = Decimal::from_float(0.01);
        self.matter = Decimal::ZERO;
        self.chall8_total_sacrifice = Decimal::ONE;
    }

    /// NC9 `multiplySameCosts`, triggered by completing a group of 10 of dimension
    /// `src_tier`: every *other* dimension — and Tickspeed — whose current cost
    /// shares the source's order of magnitude (its Decimal exponent) gets a cost
    /// bump, jumping it to the next cost step.
    pub(crate) fn nc9_bump_same_cost_from_dimension(&mut self, src_tier: usize) {
        let src_e = self.dimension_cost(src_tier).exponent();
        for t in 0..8 {
            if t != src_tier && self.dimension_cost(t).exponent() == src_e {
                self.dimensions[t].cost_bumps += 1;
            }
        }
        if self.tickspeed.cost.exponent() == src_e {
            self.tickspeed.cost_bumps += 1;
            self.tickspeed.cost *= self.tickspeed.cost_multiplier;
        }
    }

    /// NC9 `Tickspeed.multiplySameCosts`: buying a Tickspeed upgrade bumps every
    /// dimension whose current cost shares Tickspeed's order of magnitude.
    pub(crate) fn nc9_bump_same_cost_from_tickspeed(&mut self) {
        let ts_e = self.tickspeed.cost.exponent();
        for t in 0..8 {
            if self.dimension_cost(t).exponent() == ts_e {
                self.dimensions[t].cost_bumps += 1;
            }
        }
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

    /// Exit the current challenge (normal or infinity): clear it and force a Big
    /// Crunch reset (no rewards below goal). Mirrors `…ChallengeState.exit` →
    /// `bigCrunchReset(true, false)`.
    pub fn exit_challenge(&mut self) -> bool {
        if self.infinity_challenge.current != 0 {
            self.infinity_challenge.current = 0;
            self.big_crunch_reset(true, false);
            return true;
        }
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
        // Completing an NC unlocks an autobuyer, changing the cheapest-locked
        // set; re-arm the badge so the per-tick check re-evaluates (mirrors
        // NormalChallengeState.complete's clearTrigger + cache invalidation,
        // which the original runs unconditionally).
        self.clear_tab_notification_trigger(
            crate::tab_notifications::TabNotificationId::NewAutobuyer,
        );
        if !already {
            self.on_challenge_reward(id);
        }
    }

    /// Called from the Big-Crunch reward path. Completes the running challenge, or
    /// NC1 on the first-ever Infinity performed outside a challenge, then exits the
    /// challenge — unless the "Automatically retry challenges" option
    /// (`retry_challenge`) is on, in which case the challenge stays active so the
    /// crunch re-enters it. Mirrors `handleChallengeCompletion`.
    pub(crate) fn handle_challenge_completion(&mut self) {
        let retry = self.options.retry_challenge;
        // An Infinity Challenge takes precedence (the two never run at once).
        if self.infinity_challenge.current != 0 {
            let ic = self.infinity_challenge.current;
            // An IC's first completion re-arms the IC-unlock badge so the next
            // unlock badges again (mirrors handleChallengeCompletion's
            // "clear after the first completion (only)").
            if !self.infinity_challenge_completed(ic) {
                self.clear_tab_notification_trigger(
                    crate::tab_notifications::TabNotificationId::IcUnlock,
                );
            }
            self.complete_infinity_challenge(ic);
            if !retry {
                self.infinity_challenge.current = 0;
            }
            return;
        }
        let current = self.challenge.current;
        if current == 0 {
            if !self.challenge_completed(1) {
                self.complete_challenge(1);
            }
            return;
        }
        self.complete_challenge(current);
        if !retry {
            self.challenge.current = 0;
        }
    }

    /// Whether an antimatter challenge (Normal or Infinity) is currently running.
    /// Mirrors `Player.isInAntimatterChallenge`.
    pub fn in_antimatter_challenge(&self) -> bool {
        self.challenge.current != 0 || self.infinity_challenge.current != 0
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
    fn retry_challenge_keeps_it_running_after_crunch() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.options.retry_challenge = true;
        game.start_challenge(3);
        assert!(game.challenge_running(3));

        // Reaching the goal and crunching completes and rewards NC3 as usual, but
        // the challenge stays active (re-entered) instead of exiting.
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());

        assert!(game.challenge_completed(3));
        assert!(game.autobuyers.dimensions[2].is_bought);
        assert!(game.challenge_running(3));
        assert!(game.any_challenge_running());
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

    // --- NC2: buying halts production, which recovers over 3 minutes.

    #[test]
    fn nc2_buying_halts_production_then_recovers() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.start_challenge(2);
        game.autobuyers.enabled = false;
        // Starts at full production.
        assert_eq!(game.chall2_pow, 1.0);

        // Buying a dimension (or tickspeed) halts production.
        game.antimatter = Decimal::from_float(1e6);
        assert!(game.buy_dimension(0));
        assert_eq!(game.chall2_pow, 0.0);

        // Recovers linearly: dt/100/1800 per ms, i.e. full after 3 minutes.
        game.tick(90_000.0);
        assert!((game.chall2_pow - 0.5).abs() < 1e-9, "{}", game.chall2_pow);
        game.tick(90_000.0);
        assert!((game.chall2_pow - 1.0).abs() < 1e-9, "{}", game.chall2_pow);
    }

    #[test]
    fn nc2_scales_all_dimension_production() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.start_challenge(2);
        game.dimensions[0].amount = Decimal::from_float(100.0);
        game.dimensions[0].bought = 10;

        let full = game.dimension_production_per_second(0);
        game.chall2_pow = 0.5;
        let halved = game.dimension_production_per_second(0);
        assert!((halved.to_f64() / full.to_f64() - 0.5).abs() < 1e-9);
    }

    // --- NC3: 1st dimension weakened to ×0.01 but grows exponentially.

    #[test]
    fn nc3_grows_first_dimension_multiplier() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.start_challenge(3);
        game.autobuyers.enabled = false;
        // Starts weak.
        assert_eq!(game.chall3_pow, Decimal::from_float(0.01));

        // Grows ×1.00038 per 100 ms. After 100 s: 0.01 × 1.00038^1000.
        game.tick(100_000.0);
        let expected = Decimal::from_float(0.01 * 1.000_38_f64.powf(1000.0));
        assert!(
            (game.chall3_pow.to_f64() / expected.to_f64() - 1.0).abs() < 1e-6,
            "{:?} vs {:?}",
            game.chall3_pow,
            expected
        );
    }

    #[test]
    fn nc3_scales_only_the_first_dimension() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.start_challenge(3);
        game.dimensions[0].amount = Decimal::from_float(100.0);
        game.dimensions[0].bought = 10;
        game.dimensions[1].amount = Decimal::from_float(100.0);
        game.dimensions[1].bought = 10;

        let ad1_weak = game.dimension_production_per_second(0);
        let ad2_weak = game.dimension_production_per_second(1);
        game.chall3_pow = Decimal::ONE;
        let ad1_strong = game.dimension_production_per_second(0);
        let ad2_strong = game.dimension_production_per_second(1);

        // AD1 scales by chall3Pow (1.0 / 0.01 = 100×); AD2 is unaffected.
        assert!((ad1_strong.to_f64() / ad1_weak.to_f64() - 100.0).abs() < 1e-6);
        assert_eq!(ad2_weak, ad2_strong);
    }

    // --- NC11: normal matter rises and can annihilate antimatter.

    #[test]
    fn nc11_matter_rises_only_with_a_second_dimension() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.infinities = Decimal::from_float(16.0);
        game.start_challenge(11);
        game.autobuyers.enabled = false;
        // Reset to 0 on entry; stays 0 without a 2nd dimension.
        assert_eq!(game.matter, Decimal::ZERO);
        game.antimatter = Decimal::from_float(1e100);
        game.tick(1000.0);
        assert_eq!(game.matter, Decimal::ZERO);

        // With a 2nd dimension it bumps to 1 and grows above it.
        game.dimensions[1].amount = Decimal::from_float(5.0);
        game.tick(1000.0);
        assert!(game.matter > Decimal::ONE, "{:?}", game.matter);
    }

    #[test]
    fn nc11_annihilation_soft_resets_keeping_boosts_and_galaxies() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.infinities = Decimal::from_float(16.0);
        game.start_challenge(11);
        game.autobuyers.enabled = false;
        game.dim_boosts = 2;
        game.galaxies = 1;
        game.dimensions[1].amount = Decimal::from_float(5.0); // AD2 exists
        game.dimensions[0].amount = Decimal::from_float(50.0);
        game.dimensions[0].bought = 20;
        game.tickspeed.bought = 5;
        // Matter already exceeds a low antimatter, and we cannot Crunch.
        game.matter = Decimal::from_float(1e6);
        game.antimatter = Decimal::from_float(100.0);

        game.tick(50.0);

        // Dimensions/tickspeed/antimatter reset; boosts and galaxies preserved.
        assert_eq!(game.dim_boosts, 2);
        assert_eq!(game.galaxies, 1);
        assert_eq!(game.dimensions[0].bought, 0);
        assert_eq!(game.tickspeed.bought, 0);
        assert_eq!(game.antimatter, game.starting_antimatter());
        // resetChallengeStuff zeroes matter.
        assert_eq!(game.matter, Decimal::ZERO);
    }

    #[test]
    fn crunch_resets_challenge_powers_and_matter() {
        let mut game = GameState::new();
        game.chall2_pow = 0.3;
        game.chall3_pow = Decimal::from_float(5.0);
        game.matter = Decimal::from_float(1e10);
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert_eq!(game.chall2_pow, 1.0);
        assert_eq!(game.chall3_pow, Decimal::from_float(0.01));
        assert_eq!(game.matter, Decimal::ZERO);
    }

    // --- NC4: buying a dimension erases all lower-tier amounts.

    #[test]
    fn nc4_erases_lower_dimensions_on_buy() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.start_challenge(4);
        game.dimensions[0].amount = Decimal::from_float(100.0);
        game.dimensions[1].amount = Decimal::from_float(50.0); // AD2 owned → AD3 available
        game.dimensions[2].amount = Decimal::from_float(5.0);
        game.antimatter = Decimal::from_float(1e12);

        assert!(game.buy_dimension(2)); // buy a 3rd dimension
                                        // Lower amounts erased; the just-bought tier keeps its amount.
        assert_eq!(game.dimensions[0].amount, Decimal::ZERO);
        assert_eq!(game.dimensions[1].amount, Decimal::ZERO);
        assert!(game.dimensions[2].amount > Decimal::ZERO);
    }

    // --- NC6: dimensions bought with the dimension 2 tiers below, different costs.

    #[test]
    fn nc6_buys_higher_dimensions_with_lower_ones() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.start_challenge(6);
        // The 3rd dimension uses the C6 base cost (100) and is paid for with the
        // 1st dimension, not antimatter.
        game.dimensions[1].amount = Decimal::from_float(1.0); // AD2 owned (availability)
        game.dimensions[0].amount = Decimal::from_float(1000.0); // AD1 = currency
        assert_eq!(game.dimension_cost(2), Decimal::from_float(100.0));

        let am_before = game.antimatter;
        assert!(game.buy_dimension(2));
        assert_eq!(game.antimatter, am_before); // antimatter untouched
        assert_eq!(game.dimensions[0].amount, Decimal::from_float(900.0)); // AD1 spent
        assert_eq!(game.dimensions[2].bought, 1);
    }

    // --- NC9: buying 10 of a dimension (or tickspeed) bumps equal-cost costs.

    #[test]
    fn nc9_bumps_equal_cost_dimensions_on_group_of_ten() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.start_challenge(9);
        // Arrange AD1 and AD2 to share a cost order of magnitude: AD1 at group 3
        // (e = 1 + 3×3 = 10), AD2 at group 2 (e = 2 + 4×2 = 10).
        game.dimensions[0].bought = 39;
        game.dimensions[1].bought = 20;
        game.dimensions[1].amount = Decimal::ONE;
        assert_eq!(
            game.dimension_cost(0).exponent(),
            game.dimension_cost(1).exponent()
        );
        let ad2_cost_before = game.dimension_cost(1);
        game.antimatter = game.dimension_cost(0) * Decimal::from_float(2.0);

        // Buying the 40th 1st dimension completes a group of 10 → bumps AD2.
        assert!(game.buy_dimension(0));
        assert_eq!(game.dimensions[1].cost_bumps, 1);
        assert_eq!(game.dimensions[0].cost_bumps, 0); // source not bumped
        assert!(game.dimension_cost(1) > ad2_cost_before);
    }

    // --- NC12: production shifts 2 tiers, 1st/2nd make antimatter, evens stronger.

    #[test]
    fn nc12_first_two_dimensions_make_antimatter_and_shift_two_tiers() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.infinities = Decimal::from_float(16.0);
        game.start_challenge(12);
        game.autobuyers.enabled = false;
        for t in 0..4 {
            game.dimensions[t].amount = Decimal::from_float(10.0);
            game.dimensions[t].bought = 10;
        }
        let am_before = game.antimatter;
        let ad1_before = game.dimensions[0].amount;
        let ad2_before = game.dimensions[1].amount;

        game.tick(1000.0);

        // Antimatter grows (1st + 2nd dims); AD3→AD1 and AD4→AD2.
        assert!(game.antimatter > am_before);
        assert!(game.dimensions[0].amount > ad1_before);
        assert!(game.dimensions[1].amount > ad2_before);
    }

    #[test]
    fn nc12_strengthens_the_second_dimension() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.infinities = Decimal::from_float(16.0);
        game.start_challenge(12);
        game.dimensions[1].amount = Decimal::from_float(100.0);
        game.dimensions[1].bought = 10;

        // AD2 production uses amount^1.6.
        let produced = game.dimension_production_per_second(1);
        let expected = Decimal::from_float(100.0).pow(&Decimal::from_float(1.6))
            * game.dimension_multiplier(1)
            * game.tickspeed_effect();
        assert!((produced.to_f64() / expected.to_f64() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn nc12_sacrifice_keeps_the_seventh_dimension() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.infinities = Decimal::from_float(16.0);
        game.start_challenge(12);
        game.dim_boosts = 5;
        for t in 0..8 {
            game.dimensions[t].amount = Decimal::from_float(100.0);
        }
        game.dimensions[0].amount = Decimal::new(1.0, 20); // AD1 large → next boost > 1
        game.dimensions[7].bought = 5;

        assert!(game.can_sacrifice());
        assert!(game.sacrifice());
        // Dims 1–6 (indices 0–5) reset; AD7 (index 6) and AD8 (index 7) kept.
        for i in 0..6 {
            assert_eq!(game.dimensions[i].amount, Decimal::ZERO);
        }
        assert_eq!(game.dimensions[6].amount, Decimal::from_float(100.0));
        assert_eq!(game.dimensions[7].amount, Decimal::from_float(100.0));
    }
}
