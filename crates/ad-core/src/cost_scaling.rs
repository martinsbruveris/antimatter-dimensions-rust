//! Port of the original's `ExponentialCostScaling` (`core/math.js`): a cost curve
//! that is geometric (`base × increase^n`) until the cost passes a threshold
//! (`Number.MAX_VALUE` for Antimatter Dimensions and Tickspeed), after which the
//! per-purchase ratio itself grows by `cost_scale` each step (super-exponential).
//!
//! Antimatter Dimension and Tickspeed costs both use this; modelling only the
//! geometric branch made late-game (post-break) costs far too cheap, so the
//! autobuyers over-bought and production ran away by tens of orders of magnitude.

use break_infinity::Decimal;

/// `log10(Number.MAX_VALUE)` — the scaling threshold for AD/Tickspeed costs.
pub const LOG10_NUMBER_MAX_VALUE: f64 = 308.25471555991675;

/// A geometric-then-super-exponential cost curve. All fields are `log10` values,
/// matching the original's precomputation.
#[derive(Debug, Clone, Copy)]
pub struct CostScale {
    log_base_cost: f64,
    log_base_increase: f64,
    log_cost_scale: f64,
    purchases_before_scaling: f64,
    precalc_discriminant: f64,
    precalc_center: f64,
}

impl CostScale {
    /// Mirrors `new ExponentialCostScaling({ baseCost, baseIncrease, costScale,
    /// scalingCostThreshold })` — `threshold_log10` is `log10(scalingCostThreshold)`.
    pub fn new(
        base_cost: f64,
        base_increase: f64,
        cost_scale: f64,
        threshold_log10: f64,
    ) -> Self {
        let log_base_cost = base_cost.log10();
        let log_base_increase = base_increase.log10();
        let log_cost_scale = cost_scale.log10();
        let purchases_before_scaling =
            ((threshold_log10 - log_base_cost) / log_base_increase).ceil();
        let precalc_discriminant = (2.0 * log_base_increase + log_cost_scale).powi(2)
            - 8.0
                * log_cost_scale
                * (purchases_before_scaling * log_base_increase + log_base_cost);
        let precalc_center =
            -log_base_increase / log_cost_scale + purchases_before_scaling + 0.5;
        Self {
            log_base_cost,
            log_base_increase,
            log_cost_scale,
            purchases_before_scaling,
            precalc_discriminant,
            precalc_center,
        }
    }

    /// `calculateCost(currentPurchases)`: the cost of the next purchase.
    pub fn calculate_cost(&self, current_purchases: f64) -> Decimal {
        let log_mult = self.log_base_increase;
        let log_base = self.log_base_cost;
        let excess = current_purchases - self.purchases_before_scaling;
        let log_cost = if excess > 0.0 {
            current_purchases * log_mult
                + log_base
                + 0.5 * excess * (excess + 1.0) * self.log_cost_scale
        } else {
            current_purchases * log_mult + log_base
        };
        Decimal::pow10(log_cost)
    }

    /// `getContinuumValue(rawMoney, numberPerSet)`: the fractional purchase
    /// count Continuum grants, including the quadratic branch past the
    /// scaling threshold.
    pub fn get_continuum_value(&self, raw_money: Decimal, number_per_set: f64) -> f64 {
        let money = raw_money / Decimal::from_float(number_per_set);
        let log_money = money.log10();
        // `1 +` because the multiplier isn't applied to the first purchase.
        let mut cont_value =
            1.0 + (log_money - self.log_base_cost) / self.log_base_increase;
        // The linear method is valid up to one purchase past the threshold.
        if cont_value > self.purchases_before_scaling {
            let discrim =
                self.precalc_discriminant + 8.0 * self.log_cost_scale * log_money;
            if discrim < 0.0 {
                return 0.0;
            }
            cont_value =
                self.precalc_center + discrim.sqrt() / (2.0 * self.log_cost_scale);
        }
        cont_value.max(0.0)
    }

    /// `getMaxBought(currentPurchases, rawMoney, numberPerSet)`: the maximum new
    /// total affordable and the `log10` price of the top purchase, or `None` when
    /// nothing more can be bought. NOTE: like the original, this charges only for
    /// the most expensive item obtained (a deliberate bulk simplification).
    pub fn get_max_bought(
        &self,
        current_purchases: f64,
        raw_money: Decimal,
        number_per_set: f64,
    ) -> Option<(f64, f64)> {
        let money = raw_money / Decimal::from_float(number_per_set);
        let log_money = money.log10();
        let log_mult = self.log_base_increase;
        let log_base = self.log_base_cost;
        // `1 +` because the multiplier isn't applied to the first purchase.
        let mut new_purchases = (1.0 + (log_money - log_base) / log_mult).floor();
        // The linear method is valid up to one purchase past the threshold.
        if new_purchases > self.purchases_before_scaling {
            let discrim =
                self.precalc_discriminant + 8.0 * self.log_cost_scale * log_money;
            if discrim < 0.0 {
                return None;
            }
            new_purchases = (self.precalc_center
                + discrim.sqrt() / (2.0 * self.log_cost_scale))
                .floor();
        }
        if new_purchases <= current_purchases {
            return None;
        }
        let log_price = if new_purchases <= self.purchases_before_scaling + 1.0 {
            (new_purchases - 1.0) * log_mult + log_base
        } else {
            let p_excess = new_purchases - self.purchases_before_scaling;
            (new_purchases - 1.0) * log_mult
                + log_base
                + 0.5 * p_excess * (p_excess - 1.0) * self.log_cost_scale
        };
        Some((
            new_purchases - current_purchases,
            log_price + number_per_set.log10(),
        ))
    }
}
