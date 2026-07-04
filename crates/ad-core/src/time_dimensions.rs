//! Time Dimensions (Feature 4.3): 8 tiers bought with Eternity Points. TD8
//! feeds TD7 … feeds TD1, which produces **Time Shards**; shards convert into
//! free Tickspeed upgrades ([`GameState::update_free_tickspeed`], the
//! original's `FreeTickspeed.fromShards`). TDs persist across Eternities (only
//! their `amount` returns to the bought base); a full reset waits for Reality.
//!
//! Mirrors `src/core/dimensions/time-dimension.js` and the `FreeTickspeed`
//! block of `src/core/tickspeed.js`.
//!
//! **Frontier note:** tiers 5–8 are unlocked by *Dilation* studies 2–5 (Phase
//! 5), not Phase-4 time studies — the feature-decomposition doc is wrong there.
//! They are modelled and saved but not yet unlockable. See
//! `design-docs/2026-07-04-eternity.md` §3.

use break_infinity::Decimal;

use crate::state::GameState;

/// Number of Time Dimension tiers.
pub const TIME_DIMENSION_COUNT: usize = 8;

/// Base EP cost per tier (0-indexed).
const TD_BASE_COST: [Decimal; 8] = [
    Decimal::new_unchecked(1.0, 0),
    Decimal::new_unchecked(5.0, 0),
    Decimal::new_unchecked(1.0, 2),
    Decimal::new_unchecked(1.0, 3),
    Decimal::new_unchecked(1.0, 2350),
    Decimal::new_unchecked(1.0, 2650),
    Decimal::new_unchecked(1.0, 3000),
    Decimal::new_unchecked(1.0, 3350),
];

/// EP cost multiplier per purchase (0-indexed).
const TD_COST_MULT: [f64; 8] =
    [3.0, 9.0, 27.0, 81.0, 24300.0, 72900.0, 218700.0, 656100.0];

/// Purchase count where the `e6000` scaling starts (0-indexed tier).
const TD_E6000_SCALING_AMOUNT: [u64; 8] = [7322, 4627, 3382, 2665, 833, 689, 562, 456];

/// Cost thresholds walked with multiplier bumps ×[1, 1.5, 2.2].
const TD_COST_THRESHOLDS: [Decimal; 3] = [
    Decimal::NUMBER_MAX_VALUE,
    Decimal::new_unchecked(1.0, 1300),
    Decimal::new_unchecked(1.0, 6000),
];
const TD_COST_MULT_INCREASES: [f64; 3] = [1.0, 1.5, 2.2];

/// Per-purchase production multiplier (`powerMultiplier`, pre-glyph).
const TD_POWER_MULT: f64 = 4.0;

/// The tier-8 purchase count cap that feeds the multiplier (`clampMax(1e8)`).
const TD8_MULT_BOUGHT_CAP: u64 = 100_000_000;

/// Cost exponent scaling factor past 1e6000 (`TimeDimensions.scalingPast1e6000`).
const TD_SCALING_PAST_E6000: u64 = 4;

/// Free-tickspeed softcap (`FreeTickspeed.BASE_SOFTCAP`).
const FREE_TICKSPEED_SOFTCAP: f64 = 300_000.0;
/// Post-softcap cost-growth constants (`GROWTH_RATE`, `GROWTH_EXP`).
const FREE_TICKSPEED_GROWTH_RATE: f64 = 6e-6;
const FREE_TICKSPEED_GROWTH_EXP: f64 = 2.0;

/// One Time Dimension tier's mutable state (`player.dimensions.time[t]`).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimeDimension {
    /// Current amount (grows from higher-tier production; back to `bought` on
    /// an Eternity).
    pub amount: Decimal,
    /// Purchases made. Persists across Eternities.
    pub bought: u64,
    /// Next-purchase EP cost (derived from `bought`, stored like the original).
    pub cost: Decimal,
}

