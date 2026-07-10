use break_infinity::Decimal;

use crate::data::constants::{
    GALAXY_TICKSPEED_REDUCTION, INITIAL_TICKSPEED_MS, TICKSPEED_BASE_MULTIPLIERS,
    TICKSPEED_BASE_MULTIPLIERS_C5, TICKSPEED_GALAXY_BASE, TICKSPEED_GALAXY_BASE_C5,
    TICKSPEED_GALAXY_DECAY, TICKSPEED_MULTIPLIER_MIN,
};
use crate::state::GameState;

impl GameState {
    /// `Player.tickSpeedMultDecrease`: the Tickspeed cost-scale (`costScale`),
    /// reduced by the `tickspeedCostMult` Break Infinity Upgrade (−1 per purchase)
    /// and EC11 completions (−0.07 each).
    pub fn tickspeed_mult_decrease(&self) -> f64 {
        10.0 - self.infinity_rebuyables[0] as f64
            - self.eternity_challenge_completions(11) as f64 * 0.07
    }

    /// The Tickspeed purchase-cost curve (`Tickspeed.costScale`).
    fn tickspeed_cost_scale(&self) -> crate::cost_scaling::CostScale {
        crate::cost_scaling::CostScale::new(
            1000.0,
            10.0,
            self.tickspeed_mult_decrease(),
            crate::cost_scaling::LOG10_NUMBER_MAX_VALUE,
        )
    }

    /// `Tickspeed.cost`: `calculateCost(totalTickBought + chall9TickspeedCostBumps)`.
    /// Super-exponential past `Number.MAX_VALUE` (see [`crate::cost_scaling`]).
    pub fn tickspeed_purchase_cost(&self) -> Decimal {
        self.tickspeed_cost_scale()
            .calculate_cost((self.tickspeed.bought + self.tickspeed.cost_bumps) as f64)
    }

    /// Buy one tickspeed upgrade (`buyTickSpeed`). Returns true if successful.
    pub fn buy_tickspeed(&mut self) -> bool {
        if !self.tickspeed_available() || !self.tickspeed_affordable() {
            return false;
        }
        // NC9: buying a Tickspeed upgrade bumps every equal-cost dimension to
        // its next cost step (before the purchase, using the current cost).
        if self.challenge_running(9) {
            self.nc9_bump_same_cost_from_tickspeed();
        }
        // Clear the TICKSPEED tutorial highlight on the purchase, like the
        // original's buyTickSpeed (no-op once past that step).
        self.tutorial_turn_off(crate::tutorial::state::TICKSPEED);
        self.antimatter -= self.tickspeed_purchase_cost();
        self.tickspeed.bought += 1;
        // IC8: buying a Tickspeed upgrade resets the production-decay timer.
        self.records.this_infinity.last_buy_time_ms = self.records.this_infinity.time_ms;
        // Normal Challenge 2: buying a Tickspeed upgrade also halts production.
        if self.challenge_running(2) {
            self.chall2_pow = 0.0;
        }
        true
    }

    /// Faithful port of `buyMaxTickSpeed`: under NC9 it loops purchase-by-purchase
    /// (each buy bumps every equal-cost dimension, so costs are abnormal and the
    /// buy stops at the Big Crunch goal); otherwise it uses the analytic
    /// `getMaxBought`, charging only for the most expensive purchase — O(1) even
    /// for enormous balances. Returns the number bought.
    pub fn buy_max_tickspeed(&mut self) -> u64 {
        if !self.tickspeed_available() || !self.tickspeed_affordable() {
            return 0;
        }
        let mut bought = 0u64;
        self.tutorial_turn_off(crate::tutorial::state::TICKSPEED);
        if self.challenge_running(9) {
            let goal = self.infinity_goal();
            let mut cost = self.tickspeed_purchase_cost();
            // The original's loop condition is strict (`antimatter.gt(cost)`).
            while self.antimatter > cost && cost < goal {
                self.nc9_bump_same_cost_from_tickspeed();
                self.antimatter -= cost;
                self.tickspeed.bought += 1;
                bought += 1;
                cost = self.tickspeed_purchase_cost();
            }
        } else {
            // Like the original, this passes `totalTickBought` alone — the NC9
            // cost bumps are not folded in (they are zero outside NC9).
            let purchases = self.tickspeed_cost_scale().get_max_bought(
                self.tickspeed.bought as f64,
                self.antimatter,
                1.0,
            );
            let Some((quantity, log_price)) = purchases else {
                return 0;
            };
            self.antimatter -= Decimal::pow10(log_price);
            self.tickspeed.bought += quantity as u64;
            bought = quantity as u64;
        }
        if bought > 0 {
            self.records.this_infinity.last_buy_time_ms =
                self.records.this_infinity.time_ms;
            if self.challenge_running(2) {
                self.chall2_pow = 0.0;
            }
        }
        bought
    }

