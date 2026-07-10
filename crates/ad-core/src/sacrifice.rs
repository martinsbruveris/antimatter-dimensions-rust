use break_infinity::Decimal;

use crate::data::constants::{BIG_CRUNCH_THRESHOLD, SACRIFICE_EXPONENT};
use crate::state::{DimensionTier, GameState};

impl GameState {
    /// Check if the player can sacrifice dimensions. This is the *enable* gate
    /// (`Sacrifice.canSacrifice`), distinct from the *visibility* gate
    /// [`sacrifice_unlocked`](Self::sacrifice_unlocked): the original requires
    /// `DimBoost.purchasedBoosts > 4` (≥ 5 boosts) regardless of whether the
    /// button is visible, plus AD8 amount > 0 and next boost > 1. Normal
    /// Challenge 10 disables Sacrifice outright (it has no 8th dimension).
    pub fn can_sacrifice(&self) -> bool {
        if self.challenge_running(10) {
            return false;
        }
        // EC3 disables Dimensional Sacrifice.
        if self.ec_running(3) {
            return false;
        }
        // Refused once antimatter reaches the run's ceiling
        // (`Currency.antimatter.lt(Player.infinityLimit)`). Inside an antimatter
        // challenge (e.g. Infinity Challenge 2, goal `1e10500`) production freezes
        // at the goal, so a sacrifice past it must be blocked too — otherwise the
        // frozen dimensions keep getting reset while the original leaves them.
        if self.antimatter >= self.infinity_limit() {
            return false;
        }
        self.dim_boosts >= 5
            && self.dimensions[7].amount > Decimal::ZERO
            && self.next_sacrifice_boost() > Decimal::ONE
    }

    /// The exponent applied to the pre-power sacrifice multiplier. `2` normally
    /// (`SACRIFICE_EXPONENT`); `1` under Normal Challenge 8, which looks weaker
    /// but ends up stronger because NC8 accumulates a running product across
    /// resets rather than deriving the boost from a single log. (The
    /// achievement/time-study factors that further raise the exponent are later
    /// features.) Mirrors `Sacrifice.sacrificeExponent`.
    fn sacrifice_exponent(&self) -> f64 {
        let base = if self.challenge_running(8) {
            1.0
        } else if self.infinity_challenge_completed(2) {
            // IC2 completed drops a log10 (a much stronger sacrifice); the exponent
            // shrinks to `1/120` to keep the pre-Reality balance.
            1.0 / 120.0
        } else {
            SACRIFICE_EXPONENT
        };
        // `base × preIC2 × postIC2` (`Sacrifice.sacrificeExponent`): achievements
        // 32 and 57 each add +0.1 to preIC2; achievement 88 and TS228 add +0.1 /
        // +0.2 to postIC2.
        let mut pre_ic2 = 1.0;
        if self.achievement_unlocked(32) {
            pre_ic2 += 0.1;
        }
        if self.achievement_unlocked(57) {
            pre_ic2 += 0.1;
        }
        let mut post_ic2 = 1.0;
        if self.achievement_unlocked(88) {
            post_ic2 += 0.1;
        }
        if self.time_study_bought(228) {
            post_ic2 += 0.2;
        }
        base * pre_ic2 * post_ic2
    }

    /// Whether the IC2-completed sacrifice formula is active (the "pre-power" value
    /// is the sacrificed amount / ratio directly rather than its `log10 / 10`).
    fn ic2_sacrifice(&self) -> bool {
        self.infinity_challenge_completed(2)
    }

    /// Get the current sacrifice multiplier applied to the 8th dimension.
    ///
    /// Under Normal Challenge 8 this is the running product
    /// `chall8TotalSacrifice`; otherwise it is computed statelessly from the
    /// total sacrificed amount (JS `Sacrifice.totalBoost`):
    ///   if sacrificed == 0: return 1
    ///   prePowerBoost = max(1, log10(sacrificed) / 10)
    ///   totalBoost = prePowerBoost ^ exponent
    pub fn sacrifice_multiplier(&self) -> Decimal {
        if self.challenge_running(8) {
            return self.chall8_total_sacrifice;
        }
        self.total_boost(&self.sacrificed)
    }

    /// Compute the standard (non-NC8) total sacrifice boost from a given
    /// sacrificed amount: `prePowerBoost.clampMin(1) ^ exponent`, where
    /// `prePowerBoost` is `log10(sacrificed)/10` normally, or the sacrificed amount
    /// itself once Infinity Challenge 2 is completed.
    fn total_boost(&self, sacrificed: &Decimal) -> Decimal {
        if *sacrificed <= Decimal::ZERO {
            return Decimal::ONE;
        }
        let exponent = Decimal::from_float(self.sacrifice_exponent());
        let pre_power = if self.ic2_sacrifice() {
            *sacrificed
        } else {
            Decimal::from_float(sacrificed.log10() / 10.0)
        };
        pre_power.max(&Decimal::ONE).pow(&exponent)
    }