impl TimeDimension {
    /// A fresh tier (0-indexed): unbought, at its base cost.
    pub fn new(tier: usize) -> Self {
        Self {
            amount: Decimal::ZERO,
            bought: 0,
            cost: TD_BASE_COST[tier],
        }
    }
}

/// The EP cost of the `bought+1`-th purchase of tier `t` (0-indexed). Mirrors
/// `TimeDimensionState.nextCost` (pre-Pelle).
pub fn time_dimension_cost(tier: usize, bought: u64) -> Decimal {
    let base = TD_BASE_COST[tier];
    let mult = TD_COST_MULT[tier];
    let scaling_amount = TD_E6000_SCALING_AMOUNT[tier];

    // Tiers 5–8 below their scaling amount: plain geometric cost.
    if tier >= 4 && bought < scaling_amount {
        return Decimal::from_float(mult).pow(&Decimal::from(bought)) * base;
    }

    // Walk the thresholds with the bumped multipliers; the first fit wins.
    for (i, threshold) in TD_COST_THRESHOLDS.iter().enumerate() {
        let cost = Decimal::from_float(mult * TD_COST_MULT_INCREASES[i])
            .pow(&Decimal::from(bought))
            * base;
        if cost < *threshold {
            return cost;
        }
    }

    // Past 1e6000: exponent stretches ×4 beyond the scaling amount.
    let mut stepped = mult;
    if tier < 4 {
        stepped *= 2.2;
    }
    let exponent = scaling_amount + (bought - scaling_amount) * TD_SCALING_PAST_E6000;
    Decimal::from_float(stepped).pow(&Decimal::from(exponent)) * base
}

impl GameState {
    /// Whether tier `t` (0-indexed) is unlocked: tiers 1–4 always; 5–8 need
    /// their Dilation study (Phase 5, so currently locked).
    pub fn td_is_unlocked(&self, tier: usize) -> bool {
        tier < 4
    }

    /// Whether tier `t` can be bought now (unlocked + affordable).
    pub fn td_available_for_purchase(&self, tier: usize) -> bool {
        self.td_is_unlocked(tier)
            && self.eternity_points >= self.time_dimensions[tier].cost
    }

    /// Buy a single Time Dimension of tier `t` (`buySingleTimeDimension`).
    pub fn buy_time_dimension(&mut self, tier: usize) -> bool {
        if !self.td_available_for_purchase(tier) {
            return false;
        }
        let cost = self.time_dimensions[tier].cost;
        self.eternity_points -= cost;
        let d = &mut self.time_dimensions[tier];
        d.amount += Decimal::ONE;
        d.bought += 1;
        d.cost = time_dimension_cost(tier, d.bought);
        true
    }

    /// Buy as many of tier `t` as affordable. Returns the number bought.
    pub fn buy_max_time_dimension(&mut self, tier: usize) -> u64 {
        let mut count = 0;
        while self.buy_time_dimension(tier) {
            count += 1;
        }
        count
    }

    /// "Max All" (`maxAllTimeDimensions`, approximated like our other max-alls
    /// by cheapest-first repeated singles).
    pub fn max_all_time_dimensions(&mut self) {
        // Buy singles of the highest affordable new dimensions first.
        for tier in (0..TIME_DIMENSION_COUNT).rev() {
            if self.time_dimensions[tier].bought == 0 {
                self.buy_time_dimension(tier);
            }
        }
        // Then repeatedly buy the cheapest purchasable dimension.
        loop {
            let cheapest = (0..TIME_DIMENSION_COUNT)
                .filter(|&t| self.td_is_unlocked(t))
                .min_by(|&a, &b| {
                    self.time_dimensions[a]
                        .cost
                        .partial_cmp(&self.time_dimensions[b].cost)
                        .unwrap()
                });
            let Some(tier) = cheapest else { break };
            if !self.buy_time_dimension(tier) {
                break;
            }
        }
    }

