use break_infinity::Decimal;

use crate::state::GameState;

impl GameState {
    /// Check if the player can sacrifice dimensions.
    /// Requires: sacrifice unlocked AND has a non-zero amount of 1st dimension.
    pub fn can_sacrifice(&self) -> bool {
        self.sacrifice_unlocked && self.dimensions[0].amount > Decimal::from_float(0.0)
    }

    /// Compute what the sacrifice multiplier would be after sacrificing.
    /// Based on the current 1st dimension amount being added to the total sacrificed.
    pub fn sacrifice_multiplier_if_sacrificed(&self) -> Decimal {
        use crate::data::constants::{SACRIFICE_EXPONENT, SACRIFICE_MIN_AMOUNT};

        let new_sacrificed = self.sacrificed + self.dimensions[0].amount;
        if new_sacrificed <= Decimal::from_float(SACRIFICE_MIN_AMOUNT) {
            return Decimal::from_float(1.0);
        }

        let ratio = new_sacrificed / Decimal::from_float(SACRIFICE_MIN_AMOUNT);
        let exponent = Decimal::from_float(SACRIFICE_EXPONENT);
        ratio.pow(&exponent)
    }

    /// Perform a dimensional sacrifice.
    /// Resets dimensions 1-7 amounts (keeps bought count and costs) and adds
    /// the 1st dimension amount to the sacrifice total.
    /// The 8th dimension gets a production multiplier based on total sacrificed.
    /// Returns true if sacrifice was performed.
    pub fn sacrifice(&mut self) -> bool {
        if !self.can_sacrifice() {
            return false;
        }

        // Add 1st dimension amount to sacrifice total
        self.sacrificed += self.dimensions[0].amount;

        // Reset amounts for dimensions 1-7 (indices 0-6)
        for i in 0..7 {
            self.dimensions[i].amount = Decimal::default();
        }

        true
    }
}
