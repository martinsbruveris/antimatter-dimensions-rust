use break_infinity::Decimal;

use crate::data::constants::{
    AD_BASE_COSTS, AD_COST_MULTIPLIERS, C6_AD_BASE_COSTS, C6_AD_COST_MULTIPLIERS,
};
use crate::state::GameState;

impl GameState {
    /// Compute the current cost for the next purchase of a
    /// dimension tier (0-indexed). Cost increases every 10
    /// purchases: `base × mult^(bought/10 + cost_bumps)`. Normal Challenge 6 uses
    /// a different cost table; `cost_bumps` is the NC9 same-cost bump (0 otherwise).
    pub fn dimension_cost(&self, tier: usize) -> Decimal {
        let (base, mult) = if self.challenge_running(6) {
            (C6_AD_BASE_COSTS[tier], C6_AD_COST_MULTIPLIERS[tier])
        } else {
            (AD_BASE_COSTS[tier], AD_COST_MULTIPLIERS[tier])
        };
        let purchase_group =
            self.dimensions[tier].bought / 10 + self.dimensions[tier].cost_bumps;
        Decimal::from_float(base)
            * Decimal::from_float(mult).pow(&Decimal::from_float(purchase_group as f64))
    }

    /// The currency a dimension purchase spends. Normal Challenge 6 pays for the
    /// 3rd and higher dimensions with the dimension 2 tiers below (`currencyAmount`);
    /// otherwise it is antimatter.
    fn dim_currency_amount(&self, tier: usize) -> Decimal {
        if tier >= 2 && self.challenge_running(6) {
            self.dimensions[tier - 2].amount
        } else {
            self.antimatter
        }
    }

    /// Subtract `cost` from the currency a dimension purchase spends (see
    /// [`dim_currency_amount`](Self::dim_currency_amount)).
    fn spend_dim_currency(&mut self, tier: usize, cost: Decimal) {
        if tier >= 2 && self.challenge_running(6) {
            self.dimensions[tier - 2].amount -= cost;
        } else {
            self.antimatter -= cost;
        }
    }

    /// Try to buy one of the specified dimension tier
    /// (0-indexed). Returns true if the purchase was
    /// successful.
    pub fn buy_dimension(&mut self, tier: usize) -> bool {
        if !self.dim_available_for_purchase(tier) {
            return false;
        }

        let cost = self.dimension_cost(tier);
        if self.dim_currency_amount(tier) >= cost {
            self.spend_dim_currency(tier, cost);
            // Completing a group of 10 bumps other dimensions' costs under NC9
            // (equal cost) or IC5 (cheaper/pricier by tier; IC5 takes precedence,
            // mirroring `challengeCostBump`). Done before the `bought` increment so
            // the source's cost still reflects the current (pre-rollover) group.
            if self.dimensions[tier].bought % 10 == 9 {
                if self.infinity_challenge_running(5) {
                    self.ic5_bump_costs_from_dimension(tier);
                } else if self.challenge_running(9) {
                    self.nc9_bump_same_cost_from_dimension(tier);
                }
            }
            self.dimensions[tier].amount += Decimal::from_float(1.0);
            self.dimensions[tier].bought += 1;
            self.on_buy_dimension(tier);
            true
        } else {
            false
        }
    }

    /// Achievement checks fired after buying one of `tier` (0-indexed). Mirrors
    /// the original's `onBuyDimension` unlocks: "buy an Nth dimension" (11–18),
    /// the exactly-99 eighth-dimension achievement (23), and the over-1e150
    /// first-dimension achievement (28).
    fn on_buy_dimension(&mut self, tier: usize) {
        // Normal Challenge 2: any Antimatter Dimension purchase halts production
        // (resets the recovering `chall2Pow` factor to 0).
        if self.challenge_running(2) {
            self.chall2_pow = 0.0;
        }

        // Normal Challenge 4: buying a dimension erases all lower-tier dimension
        // amounts (keeping their bought counts) — like a Sacrifice with no boost.
        if self.challenge_running(4) {
            for i in 0..tier {
                self.dimensions[i].amount = Decimal::ZERO;
            }
        }

        // Track the last-bought tier (Infinity Challenge 4) and buy time (IC8).
        self.post_c4_tier = tier as u8 + 1;
        self.records.this_infinity.last_buy_time_ms = self.records.this_infinity.time_ms;

        // 11–18: buy a 1st..8th Antimatter Dimension (tier is 0-indexed).
        self.unlock_achievement(11 + tier as u16);
        // 23: have exactly 99 eighth dimensions (only buying AD8 can reach it).
        if tier == 7 && self.dimensions[7].amount == Decimal::from_float(99.0) {
            self.unlock_achievement(23);
        }
        // 28: buy a 1st dimension while holding over 1e150 of them.
        if tier == 0 && self.dimensions[0].amount.exponent() >= 150 {
            self.unlock_achievement(28);
        }
    }