    /// The all-tier Time Dimension multiplier
    /// (`timeDimensionCommonMultiplier`): Time Studies 93/103/151/221.
    /// Eternity Upgrades 4–6 join with Feature 4.6, the EC1/EC10 rewards with
    /// 4.5.
    pub(crate) fn td_common_multiplier(&self) -> Decimal {
        let mut mult = Decimal::ONE;
        // TS93: tick upgrades gained ^0.25 (min 1).
        if self.time_study_bought(93) {
            mult *= Decimal::from(self.total_tick_gained)
                .pow(&Decimal::from_float(0.25))
                .max(&Decimal::ONE);
        }
        // TS103: equal to the Replicanti Galaxy count (min 1).
        if self.time_study_bought(103) {
            mult *= Decimal::from((self.replicanti.galaxies as u64).max(1));
        }
        // TS151: flat ×1e4.
        if self.time_study_bought(151) {
            mult *= Decimal::from_float(1e4);
        }
        // TS221: based on Dimension Boosts.
        if self.time_study_bought(221) {
            mult *=
                Decimal::from_float(1.0025).pow(&Decimal::from(self.dim_boosts as u64));
        }
        // Eternity Upgrades 4–6 (achievement power / unspent TT / days played).
        mult *= self.eternity_upgrade_td_mult();
        // EC1's reward: TD multiplier from time spent this Eternity.
        if self.ec_completed(1) {
            let completions = self.eternity_challenge_completions(1) as f64;
            let base = (self.records.this_eternity.time_ms / 10.0).max(0.9);
            mult *= Decimal::from_float(base)
                .pow(&Decimal::from_float(0.3 + completions * 0.05));
        }
        // EC10's reward: TD multiplier from Infinities.
        if self.ec_completed(10) {
            mult *= self.ec10_reward_td_mult();
        }
        // EC9 (running): Infinity Power multiplies Time Dimensions instead of
        // Antimatter Dimensions, with a greatly reduced effect.
        if self.ec_running(9) {
            let log2_power = (self.infinity_power.max(&Decimal::ONE).ln()
                / std::f64::consts::LN_2)
                .max(1.0);
            mult *= Decimal::from_float(log2_power.powi(4)).max(&Decimal::ONE);
        }
        mult
    }

    /// Tier `t`'s production multiplier: `commonMult × 4^bought` (tier 8's
    /// `bought` capped at 1e8). The per-tier TS11/TS73/TS227 terms are later
    /// features.
    pub fn td_multiplier(&self, tier: usize) -> Decimal {
        let mut bought = self.time_dimensions[tier].bought;
        if tier == TIME_DIMENSION_COUNT - 1 {
            bought = bought.min(TD8_MULT_BOUGHT_CAP);
        }
        let mut mult = self.td_common_multiplier()
            * Decimal::from_float(TD_POWER_MULT).pow(&Decimal::from(bought));
        // Per-tier studies: TS11 (tier 1, tickspeed-based), TS73 (tier 3,
        // sacrifice^0.005 cap 1e1300), TS227 (tier 4, sacrifice-log^10).
        if tier == 0 && self.time_study_bought(11) {
            mult *= self.ts11_effect();
        }
        if tier == 2 && self.time_study_bought(73) {
            mult *= self
                .sacrifice_multiplier()
                .pow(&Decimal::from_float(0.005))
                .max(&Decimal::ONE)
                .min(&Decimal::new_unchecked(1.0, 1300));
        }
        if tier == 3 && self.time_study_bought(227) {
            let log = self.sacrifice_multiplier().pos_log10();
            mult *= Decimal::from_float(log.powi(10).max(1.0));
        }
        mult
    }

    /// Tier `t`'s production per second (`amount × multiplier`; EC-modified).
    pub fn td_production_per_second(&self, tier: usize) -> Decimal {
        // EC1/EC10: Time Dimensions are disabled.
        if self.ec_running(1) || self.ec_running(10) {
            return Decimal::ZERO;
        }
        // EC11: production without any multiplier.
        if self.ec_running(11) {
            return self.time_dimensions[tier].amount;
        }
        let mut production =
            self.time_dimensions[tier].amount * self.td_multiplier(tier);
        // EC7: Tickspeed directly applies to Time Dimensions.
        if self.ec_running(7) {
            production *= self.tickspeed_effect();
        }
        production
    }

