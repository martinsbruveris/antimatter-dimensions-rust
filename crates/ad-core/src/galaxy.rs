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

    /// The galaxy count where Distant cost scaling starts
    /// (`Galaxy.costScalingStart`): 100, pushed later by TS223 (+7) and TS224
    /// (+1 per 2000 Dimension Boosts). The EC5 reward joins with Feature 4.5.
    pub fn galaxy_cost_scaling_start(&self) -> u64 {
        let mut start = 100;
        if self.time_study_bought(223) {
            start += 7;
        }
        if self.time_study_bought(224) {
            start += self.dim_boosts as u64 / 2000;
        }
        // EC5's reward pushes Distant scaling 5 galaxies later per completion.
        start += 5 * self.eternity_challenge_completions(5) as u64;
        // Power-glyph sacrifice pushes it later still (up to +750).
        start += self.glyph_sac_power_effect() as u64;
        start
    }

    /// The per-galaxy requirement step (`Galaxy.costMult`): 60 (90 under NC10),
    /// reduced to 52 by TS42.
    fn galaxy_cost_mult(&self) -> u64 {
        if self.challenge_running(10) {
            90
        } else if self.time_study_bought(42) {
            52
        } else {
            GALAXY_REQUIREMENT_INCREMENT
        }
    }

    /// Get the number of required-tier dimensions needed for the next galaxy.
    /// Normal Challenge 10 raises the cost (base 99, +90 per galaxy); Distant
    /// scaling (past `galaxy_cost_scaling_start`) adds a quadratic term and
    /// Remote scaling (past 800) an exponential one (`Galaxy.requirementAt`).
    pub fn galaxy_requirement(&self) -> u64 {
        let galaxies = self.galaxies as u64;
        let base_cost = if self.challenge_running(10) {
            NC10_FIRST_GALAXY_REQUIREMENT
        } else {
            FIRST_GALAXY_REQUIREMENT
        };
        let _ = NC10_GALAXY_REQUIREMENT_INCREMENT; // step handled in galaxy_cost_mult
        let mut amount = (base_cost + galaxies * self.galaxy_cost_mult()) as f64;

        // Distant scaling: quadratic growth past the scaling start. Under EC5
        // it starts immediately (`galaxies² + galaxies`).
        if self.ec_running(5) {
            let g = galaxies as f64;
            amount += g * g + g;
        } else {
            let scaling_start = self.galaxy_cost_scaling_start();
            if galaxies >= scaling_start {
                let before_distant = (galaxies - scaling_start + 1) as f64;
                amount += before_distant * before_distant + before_distant;
            }
        }
        // Remote scaling: exponential growth past galaxy 800 (moved to
        // 100,000 by Reality Upgrade 21).
        let remote_start: u64 = if self.reality_upgrade_bought(21) {
            100_000
        } else {
            800
        };
        if galaxies >= remote_start {
            amount *= 1.002f64.powi((galaxies - (remote_start - 1)) as i32);
        }

        // The `resetBoost` Infinity Upgrade reduces the requirement by 9, and a
        // completed Infinity Challenge 5 by a further 1.
        (amount.floor() as u64).saturating_sub(
            self.reset_boost_reduction() + self.ic5_requirement_reduction(),
        )
    }

    /// Check if the player can buy an antimatter galaxy. Normal Challenge 8 and
    /// Infinity Challenge 7 disable Antimatter Galaxies entirely
    /// (`Galaxy.canBeBought`).
    pub fn can_buy_galaxy(&self) -> bool {
        if self.challenge_running(8) || self.infinity_challenge_running(7) {
            return false;
        }
        // EC6: Antimatter Galaxies cannot be gained normally.
        if self.ec_running(6) {
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

    /// Perform a galaxy reset (`galaxyReset`): clear Dimension Boosts, then
    /// the shared soft reset (which the ANR perk can soften — antimatter /
    /// dimensions / tickspeed / sacrifice kept).
    fn galaxy_reset(&mut self) {
        self.dim_boosts = 0;
        self.soft_reset(false);
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

        // TS211/TS222 reduce the per-boost requirement scaling (by 5 / 2).
        let mut discount = 0;
        if self.time_study_bought(211) {
            discount += 5;
        }
        if self.time_study_bought(222) {
            discount += 2;
        }
        let mut amount = DIM_BOOST_INITIAL_REQUIREMENT;
        if tier_1indexed == 6 && self.challenge_running(10) {
            amount += (target_resets.saturating_sub(3)) as u64
                * (NC10_DIM_BOOST_SCALING_REQUIREMENT - discount);
        } else if tier_1indexed == 8 {
            amount += (target_resets.saturating_sub(5)) as u64
                * (DIM_BOOST_SCALING_REQUIREMENT - discount);
        }
        // EC5: Dimension Boost costs scale massively
        // (`+(targetResets−1)³ + (targetResets−1)`).
        if self.ec_running(5) {
            let t = (target_resets - 1) as u64;
            amount += t * t * t + t;
        }
        // The `resetBoost` Infinity Upgrade reduces the requirement by 9, and a
        // completed Infinity Challenge 5 by a further 1.
        let amount = amount.saturating_sub(
            self.reset_boost_reduction() + self.ic5_requirement_reduction(),
        );
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

    /// Perform a dimension boost reset: reset antimatter and all dimensions
    /// (unless the ANR perk keeps them). Tickspeed and galaxies are kept.
    pub(crate) fn dim_boost_reset(&mut self) {
        self.soft_reset(false);
    }

    /// The forced variant (`softReset(0, true, true)`): NC11's matter
    /// annihilation and the Replicanti Galaxy reset ignore the ANR perk.
    pub(crate) fn dim_boost_reset_forced(&mut self) {
        self.soft_reset(true);
    }

    /// The shared `softReset` body. With the `antimatterNoReset` perk (30)
    /// and no force, antimatter / dimensions / tickspeed / sacrifice are all
    /// kept (antimatter still bumps up to its starting value).
    fn soft_reset(&mut self, forced: bool) {
        let keep = !forced && self.perk_bought(30);
        // `resetChallengeStuff` runs regardless.
        self.reset_challenge_stuff();
        if !keep {
            self.sacrificed = Decimal::ZERO;
            for i in 0..8 {
                self.dimensions[i] = DimensionTier::new();
            }
            self.tickspeed = TickspeedState::new();
        }
        // Original `softReset` also runs `skipResetsIfPossible`; a no-op unless a
        // skip level exceeds the just-incremented boost count.
        self.skip_resets_if_possible();
        if keep {
            self.antimatter = self.antimatter.max(&self.starting_antimatter());
        } else {
            self.antimatter = self.starting_antimatter();
        }
    }
}