    /// Total Tickspeed upgrades: bought plus the free upgrades from Time
    /// Shards (`Tickspeed.totalUpgrades = totalTickBought + totalTickGained`).
    pub fn total_tickspeed_upgrades(&self) -> u64 {
        self.tickspeed.bought + self.total_tick_gained
    }

    /// Whether a Tickspeed upgrade can currently be purchased
    /// (`Tickspeed.isAvailableForPurchase`): unlocked, not under EC9 or Continuum,
    /// and (pre-break) its cost still within `NUMBER_MAX_VALUE`.
    pub fn tickspeed_available(&self) -> bool {
        self.tickspeed_unlocked()
            && !self.ec_running(9)
            && !self.continuum_active()
            && (self.broke_infinity
                || self.tickspeed_purchase_cost() < Decimal::NUMBER_MAX_VALUE)
    }

    /// Whether antimatter covers the next Tickspeed upgrade
    /// (`Tickspeed.isAffordable`).
    pub fn tickspeed_affordable(&self) -> bool {
        self.antimatter >= self.tickspeed_purchase_cost()
    }

    /// Compute the current tickspeed in milliseconds:
    /// `INITIAL_TICKSPEED_MS × multiplier^totalUpgrades`. A `Decimal` because
    /// free Tickspeed upgrades push the count far past what `f64` can hold
    /// (`0.8^300000` underflows) — the original's `Tickspeed.current` is a
    /// Decimal too.
    pub fn current_tickspeed_ms(&self) -> Decimal {
        let multiplier = self.tickspeed_purchase_multiplier();
        // Lai'tela's Continuum replaces the discrete bought count with a
        // continuous value (`Tickspeed.continuumValue`).
        let upgrades = if self.continuum_active() {
            self.tickspeed_continuum_value() + self.total_tick_gained as f64
        } else {
            self.total_tickspeed_upgrades() as f64
        };
        let mut base = Decimal::from_float(INITIAL_TICKSPEED_MS)
            * Decimal::from_float(self.starting_tickspeed_mult())
            * Decimal::from_float(multiplier).pow(&Decimal::from_float(upgrades));
        // The Pelle-only `tickspeedPower` Dilation rebuyable (id 13):
        // `baseValue^(1 + 0.03·bought)` (`Tickspeed.current`, Doomed only).
        let ts_power = 1.0 + 0.03 * self.dilation_rebuyable_count(13) as f64;
        if ts_power != 1.0 && self.is_doomed() {
            base = base.pow(&Decimal::from_float(ts_power));
        }
        // Effarig's Reality replaces the tickspeed value with a compressed one
        // (`Tickspeed.current`: `Effarig.isRunning ? Effarig.tickspeed : base`).
        let tickspeed = if self.celestials.effarig.run {
            self.effarig_tickspeed(base)
        } else {
            base
        };
        // Time Dilation compresses the interval too (`Tickspeed.current`).
        if self.dilation.active {
            self.dilated_value_of(tickspeed)
        } else {
            tickspeed
        }
    }