    /// Advance Time Dimension production (`TimeDimensions.tick`): each tier
    /// feeds the tier below at `diff/10`; TD1 produces Time Shards at `diff`.
    /// Afterwards the shards are converted into free Tickspeed upgrades.
    pub(crate) fn tick_time_dimensions(&mut self, dt_ms: f64) {
        let dt_s = dt_ms / 1000.0;
        let prod: [Decimal; TIME_DIMENSION_COUNT] =
            std::array::from_fn(|t| self.td_production_per_second(t));

        let dt10 = Decimal::from_float(dt_s / 10.0);
        for tier in (1..TIME_DIMENSION_COUNT).rev() {
            self.time_dimensions[tier - 1].amount += prod[tier] * dt10;
        }
        // EC7 (running): TD1 produces 8th Infinity Dimensions instead of
        // Time Shards. EC7's reward does the same passively once completed.
        if self.ec_running(7) {
            self.infinity_dimensions[7].amount += prod[0] * Decimal::from_float(dt_s);
        } else {
            self.time_shards += prod[0] * Decimal::from_float(dt_s);
        }
        if self.ec_completed(7) {
            self.infinity_dimensions[7].amount +=
                self.ec7_reward_id8_per_second() * Decimal::from_float(dt_s);
        }

        self.update_free_tickspeed();
    }

    /// The free-tickspeed cost multiplier between upgrades (`FreeTickspeed
    /// .multToNext` base): 1.33, improved to 1.25 by Time Study 171.
    pub fn free_tickspeed_mult(&self) -> f64 {
        if self.time_study_bought(171) {
            1.25
        } else {
            1.33
        }
    }

    /// Convert Time Shards into the total free-Tickspeed-upgrade count
    /// (`FreeTickspeed.fromShards`) and take any gain (the game-loop's
    /// `totalTickGained += clampMin(newAmount - totalTickGained, 0)`).
    pub(crate) fn update_free_tickspeed(&mut self) {
        let (new_amount, _) =
            free_tickspeed_from_shards(self.time_shards, self.free_tickspeed_mult());
        if new_amount > self.total_tick_gained {
            self.total_tick_gained = new_amount;
        }
    }

    /// The shard total the *next* free Tickspeed upgrade needs
    /// (`fromShards(...).nextShards`), for the Time Dimensions tab readout.
    pub fn next_free_tickspeed_shards(&self) -> Decimal {
        free_tickspeed_from_shards(self.time_shards, self.free_tickspeed_mult()).1
    }

    /// On an Eternity: each tier's amount returns to its bought base and costs
    /// are rebuilt (`resetTimeDimensions` + `updateTimeDimensionCosts`);
    /// purchases persist.
    pub(crate) fn reset_time_dimension_amounts(&mut self) {
        for tier in 0..TIME_DIMENSION_COUNT {
            let d = &mut self.time_dimensions[tier];
            d.amount = Decimal::from(d.bought);
            d.cost = time_dimension_cost(tier, d.bought);
        }
    }
}

