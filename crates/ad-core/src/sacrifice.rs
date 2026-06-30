use break_infinity::Decimal;

use crate::data::constants::SACRIFICE_EXPONENT;
use crate::state::GameState;

impl GameState {
    /// Check if the player can sacrifice dimensions. This is the *enable* gate
    /// (`Sacrifice.canSacrifice`), distinct from the *visibility* gate
    /// [`sacrifice_unlocked`](Self::sacrifice_unlocked): the original requires
    /// `DimBoost.purchasedBoosts > 4` (≥ 5 boosts) regardless of whether the
    /// button is visible, plus AD8 amount > 0 and next boost > 1.
    pub fn can_sacrifice(&self) -> bool {
        self.dim_boosts >= 5
            && self.dimensions[7].amount > Decimal::ZERO
            && self.next_sacrifice_boost() > Decimal::ONE
    }

    /// Get the current sacrifice multiplier, computed
    /// statelessly from total sacrificed amount.
    ///
    /// JS `Sacrifice.totalBoost`:
    ///   if sacrificed == 0: return 1
    ///   prePowerBoost = max(1, log10(sacrificed) / 10)
    ///   totalBoost = prePowerBoost ^ SACRIFICE_EXPONENT
    pub fn sacrifice_multiplier(&self) -> Decimal {
        Self::total_boost(&self.sacrificed)
    }

    /// Compute the total sacrifice boost from a given
    /// sacrificed amount.
    fn total_boost(sacrificed: &Decimal) -> Decimal {
        if *sacrificed <= Decimal::ZERO {
            return Decimal::ONE;
        }
        let pre_power = (sacrificed.log10() / 10.0).max(1.0);
        Decimal::from_float(pre_power.powf(SACRIFICE_EXPONENT))
    }

    /// Compute the individual boost that the next sacrifice
    /// would give (the gain ratio). Used by the autobuyer to
    /// decide whether to sacrifice.
    ///
    /// JS `Sacrifice.nextBoost`:
    ///   sacrificed = player.sacrificed.clampMin(1)
    ///   prePowerMult = max(1, (log10(AD1) / 10)
    ///                       / max(log10(sacrificed) / 10, 1))
    ///   nextBoost = prePowerMult ^ SACRIFICE_EXPONENT
    pub fn next_sacrifice_boost(&self) -> Decimal {
        let ad1 = &self.dimensions[0].amount;
        if *ad1 <= Decimal::ONE {
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

        Decimal::from_float(ratio.powf(SACRIFICE_EXPONENT))
    }

    /// Compute what the total sacrifice multiplier would be
    /// after sacrificing.
    pub fn sacrifice_multiplier_if_sacrificed(&self) -> Decimal {
        let new_sacrificed = self.sacrificed + self.dimensions[0].amount;
        Self::total_boost(&new_sacrificed)
    }

    /// Perform a dimensional sacrifice.
    /// Adds AD1 amount to sacrifice total and resets
    /// dimensions 1-7 amounts (keeps bought counts).
    /// The 8th dimension multiplier is computed statelessly
    /// from the sacrifice total.
    /// Returns true if sacrifice was performed.
    pub fn sacrifice(&mut self) -> bool {
        if !self.can_sacrifice() {
            return false;
        }

        // Update total sacrificed
        self.sacrificed += self.dimensions[0].amount;

        // Reset amounts for dimensions 1-7 (indices 0-6)
        for i in 0..7 {
            self.dimensions[i].amount = Decimal::ZERO;
        }

        true
    }
}
