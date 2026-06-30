use break_infinity::Decimal;

use crate::data::constants::BIG_CRUNCH_THRESHOLD;
use crate::state::{DimensionTier, GameState, TickspeedState};

impl GameState {
    /// Whether the player can perform a Big Crunch: antimatter has reached the
    /// Big Crunch threshold (where it is capped, see `tick`).
    pub fn can_big_crunch(&self) -> bool {
        self.antimatter >= BIG_CRUNCH_THRESHOLD
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

        // 21: "To infinity!" — unlocks on the crunch itself (the original's
        // BIG_CRUNCH_BEFORE), so the post-reset starting antimatter already
        // reflects its 100-antimatter reward.
        self.unlock_achievement(21);
        self.antimatter = self.starting_antimatter();
        self.dimensions = std::array::from_fn(|_| DimensionTier::new());
        self.tickspeed = TickspeedState::new();
        self.dim_boosts = 0;
        self.galaxies = 0;
        self.sacrificed = Decimal::ZERO;
        self.infinity_unlocked = true;

        true
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
    fn tickspeed_unlocks_with_second_dimension_purchase() {
        let mut game = GameState::new();
        assert!(!game.tickspeed_unlocked());

        game.dimensions[1].bought = 1;
        assert!(game.tickspeed_unlocked());
    }
}