    /// The per-purchase tickspeed multiplier (fraction of
    /// current tickspeed retained per purchase). Uses two
    /// formulas depending on galaxy count:
    ///
    /// Pre-3 galaxies (linear):
    ///   base_mult[galaxies] - galaxies * 0.02
    ///
    /// 3+ galaxies (exponential):
    ///   0.8 * 0.965^(galaxies - 4)
    pub fn tickspeed_purchase_multiplier(&self) -> f64 {
        // IC3 neutralises the *per-purchase* multiplier (`getTickSpeedMultiplier`
        // returns `DC.D1`), so each Tickspeed upgrade contributes ×1. The base
        // tickspeed and its Achievement effects (36/45/66/83) still apply via
        // `starting_tickspeed_mult`, so the overall tickspeed production factor is
        // *not* 1 — only the growth from upgrades is removed.
        if self.ic3_neutralizes_tickspeed() {
            return 1.0;
        }
        // `effectiveBaseGalaxies`: antimatter galaxies plus Replicanti Galaxies feed
        // the tickspeed formula (the branch cutoff, per-galaxy reduction, and the
        // exponent). The base-multiplier lookup, however, keys off the *antimatter*
        // galaxy count (JS: `player.galaxies === 1/2`), since only 0–2 antimatter
        // galaxies are possible while the effective count is < 3.
        let eff = self.effective_galaxies();
        let galaxies = eff as f64;
        // The original's `effects` product scales the per-galaxy term: the
        // `galaxyBoost` Infinity/Break-Infinity Upgrades plus the galaxy-strength
        // Time Studies — TS212 (from Time Shards, cap ×1.1) and TS232 (from
        // Antimatter Galaxies).
        let mut effects = self.galaxy_strength_effect();
        if self.time_study_bought(212) {
            let log2_shards = self.time_shards.max(&Decimal::from_float(2.0)).ln()
                / std::f64::consts::LN_2;
            effects *= log2_shards.powf(0.005).min(1.1);
        }
        if self.time_study_bought(232) {
            effects *= (1.0 + self.galaxies as f64 / 1000.0).powf(0.2);
        }
        // Pelle terms in the `effects` product: the `galaxyPower` rebuyable
        // (`1 + x/50`) and the decay rift's second milestone (Galaxies 10%
        // stronger while Replicanti exceeds 1e1300).
        effects *= self.pelle_galaxy_power_mult();
        if self.pelle_rift_milestone(crate::celestials::pelle::RIFT_DECAY, 1)
            && self.replicanti.amount > Decimal::new(1.0, 1300)
        {
            effects *= 1.1;
        }

        // Normal Challenge 5 lowers the base multiplier (the tickspeed purchase
        // multiplier starts at ×1.080 instead of ×1.1245).
        let in_c5 = self.challenge_running(5);

        if eff < 3 {
            let base = if in_c5 {
                TICKSPEED_BASE_MULTIPLIERS_C5[self.galaxies as usize]
            } else {
                TICKSPEED_BASE_MULTIPLIERS[self.galaxies as usize]
            };
            // perGalaxy = 0.02 * effects; reduction = galaxies * perGalaxy.
            // While Doomed the galaxy count is halved, then scaled by the
            // power-glyph special Pelle effect.
            let mut galaxies = galaxies;
            if self.is_doomed() {
                galaxies *= 0.5;
            }
            galaxies *= self.pelle_special_glyph_power();
            let reduction = galaxies * GALAXY_TICKSPEED_REDUCTION * effects;
            (base - reduction).max(TICKSPEED_MULTIPLIER_MIN)
        } else {
            // JS: galaxies -= 2; galaxies *= effects; decay^(galaxies - 2) * base.
            // Unlike the `< 3` branch, the original does *not* clamp this to the
            // 0.01 floor, so deep galaxy counts drive the per-purchase multiplier
            // well below 0.01 (a much faster tickspeed).
            let galaxy_base = if in_c5 {
                TICKSPEED_GALAXY_BASE_C5
            } else {
                TICKSPEED_GALAXY_BASE
            };
            // `realitygalaxies` scales the effective galaxy count in this
            // branch (`galaxies *= getAdjustedGlyphEffect("realitygalaxies")`).
            // (`cursedgalaxies` would divide here too — cursed glyphs are a
            // later feature.)
            let mut adjusted =
                (galaxies - 2.0) * effects * self.glyph_effect_realitygalaxies();
            // Imaginary Upgrade 9 (Cosmic Filament): `1 + 0.03 × count`.
            adjusted *= 1.0 + self.imaginary_rebuyable_effect(9);
            if self.is_doomed() {
                adjusted *= 0.5;
            }
            adjusted *= self.pelle_special_glyph_power();
            galaxy_base * TICKSPEED_GALAXY_DECAY.powf(adjusted - 2.0)
        }
    }

