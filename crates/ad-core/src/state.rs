use break_infinity::Decimal;

/// A single antimatter dimension tier.
#[derive(Debug, Clone)]
pub struct DimensionTier {
    pub amount: Decimal,
    pub bought: u64,
    pub cost: Decimal,
}

impl DimensionTier {
    pub fn new(cost: Decimal) -> Self {
        Self {
            amount: Decimal::default(),
            bought: 0,
            cost,
        }
    }
}

/// Minimal game state for the framework skeleton.
/// Only AD1 is functional for now.
#[derive(Debug, Clone)]
pub struct GameState {
    pub antimatter: Decimal,
    pub ad1: DimensionTier,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            antimatter: Decimal::from_float(10.0), // Start with 10 antimatter
            ad1: DimensionTier::new(Decimal::from_float(10.0)), // AD1 costs 10
        }
    }

    /// Try to buy one AD1. Returns true if successful.
    pub fn buy_ad1(&mut self) -> bool {
        if self.antimatter >= self.ad1.cost {
            self.antimatter = self.antimatter - self.ad1.cost;
            self.ad1.amount = self.ad1.amount + Decimal::from_float(1.0);
            self.ad1.bought += 1;
            // Cost increases by 10x per purchase (simplified)
            self.ad1.cost = self.ad1.cost * Decimal::from_float(10.0);
            true
        } else {
            false
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
