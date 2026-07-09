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
        // GALAXY_RESET_BEFORE achievements (26, 38), before the reset (and before
        // `galaxy_reset` restores `infinity_no_sacrifice`).
        self.check_galaxy_before_achievements();
        // Pelle's power-up-Galaxies Strike unlocks the Decay rift.
        self.pelle_trigger_strike(2);
        self.galaxies += 1;
        self.galaxy_reset();
        // GALAXY_RESET_AFTER achievements (27, …), after the galaxy increment.
        self.check_galaxy_after_achievements();
        true
    }

    /// Perform a galaxy reset (`galaxyReset`): clear Dimension Boosts, then
    /// the shared soft reset (which the ANR perk can soften — antimatter /
    /// dimensions / tickspeed / sacrifice kept).
    fn galaxy_reset(&mut self) {
        // Achievement 143 (`galaxyReset`): Antimatter Galaxies no longer reset
        // Dimension Boosts.
        if !self.achievement_unlocked(143) {
            self.dim_boosts = 0;
        }
        self.soft_reset(false);
        // Restored per-Galaxy (not per-Infinity), matching `galaxyReset`.
        self.requirement_checks.infinity_no_sacrifice = true;
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
        self.dim_boost_bulk_requirement(1)
    }

    /// The requirement to buy `bulk` Dimension Boosts at once (`DimBoost
    /// .bulkRequirement`): the `(0-indexed tier, amount)` of the dimension gating
    /// the `bulk`-th boost from the current count.
    pub fn dim_boost_bulk_requirement(&self, bulk: u32) -> (usize, u64) {
        use crate::data::constants::{
            DIM_BOOST_INITIAL_REQUIREMENT, DIM_BOOST_SCALING_REQUIREMENT,
            NC10_DIM_BOOST_SCALING_REQUIREMENT,
        };

        let max_dim = self.max_dimensions_unlockable() as u32;
        // The boost being paid for (1-indexed count after this purchase).
        let target_resets = self.dim_boosts + bulk;
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
        // Past the Infinity goal, boosting is pointless — you should crunch
        // instead — so both the purchase (`requestDimensionBoost`: antimatter over
        // `Player.infinityLimit`) and `DimBoost.canBeBought` (max antimatter over
        // `Player.infinityGoal`, while unbroken or inside an antimatter challenge)
        // block it. The limit is the challenge goal, else the full `Decimal::MAX`
        // (so post-break boosting past `1e308` is unaffected); the goal is the
        // challenge goal, else `1e308`.
        if self.antimatter > self.infinity_limit() {
            return false;
        }
        if self.records.this_infinity.max_am > self.infinity_goal()
            && (!self.broke_infinity || self.in_antimatter_challenge())
        {
            return false;
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

    /// Whether the Dimension Boost autobuyer runs in "buy max" mode, i.e. the
    /// `autobuyMaxDimboosts` Break Infinity Upgrade is bought
    /// (`DimBoostAutobuyerState.isBuyMaxUnlocked`). In that mode it uses a distinct
    /// tick path ([`max_buy_dim_boosts`](Self::max_buy_dim_boosts)) and gate.
    pub fn is_buy_max_dimboosts_unlocked(&self) -> bool {
        self.break_infinity_upgrade_bought(
            crate::break_infinity_upgrades::BreakInfinityUpgrade::AutobuyMaxDimboosts,
        )
    }

    /// Whether a Dimension Boost would unlock a new Antimatter Dimension
    /// (`DimBoost.canUnlockNewDimension`): `purchasedBoosts + 4 < maxDimensions`.
    pub fn can_unlock_new_dimension(&self) -> bool {
        (self.dim_boosts as usize) + 4 < self.max_dimensions_unlockable()
    }

    /// Whether the `bulk`-boost requirement is met (`bulkRequirement(bulk)
    /// .isSatisfied`): the gating dimension's amount reaches the required count.
    fn dim_boost_bulk_requirement_satisfied(&self, bulk: u32) -> bool {
        let (tier, required) = self.dim_boost_bulk_requirement(bulk);
        if !self.is_dimension_unlocked(tier) {
            return false;
        }
        self.dimensions[tier].amount >= Decimal::from_float(required as f64)
    }

    /// The autobuyer's "buy max" Dimension Boost action (`maxBuyDimBoosts`): buy as
    /// many boosts as the current dimensions afford. Boosts that unlock a new
    /// dimension are bought one at a time; past that, the count is extrapolated
    /// from the linearly-scaling requirement (with a binary-search fallback for
    /// EC5's cubic scaling). Returns whether any boost happened.
    pub fn max_buy_dim_boosts(&mut self) -> bool {
        // Boosts that unlock new dims are bought one at a time.
        if self.can_unlock_new_dimension() {
            if self.dim_boost_bulk_requirement_satisfied(1) {
                return self.soft_reset_bulk(1);
            }
            return false;
        }
        if !self.dim_boost_bulk_requirement_satisfied(1) {
            return false;
        }
        if !self.dim_boost_bulk_requirement_satisfied(2) {
            return self.soft_reset_bulk(1);
        }
        // Linearly extrapolate the (tier-8) requirement: req(n) = a·n + b.
        let (tier1, amount1) = self.dim_boost_bulk_requirement(1);
        let (_, amount2) = self.dim_boost_bulk_requirement(2);
        let increase = (amount2 - amount1) as f64;
        let have = self.dimensions[tier1].amount.to_f64();
        let mut max_boosts =
            (1.0 + ((have - amount1 as f64) / increase).floor()).min(f64::MAX) as u32;
        if self.dim_boost_bulk_requirement_satisfied(max_boosts) {
            return self.soft_reset_bulk(max_boosts);
        }
        // EC5's cubic scaling can overshoot — binary-search the true maximum.
        let mut min_boosts = 2u32;
        while max_boosts != min_boosts + 1 {
            let middle = (max_boosts + min_boosts) / 2;
            if self.dim_boost_bulk_requirement_satisfied(middle) {
                min_boosts = middle;
            } else {
                max_boosts = middle;
            }
        }
        self.soft_reset_bulk(min_boosts)
    }

    /// `softReset(bulk)` for a multi-boost purchase: refuse past the Infinity
    /// limit, cap the count at the boost maximum, add the boosts, and run the
    /// shared soft reset. Returns whether it happened.
    fn soft_reset_bulk(&mut self, bulk: u32) -> bool {
        if self.antimatter > self.infinity_limit() {
            return false;
        }
        let bulk = match self.max_boosts() {
            Some(max) => bulk.min(max.saturating_sub(self.dim_boosts)),
            None => bulk,
        };
        self.tutorial_turn_off(crate::tutorial::state::DIMBOOST);
        self.dim_boosts += bulk;
        self.dim_boost_reset();
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
        // The original splits this into two gates: `Perk.antimatterNoReset` (30)
        // keeps dimensions/tickspeed/sacrifice, while keeping *antimatter* also
        // accepts Achievement 111 (`Achievement(111).isUnlocked ||
        // Perk.antimatterNoReset`).
        let keep_dimensions = !forced && self.perk_bought(30);
        let keep_antimatter =
            !forced && (self.perk_bought(30) || self.achievement_unlocked(111));
        // `resetChallengeStuff` runs regardless.
        self.reset_challenge_stuff();
        if !keep_dimensions {
            self.sacrificed = Decimal::ZERO;
            for i in 0..8 {
                self.dimensions[i] = DimensionTier::new();
            }
            self.tickspeed = TickspeedState::new();
        }
        // Original `softReset` also runs `skipResetsIfPossible`; a no-op unless a
        // skip level exceeds the just-incremented boost count.
        self.skip_resets_if_possible();
        if keep_antimatter {
            self.antimatter = self.antimatter.max(&self.starting_antimatter());
        } else {
            self.antimatter = self.starting_antimatter();
        }
    }
}