    /// Buy the maximum number of the specified dimension tier
    /// that can be afforded. Returns the number of dimensions
    /// bought.
    pub fn buy_max_dimension(&mut self, tier: usize) -> u64 {
        let mut count = 0u64;
        while self.buy_dimension(tier) {
            count += 1;
        }
        count
    }

    /// Buy dimensions until the next group of 10 is complete.
    /// Returns the number bought.
    pub fn buy_until_10_dimension(&mut self, tier: usize) -> u64 {
        if !self.dim_available_for_purchase(tier) {
            return 0;
        }
        let remaining = 10 - (self.dimensions[tier].bought % 10);
        let mut count = 0u64;
        for _ in 0..remaining {
            if !self.buy_dimension(tier) {
                break;
            }
            count += 1;
        }
        count
    }

    /// Compute the cost to buy until the next group of 10
    /// for a dimension tier. Since cost only changes every
    /// 10 purchases, all remaining buys in this group cost
    /// the same.
    pub fn dimension_cost_until_10(&self, tier: usize) -> Decimal {
        let remaining = 10 - (self.dimensions[tier].bought % 10);
        self.dimension_cost(tier) * Decimal::from_float(remaining as f64)
    }

    /// Get the antimatter production per second (from AD1).
    pub fn antimatter_per_second(&self) -> Decimal {
        self.dimension_production_per_second(0)
    }

    /// Buy max of all unlocked dimensions and tickspeed.
    pub fn max_all(&mut self) {
        self.buy_max_tickspeed();
        let unlocked = self.unlocked_dimensions();
        for tier in 0..unlocked {
            self.buy_max_dimension(tier);
        }
    }

    /// Compute the production multiplier for a given dimension
    /// tier (0-indexed). Includes:
    /// - Buy-10 multiplier (2x per 10 purchases)
    /// - Tier-dependent dim boost multiplier
    /// - Sacrifice multiplier (only for tier 8 / index 7)
    pub fn dimension_multiplier(&self, tier: usize) -> Decimal {
        // EC11: every multiplier is disabled except Infinity Power and
        // Dimension Boosts (`getDimensionFinalMultiplierUncached`).
        if self.ec_running(11) {
            let exponent = (self.dim_boosts as i64 - tier as i64).max(0);
            let mut mult = self.infinity_power_ad_multiplier();
            if exponent > 0 {
                mult *= self
                    .dim_boost_power()
                    .pow(&Decimal::from_float(exponent as f64));
            }
            return mult;
        }

        let mut mult = Decimal::from_float(1.0);

        // Buy-10 multiplier: base^(bought / 10). Base is 2, or 2.2 with the
        // `buy10Mult` Infinity Upgrade (`buy_ten_multiplier`).
        let buy10_groups = self.dimensions[tier].bought / 10;
        if buy10_groups > 0 {
            mult *= self
                .buy_ten_multiplier()
                .pow(&Decimal::from_float(buy10_groups as f64));
        }

        // Dim boost: power^max(0, boosts - tier)
        // tier is 0-indexed; JS formula is
        // power^(boosts + 1 - js_tier) where js_tier = tier+1
        // so exponent = boosts + 1 - (tier + 1) = boosts - tier.
        // Power is 2, or 2.5 with the `dimboostMult` Infinity Upgrade.
        let exponent = (self.dim_boosts as i64 - tier as i64).max(0);
        if exponent > 0 {
            mult *= self
                .dim_boost_power()
                .pow(&Decimal::from_float(exponent as f64));
        }

        // Sacrifice multiplier applies only to 8th dimension
        if tier == 7 {
            mult *= self.sacrifice_multiplier();
            // TS214: Dimensional Sacrifice boosts the 8th dimension even more
            // (each term exponent-capped like the original's clampMaxExponent).
            if self.time_study_bought(214) {
                let total = self.sacrifice_multiplier();
                let first = total
                    .pow(&Decimal::from_float(7.6))
                    .min(&Decimal::new_unchecked(1.0, 44_000));
                let second = total
                    .pow(&Decimal::from_float(1.05))
                    .min(&Decimal::new_unchecked(1.0, 120_000));
                mult *= (first * second).min(&Decimal::new_unchecked(1.0, 164_000));
            }
        } else if self.time_study_bought(71) {
            // TS71: sacrifice affects all other ADs with reduced effect.
            mult *= self
                .sacrifice_multiplier()
                .pow(&Decimal::from_float(0.25))
                .max(&Decimal::ONE)
                .min(&Decimal::new_unchecked(1.0, 210_000));
        }
        // TS234: sacrifice applies to the 1st dimension in full.
        if tier == 0 && self.time_study_bought(234) {
            mult *= self.sacrifice_multiplier();
        }

        // All-tier Time Study multipliers.
        // TS91: based on time spent this eternity (caps at 20 minutes).
        if self.time_study_bought(91) {
            let minutes = (self.records.this_eternity.time_ms / 60_000.0).min(20.0);
            mult *= Decimal::pow10(minutes * 15.0);
        }
        // TS101: equal to the Replicanti amount.
        if self.time_study_bought(101) {
            mult *= self.replicanti.amount.max(&Decimal::ONE);
        }
        // TS161: flat ×1e616.
        if self.time_study_bought(161) {
            mult *= Decimal::new_unchecked(1.0, 616);
        }
        // TS193: based on Eternities (caps at 1e13000 at 1e6 eternities).
        if self.time_study_bought(193) {
            let frac = (self.eternities / Decimal::from_float(1e6))
                .min(&Decimal::ONE)
                .to_f64();
            mult *= Decimal::pow10(13_000.0 * frac);
        }

        // Achievement effects. The global achievement power applies to every
        // dimension; achievements 28 / 23 boost the 1st / 8th dimension by 10%.
        mult *= self.achievement_power();
        if tier == 0 && self.achievement_unlocked(28) {
            mult *= Decimal::from_float(1.1);
        }
        if tier == 7 && self.achievement_unlocked(23) {
            mult *= Decimal::from_float(1.1);
        }

        // Infinity Upgrade multipliers: the common (all-tier) time multipliers and
        // the per-tier dim-pair / unspent-IP multipliers.
        mult *= self.infinity_upgrade_common_mult();
        mult *= self.infinity_upgrade_tier_mult(tier);

        // Break Infinity Upgrade multipliers (all-tier): total/current-AM,
        // infinitied, and achievement multipliers.
        mult *= self.break_infinity_upgrade_common_mult();

        // Infinity Challenge all-tier multipliers (IC3 static / IC8 decay / IC6
        // matter divide).
        mult *= self.infinity_challenge_common_mult();

        // Infinity Power (from the Infinity Dimensions) gives an `^7` all-tier
        // multiplier (`infinityPower.pow(powerConversionRate).max(1)`) — except
        // under EC9, where it multiplies Time Dimensions instead.
        if !self.ec_running(9) {
            mult *= self.infinity_power_ad_multiplier();
        }
        // EC10's restriction-side boost: an immense multiplier from Infinities.
        if self.ec_running(10) {
            mult *= self.ec10_ad_multiplier();
        }

        // `applyNDMultipliers` clamps the multiplier to >= 1 (so e.g. IC6's divide
        // and a tiny `totalTimeMult` cannot push it below the base)...
        let mult = mult.max(&Decimal::ONE);

        // ...then `applyNDPowers` raises it to the IC4 power (`^0.25` for the
        // non-latest dimensions while IC4 runs, `^1.05` all-tier once completed).
        let power = self.infinity_challenge_mult_power(tier);
        let mut mult = if power == 1.0 {
            mult
        } else {
            mult.pow(&Decimal::from_float(power))
        };

        // Time Dilation compresses the final multiplier; the `ndMultDT`
        // Dilation Upgrade applies *after* (unaffected by dilation).
        if self.dilation.active {
            mult = self.dilated_value_of(mult);
        }
        if self.dilation_upgrade_bought(6) {
            mult *= self
                .dilation
                .dilated_time
                .pow(&Decimal::from_float(308.0))
                .max(&Decimal::ONE);
        }
        mult
    }

