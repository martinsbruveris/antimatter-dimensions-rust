use break_infinity::Decimal;

use crate::data::constants::SACRIFICE_EXPONENT;
use crate::state::GameState;

impl GameState {
    /// Check if the player can sacrifice dimensions.
    /// Requires: sacrifice unlocked AND has a non-zero amount
    /// of 1st dimension.
    pub fn can_sacrifice(&self) -> bool {
        self.sacrifice_unlocked && self.dimensions[0].amount > Decimal::from_float(0.0)
    }

    /// Get the current sacrifice multiplier (running product
    /// of all past sacrifice boosts).
    pub fn sacrifice_multiplier(&self) -> Decimal {
        self.sacrifice_boost
    }

    /// Compute the individual boost that the next sacrifice
    /// would give. Uses the pre-infinity formula:
    ///   max(1, (log10(AD1) / 10) /
    ///          max(log10(total_sacrificed) / 10, 1))
    ///   ^ SACRIFICE_EXPONENT
    fn next_sacrifice_boost(
        ad1_amount: &Decimal,
        total_sacrificed: &Decimal,
    ) -> Decimal {
        if *ad1_amount <= Decimal::ONE {
            return Decimal::ONE;
        }

        let log_ad1 = ad1_amount.log10() / 10.0;
        let log_sacrificed = (total_sacrificed.log10() / 10.0).max(1.0);
        let ratio = (log_ad1 / log_sacrificed).max(1.0);

        Decimal::from_float(ratio.powf(SACRIFICE_EXPONENT))
    }

    /// Compute what the total sacrifice multiplier would be
    /// after sacrificing. Returns the new total multiplier.
    pub fn sacrifice_multiplier_if_sacrificed(&self) -> Decimal {
        let next_boost =
            Self::next_sacrifice_boost(&self.dimensions[0].amount, &self.sacrificed);
        self.sacrifice_boost * next_boost
    }

    /// Perform a dimensional sacrifice.
    /// Resets dimensions 1-7 amounts (keeps bought count) and
    /// adds the 1st dimension amount to the sacrifice total.
    /// The 8th dimension gets a production multiplier from the
    /// running product of sacrifice boosts.
    /// Returns true if sacrifice was performed.
    pub fn sacrifice(&mut self) -> bool {
        if !self.can_sacrifice() {
            return false;
        }

        let ad1_amount = self.dimensions[0].amount;

        // Compute the individual boost using existing total
        let next_boost = Self::next_sacrifice_boost(&ad1_amount, &self.sacrificed);
        self.sacrifice_boost *= next_boost;

        // Update total sacrificed
        self.sacrificed += ad1_amount;

        // Reset amounts for dimensions 1-7 (indices 0-6)
        for i in 0..7 {
            self.dimensions[i].amount = Decimal::default();
        }

        true
    }
}
