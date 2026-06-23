use break_infinity::Decimal;

use crate::data::constants::{
    AD_BASE_COSTS, AD_COST_MULTIPLIERS, FIRST_GALAXY_REQUIREMENT, GALAXY_REQUIREMENT_INCREMENT,
    INITIAL_ANTIMATTER,
};
use crate::state::{DimensionTier, GameState, TickspeedState};

impl GameState {
    /// Get the number of 8th dimensions required for the next galaxy.
    pub fn galaxy_requirement(&self) -> u64 {
        if self.galaxies == 0 {
            FIRST_GALAXY_REQUIREMENT
        } else {
            FIRST_GALAXY_REQUIREMENT
                + self.galaxies as u64 * GALAXY_REQUIREMENT_INCREMENT
        }
    }

    /// Check if the player can buy an antimatter galaxy.
    pub fn can_buy_galaxy(&self) -> bool {
        // Need enough 8th dimensions (bought count, not amount)
        self.dimensions[7].bought >= self.galaxy_requirement()
    }

    /// Buy an antimatter galaxy. Resets dimensions and tickspeed but
    /// permanently increases the tickspeed reduction.
    /// Returns true if successful.
    pub fn buy_galaxy(&mut self) -> bool {
        if !self.can_buy_galaxy() {
            return false;
        }

        self.galaxies += 1;
        self.galaxy_reset();
        true
    }

    /// Perform a galaxy reset: reset all dimensions, tickspeed, and dim boosts.
    /// Antimatter is reset to 10.
    fn galaxy_reset(&mut self) {
        self.antimatter = Decimal::from_float(INITIAL_ANTIMATTER);
        self.dim_boosts = 0;
        self.sacrifice_unlocked = false;
        self.sacrificed = Decimal::default();

        // Reset all dimensions to initial state
        for i in 0..8 {
            self.dimensions[i] = DimensionTier::new(
                Decimal::from_float(AD_BASE_COSTS[i]),
                Decimal::from_float(AD_COST_MULTIPLIERS[i]),
            );
        }

        // Reset tickspeed purchases (but galaxy count is kept, improving the multiplier)
        self.tickspeed = TickspeedState::new();
    }

    /// Get the dimension boost requirement for the next boost.
    /// Returns (tier_required_0indexed, amount_required).
    /// First 4 boosts require 20 of dims 4, 5, 6, 7 respectively.
    /// After that, each boost requires 15 more of dim 8 (index 7).
    pub fn dim_boost_requirement(&self) -> (usize, u64) {
        use crate::data::constants::{
            DIM_BOOST_INITIAL_REQUIREMENT, DIM_BOOST_SCALING_REQUIREMENT,
        };

        if self.dim_boosts < 4 {
            // Boosts 0-3 require 20 of dims 4-7 (indices 3-6)
            let tier = 3 + self.dim_boosts as usize; // tier index 3, 4, 5, 6
            (tier, DIM_BOOST_INITIAL_REQUIREMENT)
        } else {
            // Subsequent boosts require increasingly more 8th dimensions
            let extra = (self.dim_boosts - 4) as u64;
            let required = DIM_BOOST_INITIAL_REQUIREMENT
                + (extra + 1) * DIM_BOOST_SCALING_REQUIREMENT;
            (7, required)
        }
    }

    /// Check if the player can perform a dimension boost.
    pub fn can_dim_boost(&self) -> bool {
        let (tier, required) = self.dim_boost_requirement();
        self.is_dimension_unlocked(tier) && self.dimensions[tier].bought >= required
    }

    /// Perform a dimension boost. Resets dimensions but keeps galaxies.
    /// Returns true if successful.
    pub fn buy_dim_boost(&mut self) -> bool {
        if !self.can_dim_boost() {
            return false;
        }

        self.dim_boosts += 1;

        // Unlock sacrifice when 5th dimension becomes available
        if self.dim_boosts >= 1 {
            self.sacrifice_unlocked = true;
        }

        self.dim_boost_reset();
        true
    }

    /// Perform a dimension boost reset: reset antimatter and all dimensions.
    /// Tickspeed and galaxies are kept.
    fn dim_boost_reset(&mut self) {
        self.antimatter = Decimal::from_float(INITIAL_ANTIMATTER);
        self.sacrificed = Decimal::default();

        // Reset all dimensions
        for i in 0..8 {
            self.dimensions[i] = DimensionTier::new(
                Decimal::from_float(AD_BASE_COSTS[i]),
                Decimal::from_float(AD_COST_MULTIPLIERS[i]),
            );
        }

        // Tickspeed is reset on dim boost
        self.tickspeed = TickspeedState::new();
    }
}
