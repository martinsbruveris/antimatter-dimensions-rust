use break_infinity::Decimal;

use crate::data::constants::{BIG_CRUNCH_THRESHOLD, INITIAL_ANTIMATTER};
use crate::state::{DimensionTier, GameState, TickspeedState};

impl GameState {
    /// Whether the player can perform a Big Crunch: antimatter has reached the
    /// Big Crunch threshold (where it is capped, see `tick`).
    pub fn can_big_crunch(&self) -> bool {
        self.antimatter >= BIG_CRUNCH_THRESHOLD
    }

    /// Perform the first Big Crunch (Infinity): reset all pre-Infinity progress
    /// — antimatter, dimensions, tickspeed, dimension boosts, galaxies, and
    /// sacrifices — back to the start. Autobuyer configuration is preserved
    /// (it is not pre-Infinity progress). Returns true if the crunch happened.
    ///
    /// Infinity Points are not awarded yet; that comes in a later step.
    pub fn big_crunch(&mut self) -> bool {
        if !self.can_big_crunch() {
            return false;
        }

        self.antimatter = Decimal::from_float(INITIAL_ANTIMATTER);
        self.dimensions = std::array::from_fn(|_| DimensionTier::new());
        self.tickspeed = TickspeedState::new();
        self.dim_boosts = 0;
        self.galaxies = 0;
        self.sacrificed = Decimal::ZERO;

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

        // Everything back to a fresh game.
        let fresh = GameState::new();
        assert_eq!(game.antimatter, fresh.antimatter);
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
    }
}
