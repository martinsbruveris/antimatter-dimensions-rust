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
        let purchase_group = (self.dimensions[tier].bought / 10
            + self.dimensions[tier].cost_bumps) as f64;
        self.ad_cost_scale(base, mult)
            .calculate_cost(purchase_group)
    }

    /// `Player.dimensionMultDecrease`: the AD cost-scale (`costScale`), reduced by
    /// the `dimCostMult` Break Infinity Upgrade (−1 per purchase) and EC6
    /// completions (−0.2 each).
    pub fn dimension_mult_decrease(&self) -> f64 {
        10.0 - self.infinity_rebuyables[1] as f64
            - self.eternity_challenge_completions(6) as f64 * 0.2
    }

    /// The Antimatter Dimension purchase-cost curve for a tier's `(base, mult)`.
    fn ad_cost_scale(&self, base: f64, mult: f64) -> crate::cost_scaling::CostScale {
        crate::cost_scaling::CostScale::new(
            base,
            mult,
            self.dimension_mult_decrease(),
            crate::cost_scaling::LOG10_NUMBER_MAX_VALUE,
        )
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
        if !self.buy_one_dimension(tier) {
            return false;
        }
        // 28: buy a *single* 1st dimension while holding over 1e150 of them. The
        // original checks this only in `buyOneDimension`, never on the bulk
        // "buy max"/until-ten paths — which reuse `buy_one_dimension` directly.
        if tier == 0 && self.dimensions[0].amount.exponent() >= 150 {
            self.unlock_achievement(28);
        }
        true
    }

    /// The core single-dimension purchase, shared by the genuine "buy one"
    /// entry point ([`buy_dimension`](Self::buy_dimension)) and the group
    /// completion loop ([`buy_until_10_dimension`](Self::buy_until_10_dimension)).
    /// Mirrors the body of the original's `buyOneDimension` *minus* its
    /// `Achievement(28)` unlock, which fires only on the genuine single buy.
    fn buy_one_dimension(&mut self, tier: usize) -> bool {
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

        // Reality-upgrade requirement flags (`antimatter-dimension.js`): buying a
        // non-8th tier breaks "only AD8", a non-1st breaks "only AD1", and buying
        // AD1 breaks "no AD1" (tier is 0-indexed: AD1 == 0, AD8 == 7).
        if tier != 7 {
            self.requirement_checks.eternity_only_ad8 = false;
        }
        if tier != 0 {
            self.requirement_checks.eternity_only_ad1 = false;
        }
        if tier == 0 {
            self.requirement_checks.eternity_no_ad1 = false;
        }
        if tier == 7 {
            self.requirement_checks.infinity_no_ad8 = false;
        }

        // 11–18: buy a 1st..8th Antimatter Dimension (tier is 0-indexed).
        self.unlock_achievement(11 + tier as u16);
        // 23: have exactly 99 eighth dimensions (only buying AD8 can reach it).
        if tier == 7 && self.dimensions[7].amount == Decimal::from_float(99.0) {
            self.unlock_achievement(23);
        }
    }

    /// Buy the maximum number of the specified dimension tier that can be
    /// afforded. Mirrors the original `buyMaxDimension(tier)` (unbounded bulk).
    /// Returns the number of dimensions bought.
    pub fn buy_max_dimension(&mut self, tier: usize) -> u64 {
        self.buy_max_dimension_bulk(tier, f64::INFINITY)
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
            if !self.buy_one_dimension(tier) {
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

    /// Whether the whole next group of 10 is affordable (`isAffordableUntil10`):
    /// the currency covers `costUntil10`, not merely a single purchase.
    pub fn dim_affordable_until_10(&self, tier: usize) -> bool {
        self.dim_currency_amount(tier) >= self.dimension_cost_until_10(tier)
    }

    /// Whether a *single* purchase of `tier` is affordable (`isAffordable`): the
    /// spending currency covers one unit's `cost`. Mirrors the original's guards —
    /// never while Continuum is active, and pre-break not once the cost passes
    /// `NUMBER_MAX_VALUE`. This is the AD autobuyer's `canTick` readiness gate (it
    /// checks the single cost even in "Buys max" mode).
    pub fn dim_single_affordable(&self, tier: usize) -> bool {
        if self.continuum_active() {
            return false;
        }
        let cost = self.dimension_cost(tier);
        if !self.broke_infinity && cost > Decimal::NUMBER_MAX_VALUE {
            return false;
        }
        self.dim_currency_amount(tier) >= cost
    }

    /// Faithful port of the original `buyMaxDimension(tier, bulk)` — the "Buys
    /// max" (`BUY_10`) action for both the manual button (`bulk = ∞`) and the AD
    /// autobuyer (`bulk` = its bulk-multiplier setting). It always buys in
    /// *complete groups of ten*, never leaving a partial group:
    ///
    ///  1. Bail unless the whole current group is affordable (`isAffordableUntil10`).
    ///  2. Complete the current group (one "set"), consuming one unit of bulk.
    ///  3. Bulk-buy further complete groups, up to the remaining bulk. In the
    ///     normal regime this is the analytic `getMaxBought` (below), charging only
    ///     for the most expensive group obtained — the original's deliberate bulk
    ///     simplification — which also makes huge buys O(1) rather than a loop.
    ///     Under NC9 / IC5 the per-group cost bumps are abnormal, so it instead
    ///     loops group-by-group (like the original) to trigger them.
    ///
    /// Returns the number of dimensions bought.
    pub fn buy_max_dimension_bulk(&mut self, tier: usize, bulk: f64) -> u64 {
        if !self.dim_available_for_purchase(tier) || !self.dim_affordable_until_10(tier)
        {
            return 0;
        }
        // In an antimatter challenge, a group whose single cost already exceeds the
        // Big Crunch goal is not bought (`dimension.cost.gt(goal) && isInChallenge`).
        let goal = self.infinity_goal();
        if self.dimension_cost(tier) > goal && self.in_any_antimatter_challenge() {
            return 0;
        }

        // Complete the current group (the original's "buy any remaining until 10
        // before attempting to bulk-buy"). The affordability check above guarantees
        // every buy in it succeeds, landing on the next multiple of ten. Unlike the
        // manual "buy until 10" (`buyManyDimension`), `buyMaxDimension` finishes the
        // group via `buyUntilTen`, which *rounds* the dimension amount — so mirror
        // that here (the fractional stock from production would otherwise linger).
        let mut count = self.buy_until_10_dimension(tier);
        self.dimensions[tier].amount = self.dimensions[tier].amount.round();
        let mut bulk_left = bulk - 1.0;
        if bulk_left <= 0.0 {
            return count;
        }

        // NC9 (equal-cost) / IC5 (per-tier) impose abnormal per-group cost bumps,
        // so bulk-buying must proceed one group at a time to apply them.
        if self.challenge_running(9) || self.infinity_challenge_running(5) {
            while bulk_left > 0.0
                && self.dim_affordable_until_10(tier)
                && self.dimension_cost(tier) < goal
            {
                count += self.buy_until_10_dimension(tier);
                self.dimensions[tier].amount = self.dimensions[tier].amount.round();
                bulk_left -= 1.0;
            }
            return count;
        }

        // Normal regime: analytic bulk buy via `getMaxBought` (10-per-set), which
        // handles both the geometric branch and the super-exponential scaling past
        // `Number.MAX_VALUE`.
        let (base, mult) = if self.challenge_running(6) {
            (C6_AD_BASE_COSTS[tier], C6_AD_COST_MULTIPLIERS[tier])
        } else {
            (AD_BASE_COSTS[tier], AD_COST_MULTIPLIERS[tier])
        };
        let p0 = (self.dimensions[tier].bought / 10 + self.dimensions[tier].cost_bumps)
            as f64;
        let money = self.dim_currency_amount(tier);
        let (quantity, log_price) = match self
            .ad_cost_scale(base, mult)
            .get_max_bought(p0, money, 10.0)
        {
            Some(q) => q,
            None => return count,
        };
        // Clamp to the remaining bulk (the manual path passes ∞, so never clamps).
        // The original charges the price of the *unclamped* top group regardless of
        // clamping (`Decimal.pow10(maxBought.logPrice)`), so `log_price` is unchanged.
        let buying = quantity.min(bulk_left) as i64;
        let bought = 10 * buying;
        let amount = self.dimensions[tier].amount + Decimal::from_float(bought as f64);
        self.dimensions[tier].amount = amount.round();
        self.dimensions[tier].bought += bought as u64;
        self.spend_dim_currency(tier, Decimal::pow10(log_price));
        count + bought as u64
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
        // `buy10Mult` Infinity Upgrade (`buy_ten_multiplier`). Lai'tela's
        // Continuum replaces the discrete buy-10 count with a continuous value.
        let buy10_value = if self.continuum_active() {
            self.ad_continuum_value(tier)
        } else {
            (self.dimensions[tier].bought / 10) as f64
        };
        if buy10_value > 0.0 {
            mult *= self
                .buy_ten_multiplier()
                .pow(&Decimal::from_float(buy10_value));
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
        // dimension; the rest are per-tier or all-tier multipliers ported from
        // `antimatterDimensionCommonMultiplier` / `applyNDMultipliers`.
        mult *= self.achievement_power();
        mult *= self.achievement_ad_common_mult();
        // 1st dimension (tier 0): 28 / 31 / 68 / 71.
        if tier == 0 {
            if self.achievement_unlocked(28) {
                mult *= Decimal::from_float(1.1);
            }
            if self.achievement_unlocked(31) {
                mult *= Decimal::from_float(1.05);
            }
            if self.achievement_unlocked(68) {
                mult *= Decimal::from_float(1.5);
            }
            if self.achievement_unlocked(71) {
                mult *= Decimal::from_float(3.0);
            }
        }
        // 8th dimension (tier 7): 23.
        if tier == 7 && self.achievement_unlocked(23) {
            mult *= Decimal::from_float(1.1);
        }
        // 34: Antimatter Dimensions 1–7 (tier < 8) ×1.02.
        if tier < 7 && self.achievement_unlocked(34) {
            mult *= Decimal::from_float(1.02);
        }
        // 64: Antimatter Dimensions 1–4 (tier ≤ 3) ×1.25.
        if tier <= 3 && self.achievement_unlocked(64) {
            mult *= Decimal::from_float(1.25);
        }
        // Infinity Challenge 8's completion reward: Antimatter Dimensions 2–7
        // (0-indexed 1–6) gain `(AD1.multiplier × AD8.multiplier)^0.02`, using the
        // 1st and 8th dimensions' *final* multipliers (which never include this
        // reward themselves, so there is no recursion).
        if (1..=6).contains(&tier) && self.infinity_challenge_completed(8) {
            let base = self.dimension_multiplier(0) * self.dimension_multiplier(7);
            mult *= base.pow(&Decimal::from_float(0.02));
        }
        // 43: every dimension gains a boost proportional to its (1-indexed) tier.
        if self.achievement_unlocked(43) {
            mult *= Decimal::from_float(1.0 + (tier as f64 + 1.0) / 100.0);
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

        // The `powermult` glyph effect (`applyNDMultipliers`).
        mult *= self.glyph_effect_powermult();

        // Ra Alchemy: `dimensionality` (×10^(5·amount) all-dim) and `force`
        // (×RM^(5·amount)).
        mult *= Decimal::pow10(self.alchemy_dimensionality_log10());
        let force = self.alchemy_force();
        if force != 0.0 {
            mult *= self.reality.machines.pow(&Decimal::from_float(force));
        }

        // Pelle: the `antimatterDimensionMult` rebuyable (while doomed).
        if self.is_doomed() {
            mult *= self.pelle_ad_mult();
        }

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
        // non-latest dimensions while IC4 runs, `^1.05` all-tier once completed)
        // and the `powerpow` glyph power.
        let power = self.infinity_challenge_mult_power(tier);
        let mut mult = if power == 1.0 {
            mult
        } else {
            mult.pow(&Decimal::from_float(power))
        };
        // Ra's `momentumValue` folds into the glyph power exponent
        // (`applyNDPowers`).
        let glyph_pow = self.glyph_effect_powerpow() * self.ra_momentum_value();
        if glyph_pow != 1.0 {
            mult = mult.pow(&Decimal::from_float(glyph_pow));
        }
        // Ra Alchemy `power`: AD multiplier `^(1 + amount/200000)`.
        let alch_power = self.alchemy_dimension_power(crate::celestials::alchemy::POWER);
        if alch_power != 1.0 {
            mult = mult.pow(&Decimal::from_float(alch_power));
        }
        // V's `adPow` reward: a persistent AD power `1 + √ST/100`
        // (`applyNDPowers`' `VUnlocks.adPow`).
        let ad_pow = self.v_ad_pow();
        if ad_pow != 1.0 {
            mult = mult.pow(&Decimal::from_float(ad_pow));
        }
        // Pelle: the Infinity Strike raises AD multipliers `^0.5`; the Paradox
        // rift gives an all-Dimension power.
        if self.is_doomed() {
            if self.pelle_has_strike(1) {
                mult = mult.pow(&Decimal::from_float(0.5));
            }
            if self.pelle_rift_unlocked(crate::celestials::pelle::RIFT_PARADOX) {
                mult = mult.pow(
                    &self.pelle_rift_effect(crate::celestials::pelle::RIFT_PARADOX),
                );
            }
        }

        // Time Dilation compresses the final multiplier (raised to the
        // `dilationpow` glyph power first); the `ndMultDT` Dilation Upgrade
        // applies *after* (unaffected by dilation).
        if self.dilation.active {
            let dilation_pow = self.glyph_effect_dilationpow();
            if dilation_pow != 1.0 {
                mult = mult.pow(&Decimal::from_float(dilation_pow));
            }
            mult = self.dilated_value_of(mult);
        } else if self.celestials.enslaved.run {
            // Enslaved keeps AD multipliers "always dilated".
            mult = self.dilated_value_of(mult);
        }
        if self.dilation_upgrade_bought(6) {
            mult *= self
                .dilation
                .dilated_time
                .pow(&Decimal::from_float(308.0))
                .max(&Decimal::ONE);
        }

        // Celestial-run final-stage transforms (mutually exclusive, after all
        // nerfs): Effarig compresses via `multiplier`, V square-roots.
        if self.celestials.effarig.run {
            mult = self.effarig_multiplier(mult);
        } else if self.celestials.v.run {
            mult = mult.pow(&Decimal::from_float(0.5));
        }

        // Ra Alchemy `inflation`: AD multipliers above `10^threshold` are raised
        // `^1.05`. Intentionally after all nerfs (matches the original comment).
        if let Some(threshold_log) = self.alchemy_inflation_log10() {
            if mult.pos_log10() >= threshold_log {
                mult = mult.pow(&Decimal::from_float(1.05));
            }
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
        // Lai'tela's Reality disables dimensions above `maxAllowedDimension`.
        if self.laitela_dimension_disabled((tier + 1) as u32) {
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

        // `cappedProductionInNormalChallenges`: unless post-break (Infinity broken
        // and outside a Normal Challenge, or inside an Infinity Challenge, or in
        // Enslaved's Reality), each dimension's per-second production is capped at
        // 1e315. This bounds the pre-break Big-Crunch overshoot (`maxAM`).
        let post_break = (self.broke_infinity && self.challenge.current == 0)
            || self.infinity_challenge.current != 0
            || self.celestials.enslaved.run;
        if !post_break {
            production = production.min(&Decimal::new_unchecked(1.0, 315));
        }

        production
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "Buys max" at bulk 1 (the autobuyer's default) must complete a group of ten
    /// atomically: with only part of the group affordable it buys nothing (never
    /// leaving a partial group), matching the original's `isAffordableUntil10`
    /// guard. Buying one-at-a-time here would wrongly stall mid-group.
    #[test]
    fn buy_max_bulk_one_is_all_or_nothing() {
        let mut game = GameState::new();
        // Sit exactly on a group boundary so the next group costs `cost × 10`.
        game.dimensions[0].bought = 10;
        let cost = game.dimension_cost(0);

        // Enough for five individual buys but not the whole group: nothing happens.
        game.antimatter = cost * Decimal::from_float(5.0);
        assert_eq!(game.buy_max_dimension_bulk(0, 1.0), 0);
        assert_eq!(game.dimensions[0].bought, 10);

        // The whole group affordable: it completes to the next multiple of ten.
        game.antimatter = cost * Decimal::from_float(10.0);
        assert_eq!(game.buy_max_dimension_bulk(0, 1.0), 10);
        assert_eq!(game.dimensions[0].bought, 20);
    }

    /// Bulk caps the number of groups: with ample antimatter, `bulk = 2` completes
    /// the current group plus one more (20 dimensions), not everything affordable.
    #[test]
    fn buy_max_bulk_caps_group_count() {
        let mut game = GameState::new();
        game.antimatter = Decimal::from_float(1e9);
        // From a clean start, bulk 2 buys exactly two groups of ten.
        assert_eq!(game.buy_max_dimension_bulk(0, 2.0), 20);
        assert_eq!(game.dimensions[0].bought, 20);
    }

    /// The Infinity Challenge 8 completion reward multiplies only the 2nd–7th
    /// Antimatter Dimensions by `(AD1.mult × AD8.mult)^0.02`.
    #[test]
    fn ic8_reward_multiplies_only_ad2_through_ad7() {
        let mut game = GameState::new();
        // Non-trivial 1st/8th multipliers so the reward is meaningfully > 1.
        game.broke_infinity = true;
        game.dimensions[0].bought = 100;
        game.dimensions[7].bought = 100;
        game.dim_boosts = 5;

        // Toggle only the IC8-completed bit, so no achievement side effects shift
        // the baseline. With the reward:
        game.infinity_challenge.completed |= 1u16 << 8;
        let with: Vec<f64> = (0..8)
            .map(|t| game.dimension_multiplier(t).to_f64())
            .collect();
        let reward = (game.dimension_multiplier(0) * game.dimension_multiplier(7))
            .pow(&Decimal::from_float(0.02))
            .to_f64();
        assert!(reward > 1.0, "reward={reward}");

        // Without it:
        game.infinity_challenge.completed &= !(1u16 << 8);
        let without: Vec<f64> = (0..8)
            .map(|t| game.dimension_multiplier(t).to_f64())
            .collect();

        // AD1 (tier 0) and AD8 (tier 7) do not receive the reward.
        assert!((with[0] / without[0] - 1.0).abs() < 1e-9);
        assert!((with[7] / without[7] - 1.0).abs() < 1e-9);
        // AD2–AD7 (tiers 1–6) each gain exactly the reward factor.
        for t in 1..=6 {
            let ratio = with[t] / (without[t] * reward);
            assert!((ratio - 1.0).abs() < 1e-9, "tier {t}: ratio={ratio}");
        }
    }

    /// Pre-break (outside a challenge), each dimension's per-second production is
    /// capped at 1e315 (`cappedProductionInNormalChallenges`); the cap lifts once
    /// Infinity is broken.
    #[test]
    fn ad_production_capped_at_1e315_until_break() {
        let mut game = GameState::new();
        // A 1st dimension huge enough that raw production far exceeds the cap.
        game.dimensions[0].bought = 100;
        game.dimensions[0].amount = Decimal::new(1.0, 400);

        assert!(!game.broke_infinity);
        assert_eq!(
            game.dimension_production_per_second(0),
            Decimal::new(1.0, 315)
        );

        // Post-break the cap lifts and production runs uncapped.
        game.broke_infinity = true;
        assert!(game.dimension_production_per_second(0) > Decimal::new(1.0, 315));
    }

    /// Unbounded bulk buys complete groups only (lands on a multiple of ten) and,
    /// unlike a one-at-a-time loop, terminates in O(1) even for enormous balances.
    #[test]
    fn buy_max_unbounded_lands_on_group_boundary() {
        let mut game = GameState::new();
        game.antimatter = Decimal::new(1.0, 300);
        let bought = game.buy_max_dimension(0);
        assert!(bought > 0);
        assert_eq!(bought % 10, 0);
        assert_eq!(game.dimensions[0].bought % 10, 0);
        // Charged no more than we had.
        assert!(game.antimatter >= Decimal::ZERO);
    }
}
