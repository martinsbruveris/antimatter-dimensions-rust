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
        if self.challenge_running(8) {
            1.0
        } else {
            SACRIFICE_EXPONENT
        }
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
        Self::total_boost(&self.sacrificed, self.sacrifice_exponent())
    }

    /// Compute the standard (non-NC8) total sacrifice boost from a given
    /// sacrificed amount and exponent.
    fn total_boost(sacrificed: &Decimal, exponent: f64) -> Decimal {
        if *sacrificed <= Decimal::ZERO {
            return Decimal::ONE;
        }
        let pre_power = (sacrificed.log10() / 10.0).max(1.0);
        Decimal::from_float(pre_power.powf(exponent))
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

        let log_ad1 = ad1.log10() / 10.0;
        let log_sacrificed = (sacrificed.log10() / 10.0).max(1.0);
        let ratio = (log_ad1 / log_sacrificed).max(1.0);

        Decimal::from_float(ratio.powf(self.sacrifice_exponent()))
    }

    /// Compute what the total sacrifice multiplier would be after sacrificing.
    pub fn sacrifice_multiplier_if_sacrificed(&self) -> Decimal {
        if self.challenge_running(8) {
            return self.chall8_total_sacrifice * self.next_sacrifice_boost();
        }
        let new_sacrificed = self.sacrificed + self.dimensions[0].amount;
        Self::total_boost(&new_sacrificed, self.sacrifice_exponent())
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

        if self.challenge_running(8) {
            if self.chall8_total_sacrifice >= BIG_CRUNCH_THRESHOLD {
                return false;
            }
            let next_boost = self.next_sacrifice_boost();
            self.chall8_total_sacrifice *= next_boost;
            self.sacrificed += self.dimensions[0].amount;
            for i in 0..8 {
                self.dimensions[i] = DimensionTier::new();
            }
            self.antimatter = self.starting_antimatter();
            return true;
        }

        // Update total sacrificed
        self.sacrificed += self.dimensions[0].amount;

        // Reset amounts for the lower dimensions (`resetAmountUpToTier`): up to
        // tier 7 (indices 0–6) normally, or tier 6 (indices 0–5, keeping AD7)
        // under Normal Challenge 12.
        let max_tier = if self.challenge_running(12) { 6 } else { 7 };
        for i in 0..max_tier {
            self.dimensions[i].amount = Decimal::ZERO;
        }

        true
    }
}