    /// Get the per-second production rate for a dimension tier.
    /// Each dimension produces the tier below it (AD8 produces
    /// AD7, ..., AD1 produces antimatter).
    /// Production = amount * multiplier * tickspeed_effect
    pub fn dimension_production_per_second(&self, tier: usize) -> Decimal {
        if tier >= 8 || !self.is_dimension_unlocked(tier) {
            return Decimal::ZERO;
        }
        // EC3: Antimatter Dimensions 5–8 don't produce anything.
        if self.ec_running(3) && tier > 3 {
            return Decimal::ZERO;
        }

        let mut amount = self.dimensions[tier].amount;
        // Normal Challenge 12 strengthens the even dimensions (2nd/4th/6th) to
        // compensate for producing 2 tiers below (tiers are 0-indexed here).
        if self.challenge_running(12) {
            let exponent = match tier {
                1 => Some(1.6),
                3 => Some(1.4),
                5 => Some(1.2),
                _ => None,
            };
            if let Some(exp) = exponent {
                amount = amount.pow(&Decimal::from_float(exp));
            }
        }
        let multiplier = self.dimension_multiplier(tier);
        let tickspeed_effect = self.tickspeed_effect();
        let mut production = amount * multiplier * tickspeed_effect;

        // Normal Challenge 2 halts all production by the recovering `chall2Pow`
        // factor (0 right after a purchase, back to 1 over 3 minutes).
        if self.challenge_running(2) {
            production *= Decimal::from_float(self.chall2_pow);
        }
        // Normal Challenge 3 multiplies the 1st dimension by its uncapped
        // exponential `chall3Pow` (which also weakens it to ×0.01 at the start).
        if tier == 0 && self.challenge_running(3) {
            production *= self.chall3_pow;
        }

        production
    }
}