/// `FreeTickspeed.fromShards`: the total free Tickspeed upgrades a shard total
/// is worth plus the shard threshold of the next upgrade. Below the 300 000
/// softcap the count is `ceil(ln(shards) / ln(mult))`; past it, upgrades follow
/// the quadratic cost curve `cost(n) = c·n² + n` (in implicit post-cap units),
/// inverted with Newton's method exactly like the original.
fn free_tickspeed_from_shards(shards: Decimal, tickmult: f64) -> (u64, Decimal) {
    let log_tickmult = tickmult.ln();
    if shards <= Decimal::ZERO {
        return (0, Decimal::ONE);
    }
    let log_shards = shards.ln();
    let uncapped = (log_shards / log_tickmult).max(0.0);
    if uncapped <= FREE_TICKSPEED_SOFTCAP {
        let count = uncapped.ceil();
        let next = Decimal::from_float(tickmult).pow(&Decimal::from_float(count));
        return (count as u64, next);
    }

    // Log of (cost - cost up to softcap); costs implicitly transformed by
    // (ln(x) - priceToCap) / logTickmult.
    let price_to_cap = FREE_TICKSPEED_SOFTCAP * log_tickmult;
    let desired_cost = (log_shards - price_to_cap) / log_tickmult;
    let coeff = FREE_TICKSPEED_GROWTH_RATE / FREE_TICKSPEED_GROWTH_EXP / log_tickmult;
    let bought_to_cost =
        |bought: f64| coeff * bought.max(0.0).powf(FREE_TICKSPEED_GROWTH_EXP) + bought;
    let derivative = |x: f64| {
        FREE_TICKSPEED_GROWTH_EXP
            * coeff
            * x.max(0.0).powf(FREE_TICKSPEED_GROWTH_EXP - 1.0)
            + 1.0
    };
    let newton = |bought: f64| {
        bought - (bought_to_cost(bought) - desired_cost) / derivative(bought)
    };

    let mut approximation =
        desired_cost.min((desired_cost / coeff).powf(1.0 / FREE_TICKSPEED_GROWTH_EXP));
    let mut old_approximation;
    let mut counter = 0;
    // Concave-upwards cost: successive Newton iterations from an over-estimate
    // stay over-estimates, so stop on non-progress (counter as a fallback).
    loop {
        old_approximation = approximation;
        approximation = newton(approximation);
        counter += 1;
        if approximation >= old_approximation || counter >= 100 {
            break;
        }
    }
    let purchases = approximation.floor();
    // Cost of the next upgrade, undoing the implicit transform.
    let next = Decimal::from_float(
        price_to_cap + bought_to_cost(purchases + 1.0) * log_tickmult,
    )
    .exp();
    ((purchases + FREE_TICKSPEED_SOFTCAP) as u64, next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn td_costs_follow_the_original_curve() {
        // TD1: 1, 3, 9, 27 EP...
        assert_eq!(time_dimension_cost(0, 0), Decimal::ONE);
        assert_eq!(time_dimension_cost(0, 1), Decimal::from_float(3.0));
        assert_eq!(time_dimension_cost(0, 2), Decimal::from_float(9.0));
        // TD5 starts at 1e2350 with ×24300 steps.
        assert_eq!(time_dimension_cost(4, 0), Decimal::new(1.0, 2350));
        let c1 = time_dimension_cost(4, 1);
        assert_eq!(c1, Decimal::new(1.0, 2350) * Decimal::from_float(24300.0));
    }

    #[test]
    fn td_cost_multiplier_bumps_past_thresholds() {
        // Past Number.MAX_VALUE the multiplier steps from 3 to 4.5 (×1.5).
        // 3^n × 1 crosses 1.8e308 around n = 646.
        let below = time_dimension_cost(0, 600);
        assert!(below < Decimal::NUMBER_MAX_VALUE);
        let above = time_dimension_cost(0, 700);
        // 4.5^700 ≈ 10^457.3 — the bumped-multiplier branch.
        assert_eq!(above, Decimal::from_float(4.5).pow(&Decimal::from(700u64)));
    }

    #[test]
    fn buy_time_dimension_spends_ep_and_scales_cost() {
        let mut game = GameState::new();
        game.eternity_points = Decimal::from_float(10.0);
        assert!(game.buy_time_dimension(0)); // costs 1 EP
        assert_eq!(game.eternity_points, Decimal::from_float(9.0));
        assert_eq!(game.time_dimensions[0].bought, 1);
        assert_eq!(game.time_dimensions[0].amount, Decimal::ONE);
        assert_eq!(game.time_dimensions[0].cost, Decimal::from_float(3.0));

        assert!(game.buy_time_dimension(0)); // 3 EP, leaving 6
                                             // The third costs 9 EP — more than the 6 left.
        assert!(!game.buy_time_dimension(0));
        assert_eq!(game.time_dimensions[0].bought, 2);
    }

    #[test]
    fn tiers_5_to_8_locked_pre_dilation() {
        let mut game = GameState::new();
        game.eternity_points = Decimal::new(1.0, 4000);
        for tier in 4..8 {
            assert!(!game.td_is_unlocked(tier));
            assert!(!game.buy_time_dimension(tier));
        }
    }

    #[test]
    fn td_multiplier_is_4x_per_purchase() {
        let mut game = GameState::new();
        game.eternity_points = Decimal::from_float(100.0);
        game.buy_time_dimension(0);
        game.buy_time_dimension(0);
        assert_eq!(game.td_multiplier(0), Decimal::from_float(16.0));
    }

    #[test]
    fn td1_produces_time_shards_and_free_tickspeed() {
        let mut game = GameState::new();
        game.eternity_points = Decimal::from_float(10.0);
        game.buy_time_dimension(0);

        // 1 TD1 × mult 4 for 10 s → 40 shards → floor... ceil(ln40/ln1.33) = 13.
        game.tick_time_dimensions(10_000.0);
        assert_eq!(game.time_shards, Decimal::from_float(40.0));
        let expected = (40.0f64.ln() / 1.33f64.ln()).ceil() as u64;
        assert_eq!(game.total_tick_gained, expected);

        // Free tickspeed upgrades speed up the game like bought ones.
        assert!(game.tickspeed_effect() > Decimal::ONE);
    }

    #[test]
    fn production_chain_feeds_lower_tiers() {
        let mut game = GameState::new();
        game.time_dimensions[1].amount = Decimal::from_float(10.0);
        game.tick_time_dimensions(1000.0);
        // TD2 (mult 1, nothing bought) feeds TD1 at amount × dt/10 = 1.
        assert_eq!(game.time_dimensions[0].amount, Decimal::ONE);
    }

    #[test]
    fn free_tickspeed_softcap_slows_growth() {
        // Below the cap: count = ceil(ln(shards)/ln(1.33)).
        let (below, next) = free_tickspeed_from_shards(Decimal::new(1.0, 300), 1.33);
        assert!(next > Decimal::new(1.0, 300));
        assert_eq!(below, ((300.0 * 10f64.ln()) / 1.33f64.ln()).ceil() as u64);

        // Far above the softcap the count grows much slower than ln-linear.
        let shards_at_cap = Decimal::from_float(1.33f64.ln() * 300_000.0).exp();
        let (above, _) =
            free_tickspeed_from_shards(shards_at_cap * Decimal::new(1.0, 10000), 1.33);
        let uncapped_equiv = 300_000.0 + (10_000.0 * 10f64.ln()) / 1.33f64.ln();
        assert!(above as f64 > FREE_TICKSPEED_SOFTCAP);
        assert!((above as f64) < uncapped_equiv);
    }

    #[test]
    fn eternity_resets_amounts_keeps_purchases() {
        let mut game = GameState::new();
        game.eternity_points = Decimal::from_float(100.0);
        game.buy_time_dimension(0);
        game.buy_time_dimension(0);
        game.time_dimensions[0].amount = Decimal::from_float(500.0);
        game.time_shards = Decimal::from_float(1e10);
        game.total_tick_gained = 50;

        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        assert!(game.eternity());

        assert_eq!(game.time_dimensions[0].bought, 2);
        assert_eq!(game.time_dimensions[0].amount, Decimal::from_float(2.0));
        assert_eq!(game.time_shards, Decimal::ZERO);
        assert_eq!(game.total_tick_gained, 0);
        // EP was spent but the eternity's EP reward still arrives.
        assert!(game.eternity_points > Decimal::ZERO);
    }
}
