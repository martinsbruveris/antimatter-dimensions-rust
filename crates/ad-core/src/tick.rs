use break_infinity::Decimal;

use crate::GameState;

impl GameState {
    /// Advance the game by `dt` milliseconds.
    /// AD1 produces antimatter: production = amount * dt/1000
    pub fn tick(&mut self, dt_ms: f64) {
        let dt_seconds = dt_ms / 1000.0;
        let production = self.ad1.amount * Decimal::from_float(dt_seconds);
        self.antimatter = self.antimatter + production;
    }
}