    /// The "starting tickspeed" multiplier folded into `baseValue`
    /// (`Tickspeed.baseValue.timesEffectsOf(Achievement 36/45/66/83)`). Each
    /// factor is < 1, shrinking the base tickspeed interval (faster ticks).
    /// 66 (0.98) and 83 (0.95^galaxies) are wired with their achievement batches.
    fn starting_tickspeed_mult(&self) -> f64 {
        let mut mult = 1.0;
        // 36: multiply starting tickspeed by 1/1.02.
        if self.achievement_unlocked(36) {
            mult /= 1.02;
        }
        // 45: multiply starting tickspeed by 0.98.
        if self.achievement_unlocked(45) {
            mult *= 0.98;
        }
        // 66: multiply starting tickspeed by 0.98.
        if self.achievement_unlocked(66) {
            mult *= 0.98;
        }
        // 83: tickspeed ×0.95 per Antimatter Galaxy (`0.95^galaxies`).
        if self.achievement_unlocked(83) {
            mult *= 0.95_f64.powi(self.galaxies as i32);
        }
        mult
    }

    /// Compute the effective production multiplier from
    /// tickspeed. Production is inversely proportional to
    /// tickspeed interval:
    ///   effect = INITIAL_TICKSPEED_MS / current_tickspeed_ms
    pub fn tickspeed_effect(&self) -> Decimal {
        // IC3 does *not* zero this: it only forces the per-purchase multiplier to
        // ×1 (handled in `tickspeed_purchase_multiplier`), so `current` is still
        // `1000 · starting_tickspeed_mult · 1^upgrades` and the production factor
        // is `1000 / current = 1 / starting_tickspeed_mult` (≠ 1 whenever a
        // starting-tickspeed Achievement is owned).
        let current = self.current_tickspeed_ms();
        if current <= Decimal::ZERO {
            return Decimal::from_float(1.0);
        }
        Decimal::from_float(INITIAL_TICKSPEED_MS) / current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The analytic `buy_max_tickspeed` mirrors `getMaxBought`: it buys every
    /// upgrade whose *individual* cost fits the balance and charges only for the
    /// most expensive one (the original's deliberate bulk simplification). With
    /// 1e40 antimatter (base 1000, ×10 per buy) that is 38 upgrades — one more
    /// than a cumulative-cost loop affords — for exactly 1e40.
    #[test]
    fn buy_max_tickspeed_charges_only_top_purchase() {
        let mut game = GameState::new();
        game.dimensions[1].bought = 1; // unlock tickspeed
        game.antimatter = Decimal::from_float(1e40);
        assert_eq!(game.buy_max_tickspeed(), 38);
        assert_eq!(game.tickspeed.bought, 38);
        assert_eq!(game.antimatter, Decimal::ZERO);
    }

    /// The bulk buy handles the super-exponential regime past `Number.MAX_VALUE`
    /// in O(1): the quadratic branch of `getMaxBought` puts 1e5000 antimatter at
    /// 402 total upgrades (306 geometric + the scaling tail).
    #[test]
    fn buy_max_tickspeed_handles_super_exponential_regime() {
        let mut game = GameState::new();
        game.dimensions[1].bought = 1;
        game.broke_infinity = true;
        game.antimatter = Decimal::new(1.0, 5000);
        let bought = game.buy_max_tickspeed();
        assert_eq!(bought, 402);
        assert!(game.antimatter >= Decimal::ZERO);
    }

    /// Past 2 Antimatter Galaxies the per-purchase Tickspeed multiplier
    /// (`0.8·0.965^…`) is *not* clamped to the 0.01 floor (that floor only applies
    /// below 3 galaxies), so deep galaxy counts drive it well below 0.01.
    #[test]
    fn tickspeed_multiplier_unclamped_past_two_galaxies() {
        let mut game = GameState::new();
        game.galaxies = 300;
        let m = game.tickspeed_purchase_multiplier();
        assert!(m > 0.0 && m < 0.01, "expected 0 < m < 0.01, got {m}");
    }
}