    /// Compute the individual boost that the next sacrifice would give (the gain
    /// ratio). Used by the autobuyer to decide whether to sacrifice, and to
    /// advance `chall8TotalSacrifice` under NC8.
    ///
    /// Standard (JS `Sacrifice.nextBoost`):
    ///   sacrificed = player.sacrificed.clampMin(1)
    ///   prePowerMult = max(1, (log10(AD1) / 10) / max(log10(sacrificed) / 10, 1))
    ///   nextBoost = prePowerMult ^ exponent
    ///
    /// NC8: `prePowerMult = AD1^0.05/sacrificed^0.04 × AD1^0.05/(sacrificed+AD1)^0.04`.
    pub fn next_sacrifice_boost(&self) -> Decimal {
        let ad1 = self.dimensions[0].amount;

        if self.challenge_running(8) {
            if ad1 <= Decimal::ZERO {
                return Decimal::ONE;
            }
            let sacrificed = self.sacrificed.max(&Decimal::ONE);
            let p05 = Decimal::from_float(0.05);
            let p04 = Decimal::from_float(0.04);
            let term1 = ad1.pow(&p05) / sacrificed.pow(&p04);
            let term2 = ad1.pow(&p05) / (sacrificed + ad1).pow(&p04);
            let pre = (term1 * term2).max(&Decimal::ONE);
            return pre.pow(&Decimal::from_float(self.sacrifice_exponent()));
        }

        if ad1 <= Decimal::ONE {
            return Decimal::ONE;
        }

        let sacrificed = if self.sacrificed <= Decimal::ZERO {
            Decimal::ONE
        } else {
            self.sacrificed
        };

        let exponent = Decimal::from_float(self.sacrifice_exponent());
        let pre_power = if self.ic2_sacrifice() {
            // IC2 completed: `prePowerMult = AD1 / sacrificed` (no log10).
            ad1 / sacrificed
        } else {
            let log_ad1 = ad1.log10() / 10.0;
            let log_sacrificed = (sacrificed.log10() / 10.0).max(1.0);
            Decimal::from_float((log_ad1 / log_sacrificed).max(1.0))
        };
        pre_power.max(&Decimal::ONE).pow(&exponent)
    }

    /// Compute what the total sacrifice multiplier would be after sacrificing.
    pub fn sacrifice_multiplier_if_sacrificed(&self) -> Decimal {
        if self.challenge_running(8) {
            return self.chall8_total_sacrifice * self.next_sacrifice_boost();
        }
        let new_sacrificed = self.sacrificed + self.dimensions[0].amount;
        self.total_boost(&new_sacrificed)
    }

    /// Whether the Dimensional Sacrifice autobuyer runs
    /// (`SacrificeAutobuyerState.isUnlocked`): the `autoIC` Eternity milestone (7
    /// eternities) or a completed Infinity Challenge 2.
    pub fn sacrifice_autobuyer_unlocked(&self) -> bool {
        self.eternity_milestone_reached(7) || self.infinity_challenge_completed(2)
    }

    /// Perform a dimensional sacrifice.
    ///
    /// Standard: adds AD1 amount to the sacrifice total and resets dimensions
    /// 1–7 amounts (keeps bought counts); the 8th-dimension multiplier is then
    /// derived statelessly from the sacrifice total.
    ///
    /// Under Normal Challenge 8: advances the running product
    /// `chall8TotalSacrifice *= nextBoost`, adds AD1 to the sacrifice total, and
    /// resets **every** dimension (amount, bought, cost) plus antimatter — a much
    /// harsher reset for a much stronger boost. Once the product reaches the cap
    /// (`Number.MAX_VALUE`) further sacrifices are no-ops.
    ///
    /// Returns true if a sacrifice was performed.
    pub fn sacrifice(&mut self) -> bool {
        if !self.can_sacrifice() {
            return false;
        }
        // Below the Break, or in a Normal (not Infinity) Challenge, refuse once
        // antimatter passes the Infinity cap — you should crunch, not sacrifice
        // (`sacrificeReset`'s antimatter guard).
        let nc_not_ic =
            self.challenge.current != 0 && self.infinity_challenge.current == 0;
        if (!self.broke_infinity || nc_not_ic)
            && self.antimatter > Decimal::NUMBER_MAX_VALUE
        {
            return false;
        }
        // Under NC8 the running product doubles as the sacrifice multiplier and
        // is capped at `Number.MAX_VALUE`.
        if self.challenge_running(8)
            && self.chall8_total_sacrifice >= BIG_CRUNCH_THRESHOLD
        {
            return false;
        }

        // SACRIFICE_RESET_BEFORE achievements (88), read from the pre-sacrifice
        // `nextBoost`.
        self.check_sacrifice_before_achievements();

        // `chall8TotalSacrifice *= nextBoost` and `sacrificed += AD1` run for every
        // sacrifice, even outside NC8 (`sacrificeReset`); NC8 merely also *uses* the
        // product as its multiplier.
        let next_boost = self.next_sacrifice_boost();
        self.chall8_total_sacrifice *= next_boost;
        self.sacrificed += self.dimensions[0].amount;

        // Achievement 118 stops a Sacrifice from resetting the Antimatter
        // Dimensions (`isAch118Unlocked`).
        let keep_dimensions = self.achievement_unlocked(118);
        if self.challenge_running(8) {
            // NC8 fully resets every dimension (and antimatter) for a much
            // stronger boost.
            if !keep_dimensions {
                for i in 0..8 {
                    self.dimensions[i] = DimensionTier::new();
                }
            }
            self.antimatter = self.starting_antimatter();
        } else if !keep_dimensions {
            // `resetAmountUpToTier`: reset amounts up to tier 7 (indices 0–6), or
            // tier 6 (keeping AD7) under Normal Challenge 12.
            let max_tier = if self.challenge_running(12) { 6 } else { 7 };
            for i in 0..max_tier {
                self.dimensions[i].amount = Decimal::ZERO;
            }
        }

        // A Sacrifice breaks the "no Sacrifice since last Galaxy" flag
        // (`sacrifice.js`).
        self.requirement_checks.infinity_no_sacrifice = false;
        // SACRIFICE_RESET_AFTER achievements (32, 118, …).
        self.check_sacrifice_after_achievements();
        true
    }
}
