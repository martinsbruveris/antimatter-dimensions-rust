use break_infinity::Decimal;

use crate::data::constants::{
    FIRST_GALAXY_REQUIREMENT, GALAXY_REQUIREMENT_INCREMENT,
    NC10_FIRST_GALAXY_REQUIREMENT, NC10_GALAXY_REQUIREMENT_INCREMENT,
};
use crate::state::{DimensionTier, GameState, TickspeedState};

impl GameState {
    /// The dimension tier (0-indexed) whose amount gates the next galaxy.
    /// Mirrors `Galaxy.requiredTier`: the 6th dimension (index 5) under Normal
    /// Challenge 10, otherwise the 8th (index 7).
    pub fn galaxy_required_tier(&self) -> usize {
        if self.challenge_running(10) {
            5
        } else {
            7
        }
    }

    /// Get the number of required-tier dimensions needed for the next galaxy.
    /// Normal Challenge 10 raises the cost (base 99, +90 per galaxy).
    pub fn galaxy_requirement(&self) -> u64 {
        let (base_cost, cost_mult) = if self.challenge_running(10) {
            (
                NC10_FIRST_GALAXY_REQUIREMENT,
                NC10_GALAXY_REQUIREMENT_INCREMENT,
            )
        } else {
            (FIRST_GALAXY_REQUIREMENT, GALAXY_REQUIREMENT_INCREMENT)
        };
        let base = base_cost + self.galaxies as u64 * cost_mult;
        // The `resetBoost` Infinity Upgrade reduces the requirement by 9.
        base.saturating_sub(self.reset_boost_reduction())
    }

    /// Check if the player can buy an antimatter galaxy. Normal Challenge 8
    /// disables Antimatter Galaxies entirely (`Galaxy.canBeBought`).
    pub fn can_buy_galaxy(&self) -> bool {
        if self.challenge_running(8) {
            return false;
        }
        // Check total amount (floor) of the required-tier dimension.
        let tier = self.galaxy_required_tier();
        let amount = self.dimensions[tier].amount.to_f64().floor() as u64;
        amount >= self.galaxy_requirement()
    }

    /// Buy an antimatter galaxy. Resets dimensions and
    /// tickspeed but permanently increases the tickspeed
    /// reduction. Returns true if successful.
    pub fn buy_galaxy(&mut self) -> bool {
        if !self.can_buy_galaxy() {
            return false;
        }

        // Clear the GALAXY tutorial highlight on the purchase (the original's
        // requestGalaxyReset calls turnOffEffect before the reset).
        self.tutorial_turn_off(crate::tutorial::state::GALAXY);
        // 26: buy an Antimatter Galaxy (fires before the reset, like the
        // original's GALAXY_RESET_BEFORE).
        self.unlock_achievement(26);
        self.galaxies += 1;
        self.galaxy_reset();
        // 27: have ≥ 2 Antimatter Galaxies (after the increment).
        if self.galaxies >= 2 {
            self.unlock_achievement(27);
        }
        true
    }

    /// Perform a galaxy reset: reset all dimensions,
    /// tickspeed, and dim boosts. Antimatter is reset to its starting value.
    fn galaxy_reset(&mut self) {
        self.antimatter = self.starting_antimatter();
        self.dim_boosts = 0;
        self.sacrificed = Decimal::ZERO;

        for i in 0..8 {
            self.dimensions[i] = DimensionTier::new();
        }

        self.tickspeed = TickspeedState::new();
        // Reset the per-run challenge accumulators (`softReset` → `resetChallengeStuff`).
        self.reset_challenge_stuff();
        // Re-apply skip-reset Infinity Upgrades (original `softReset` calls
        // `skipResetsIfPossible`), so e.g. skipResetGalaxy restores 4 boosts.
        self.skip_resets_if_possible();
    }

    /// Get the dimension boost requirement for the next boost.
    /// Returns (tier_required_0indexed, amount_required).
    ///
    /// Mirrors `DimBoost.bulkRequirement(1)`. The required tier is
    /// `min(targetResets + 3, maxDimensionsUnlockable)` (1-indexed, where
    /// `targetResets = purchasedBoosts + 1`): the first boosts unlock the 5th/6th/…
    /// dimensions at 20 each, then the top tier scales — +15 per boost past the
    /// 5th at tier 8, or (under Normal Challenge 10) +20 per boost past the 3rd
    /// at tier 6.
    pub fn dim_boost_requirement(&self) -> (usize, u64) {
        use crate::data::constants::{
            DIM_BOOST_INITIAL_REQUIREMENT, DIM_BOOST_SCALING_REQUIREMENT,
            NC10_DIM_BOOST_SCALING_REQUIREMENT,
        };

        let max_dim = self.max_dimensions_unlockable() as u32;
        // The boost being paid for (1-indexed count after this purchase).
        let target_resets = self.dim_boosts + 1;
        // 1-indexed tier that gates it.
        let tier_1indexed = (target_resets + 3).min(max_dim);

        let mut amount = DIM_BOOST_INITIAL_REQUIREMENT;
        if tier_1indexed == 6 && self.challenge_running(10) {
            amount += (target_resets.saturating_sub(3)) as u64
                * NC10_DIM_BOOST_SCALING_REQUIREMENT;
        } else if tier_1indexed == 8 {
            amount +=
                (target_resets.saturating_sub(5)) as u64 * DIM_BOOST_SCALING_REQUIREMENT;
        }
        // The `resetBoost` Infinity Upgrade reduces the requirement by 9.
        let amount = amount.saturating_sub(self.reset_boost_reduction());
        (tier_1indexed as usize - 1, amount)
    }

    /// Check if the player can perform a dimension boost. Normal Challenge 8 caps
    /// the total at 5 ([`max_boosts`](Self::max_boosts)).
    pub fn can_dim_boost(&self) -> bool {
        if let Some(max) = self.max_boosts() {
            if self.dim_boosts >= max {
                return false;
            }
        }
        let (tier, required) = self.dim_boost_requirement();
        if !self.is_dimension_unlocked(tier) {
            return false;
        }
        // Check total amount (floor) of the required dimension
        let amount = self.dimensions[tier].amount.to_f64().floor() as u64;
        amount >= required
    }

    /// Perform a dimension boost. Resets dimensions but keeps
    /// galaxies. Returns true if successful.
    pub fn buy_dim_boost(&mut self) -> bool {
        if !self.can_dim_boost() {
            return false;
        }

        // Clear the DIMBOOST tutorial highlight on the purchase (the original's
        // requestDimensionBoost calls turnOffEffect before the reset).
        self.tutorial_turn_off(crate::tutorial::state::DIMBOOST);
        self.dim_boosts += 1;
        self.dim_boost_reset();
        // 25: buy 10 Dimension Boosts.
        if self.dim_boosts >= 10 {
            self.unlock_achievement(25);
        }
        true
    }

    /// Perform a dimension boost reset: reset antimatter and
    /// all dimensions. Tickspeed and galaxies are kept.
    fn dim_boost_reset(&mut self) {
        self.antimatter = self.starting_antimatter();
        self.sacrificed = Decimal::ZERO;

        for i in 0..8 {
            self.dimensions[i] = DimensionTier::new();
        }

        self.tickspeed = TickspeedState::new();
        // Reset the per-run challenge accumulators (`softReset` → `resetChallengeStuff`).
        self.reset_challenge_stuff();
        // Original `softReset` also runs `skipResetsIfPossible`; a no-op unless a
        // skip level exceeds the just-incremented boost count.
        self.skip_resets_if_possible();
    }
}
