//! Replicanti (Feature 3.2): self-replicating entities unlocked with Infinity
//! Points. They grow continuously, multiply all Infinity Dimensions
//! (`replicanti_mult`), and can be spent on **Replicanti Galaxies** — resets that
//! add a galaxy to the tickspeed formula. Replicanti persist across a Big Crunch
//! (reset only on Eternity, a later feature).
//!
//! Pre-Eternity the mechanic simplifies sharply: `isUncapped` (TS192/Pelle) is
//! always false, so Replicanti stay capped at `Number.MAX_VALUE`, the over-cap
//! interval scaling never runs, and the speed multiplier / `extra` terms are ×1/×0.
//! See `design-docs/2026-07-03-replicanti.md`.

use break_infinity::Decimal;

use crate::state::GameState;

/// IP cost to unlock Replicanti (`Replicanti.unlock`, pre-Pelle).
pub const REPLICANTI_UNLOCK_COST: Decimal = Decimal::new_unchecked(1.0, 140);

/// Hard cap on the replicanti amount pre-Eternity: `Decimal.NUMBER_MAX_VALUE` =
/// `Number.MAX_VALUE` ≈ 1.8e308. (Distinct from [`Decimal::MAX_VALUE`], which is the
/// Decimal-infinity sentinel far beyond this.)
pub const REPLICANTI_CAP: Decimal = Decimal::NUMBER_MAX_VALUE;

/// Base cost and per-buy multiplier of the chance upgrade (`chanceCost`).
const CHANCE_BASE_COST: Decimal = Decimal::new_unchecked(1.0, 150);
const CHANCE_COST_MULT: f64 = 1e15;
/// Base cost and per-buy multiplier of the interval upgrade (`intervalCost`).
const INTERVAL_BASE_COST: Decimal = Decimal::new_unchecked(1.0, 140);
const INTERVAL_COST_MULT: f64 = 1e10;

/// Chance cap (100%) and interval floor (50 ms) pre-time-studies.
const CHANCE_CAP: f64 = 1.0;
const INTERVAL_CAP_MS: f64 = 50.0;

/// Starting chance / interval (`Replicanti.reset`).
const INITIAL_CHANCE: f64 = 0.01;
const INITIAL_INTERVAL_MS: f64 = 1000.0;

/// Replicanti state (`player.replicanti`). Persists across a Big Crunch.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReplicantiState {
    /// Whether Replicanti are unlocked (`unl`).
    pub unlocked: bool,
    /// Current amount, capped at [`REPLICANTI_CAP`].
    pub amount: Decimal,
    /// Sub-interval time accumulator in ms (`timer`).
    pub timer_ms: f64,
    /// Reproduction chance per interval (`chance`, 0.01…1.0). Upgrade 1.
    pub chance: f64,
    /// Next chance-upgrade IP cost (`chanceCost`, ×1e15/buy).
    pub chance_cost: Decimal,
    /// Reproduction interval in ms (`interval`, 1000…50). Upgrade 2.
    pub interval_ms: f64,
    /// Next interval-upgrade IP cost (`intervalCost`, ×1e10/buy).
    pub interval_cost: Decimal,
    /// Replicanti Galaxies made (`galaxies`).
    pub galaxies: u32,
    /// Max Replicanti Galaxies (`boughtGalaxyCap`). Upgrade 3.
    pub galaxy_cap: u32,
}

impl ReplicantiState {
    /// A fresh, locked Replicanti state (`Replicanti.reset(force = true)`).
    pub fn new() -> Self {
        Self {
            unlocked: false,
            amount: Decimal::ZERO,
            timer_ms: 0.0,
            chance: INITIAL_CHANCE,
            chance_cost: CHANCE_BASE_COST,
            interval_ms: INITIAL_INTERVAL_MS,
            interval_cost: INTERVAL_BASE_COST,
            galaxies: 0,
            galaxy_cap: 0,
        }
    }
}

impl Default for ReplicantiState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// Whether Replicanti can be unlocked now (not yet unlocked, enough IP).
    pub fn can_unlock_replicanti(&self) -> bool {
        !self.replicanti.unlocked && self.infinity_points >= REPLICANTI_UNLOCK_COST
    }

    /// Unlock Replicanti, spending the IP cost. Returns whether unlocked afterwards.
    pub fn unlock_replicanti(&mut self) -> bool {
        if self.replicanti.unlocked {
            return true;
        }
        if self.infinity_points < REPLICANTI_UNLOCK_COST {
            return false;
        }
        self.infinity_points -= REPLICANTI_UNLOCK_COST;
        self.replicanti.unlocked = true;
        self.replicanti.timer_ms = 0.0;
        self.replicanti.amount = Decimal::ONE;
        true
    }

    /// Grow Replicanti over `dt_ms` (`replicantiLoop`, simplified for pre-Eternity:
    /// always capped, speed mult 1, no over-cap scaling). Uses the continuous "fast
    /// gain" approximation `amount ·= (1 + chance)^ticks`; the binomial/Poisson
    /// randomness at tiny amounts is dropped as a faithful aggregate.
    pub fn tick_replicanti(&mut self, dt_ms: f64) {
        if !self.replicanti.unlocked {
            return;
        }
        let interval = self.replicanti.interval_ms;
        if interval <= 0.0 {
            return;
        }
        let ticks = (dt_ms + self.replicanti.timer_ms) / interval;
        let whole = ticks.floor();
        // Roll leftover sub-interval time back into the timer (JS drops it above 100
        // ticks/loop to avoid round-off).
        self.replicanti.timer_ms = if ticks < 100.0 {
            (ticks - whole) * interval
        } else {
            0.0
        };
        if whole <= 0.0 {
            return;
        }
        // amount ·= (1 + chance)^whole, then clamp to the cap. Decimal::pow handles
        // the f64 overflow that a fast interval would otherwise cause.
        let growth = Decimal::from_float(1.0 + self.replicanti.chance)
            .pow(&Decimal::from_float(whole));
        self.replicanti.amount = (self.replicanti.amount * growth).min(&REPLICANTI_CAP);
    }

    /// Total galaxies feeding the tickspeed formula: antimatter galaxies plus
    /// Replicanti Galaxies (`effectiveBaseGalaxies`, pre-time-studies).
    pub fn effective_galaxies(&self) -> u32 {
        self.galaxies + self.replicanti.galaxies
    }

    /// Replicanti's multiplier to all Infinity Dimensions: `log2(max(amount, 1))^2`,
    /// clamped to ≥ 1 (`replicantiMult`, dropping the later TS/glyph terms). Folded
    /// into `id_common_multiplier` while `unlocked && amount > 1`.
    pub fn replicanti_mult(&self) -> Decimal {
        let log2 = self.replicanti.amount.max(&Decimal::ONE).log10()
            / std::f64::consts::LOG10_2;
        Decimal::from_float(log2 * log2).max(&Decimal::ONE)
    }

    // --- Replicanti Galaxies -------------------------------------------------

    /// Whether a Replicanti Galaxy can be bought: amount at the cap and below the
    /// bought-galaxy cap (`Replicanti.galaxies.canBuyMore`).
    pub fn can_buy_replicanti_galaxy(&self) -> bool {
        self.replicanti.amount >= REPLICANTI_CAP
            && self.replicanti.galaxies < self.replicanti.galaxy_cap
    }

    /// Buy one Replicanti Galaxy (`replicantiGalaxy`): reset replicanti to 1, add an
    /// RG, and perform an antimatter-galaxy-like soft reset with Dimension Boosts
    /// cleared (`addReplicantiGalaxies`: `dimensionBoosts = 0` then
    /// `softReset(0, true, true)`). Returns whether it happened.
    pub fn buy_replicanti_galaxy(&mut self) -> bool {
        if !self.can_buy_replicanti_galaxy() {
            return false;
        }
        self.replicanti.timer_ms = 0.0;
        self.replicanti.amount = Decimal::ONE;
        self.replicanti.galaxies += 1;
        // replicantiNoReset milestone (40 eternities): the RG no longer wipes
        // Dimension Boosts / dimensions / antimatter (`addReplicantiGalaxies`).
        if !self.eternity_milestone_reached(40) {
            self.dim_boosts = 0;
            self.dim_boost_reset();
        }
        true
    }

    // --- Upgrades ------------------------------------------------------------

    /// Whether the chance upgrade is capped (rounded chance ≥ 100%).
    pub fn replicanti_chance_capped(&self) -> bool {
        (self.replicanti.chance * 100.0).round() / 100.0 >= CHANCE_CAP
    }

    /// Whether the chance upgrade can be bought (not capped, affordable).
    pub fn can_buy_replicanti_chance(&self) -> bool {
        !self.replicanti_chance_capped()
            && self.infinity_points >= self.replicanti.chance_cost
    }

    /// Buy one chance upgrade: `+0.01` chance (rounded to the nearest %), cost ×1e15.
    pub fn buy_replicanti_chance(&mut self) -> bool {
        if !self.can_buy_replicanti_chance() {
            return false;
        }
        self.infinity_points -= self.replicanti.chance_cost;
        self.replicanti.chance_cost *= Decimal::from_float(CHANCE_COST_MULT);
        // nearestPercent(value + 0.01).
        self.replicanti.chance =
            ((self.replicanti.chance + 0.01) * 100.0).round() / 100.0;
        true
    }

    /// Whether the interval upgrade is capped (interval ≤ 50 ms).
    pub fn replicanti_interval_capped(&self) -> bool {
        self.replicanti.interval_ms <= INTERVAL_CAP_MS
    }

    /// Whether the interval upgrade can be bought (not capped, affordable).
    pub fn can_buy_replicanti_interval(&self) -> bool {
        !self.replicanti_interval_capped()
            && self.infinity_points >= self.replicanti.interval_cost
    }

    /// Buy one interval upgrade: `×0.9` ms (floored at 50 ms), cost ×1e10.
    pub fn buy_replicanti_interval(&mut self) -> bool {
        if !self.can_buy_replicanti_interval() {
            return false;
        }
        self.infinity_points -= self.replicanti.interval_cost;
        self.replicanti.interval_cost *= Decimal::from_float(INTERVAL_COST_MULT);
        self.replicanti.interval_ms =
            (self.replicanti.interval_ms * 0.9).max(INTERVAL_CAP_MS);
        true
    }

    /// The galaxy-cap upgrade's current IP cost, derived from `galaxy_cap`
    /// (`baseCostAfterCount`, ignoring the ≥100 distant / ≥1000 remote scaling far
    /// past our frontier): `10^(170 + 25·count + 5·count·(count−1)/2)`.
    pub fn replicanti_galaxy_cost(&self) -> Decimal {
        let count = self.replicanti.galaxy_cap as f64;
        let log_cost = 170.0 + 25.0 * count + 5.0 * count * (count - 1.0) / 2.0;
        Decimal::pow10(log_cost)
    }

    /// Whether the galaxy-cap upgrade can be bought (affordable).
    pub fn can_buy_replicanti_galaxy_cap(&self) -> bool {
        self.infinity_points >= self.replicanti_galaxy_cost()
    }

    /// Buy one galaxy-cap upgrade: `+1` max Replicanti Galaxy.
    pub fn buy_replicanti_galaxy_cap(&mut self) -> bool {
        let cost = self.replicanti_galaxy_cost();
        if self.infinity_points < cost {
            return false;
        }
        self.infinity_points -= cost;
        self.replicanti.galaxy_cap += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::constants::BIG_CRUNCH_THRESHOLD;

    fn unlocked_game() -> GameState {
        let mut game = GameState::new();
        game.infinity_points = Decimal::new(1.0, 141);
        assert!(game.unlock_replicanti());
        game.infinity_points = Decimal::ZERO;
        game
    }

    #[test]
    fn unlock_costs_ip_and_seeds_one() {
        let mut game = GameState::new();
        game.infinity_points = Decimal::new(1.0, 139); // just under 1e140
        assert!(!game.can_unlock_replicanti());
        assert!(!game.unlock_replicanti());

        game.infinity_points = Decimal::new(1.0, 141);
        assert!(game.can_unlock_replicanti());
        assert!(game.unlock_replicanti());
        assert!(game.replicanti.unlocked);
        assert_eq!(game.replicanti.amount, Decimal::ONE);
        // Spent exactly 1e140.
        assert_eq!(
            game.infinity_points,
            Decimal::new(1.0, 141) - Decimal::new(1.0, 140)
        );
    }

    #[test]
    fn growth_multiplies_amount_and_stays_capped() {
        let mut game = unlocked_game();
        // One 1000 ms interval at 1% chance → ×1.01.
        game.tick_replicanti(1000.0);
        assert_eq!(game.replicanti.amount, Decimal::from_float(1.01));

        // A huge burst clamps to the cap rather than exceeding it.
        game.replicanti.amount = Decimal::new(1.0, 300);
        game.replicanti.chance = 1.0;
        game.replicanti.interval_ms = 1.0;
        game.tick_replicanti(1000.0); // 1000 doublings from 1e300
        assert_eq!(game.replicanti.amount, REPLICANTI_CAP);
    }

    #[test]
    fn sub_interval_time_rolls_over() {
        let mut game = unlocked_game();
        game.replicanti.interval_ms = 1000.0;
        // Half an interval: no whole tick, amount unchanged, timer holds the 500 ms.
        game.tick_replicanti(500.0);
        assert_eq!(game.replicanti.amount, Decimal::ONE);
        assert_eq!(game.replicanti.timer_ms, 500.0);
        // Another 500 ms completes one tick.
        game.tick_replicanti(500.0);
        assert_eq!(game.replicanti.amount, Decimal::from_float(1.01));
    }

    #[test]
    fn replicanti_galaxies_feed_tickspeed() {
        let mut game = GameState::new();
        let before = game.tickspeed_purchase_multiplier();
        assert_eq!(game.effective_galaxies(), 0);

        game.replicanti.galaxies = 1;
        assert_eq!(game.effective_galaxies(), 1);
        // A galaxy lowers the per-purchase multiplier (faster tickspeed).
        assert!(game.tickspeed_purchase_multiplier() < before);
    }

    #[test]
    fn buying_rg_resets_and_grants_a_galaxy() {
        let mut game = unlocked_game();
        game.replicanti.galaxy_cap = 1;
        game.replicanti.amount = REPLICANTI_CAP;
        game.dim_boosts = 3;
        game.dimensions[0].amount = Decimal::from_float(1e10);
        game.dimensions[0].bought = 20;

        assert!(game.can_buy_replicanti_galaxy());
        assert!(game.buy_replicanti_galaxy());

        assert_eq!(game.replicanti.galaxies, 1);
        assert_eq!(game.replicanti.amount, Decimal::ONE);
        // addReplicantiGalaxies clears dim boosts and soft-resets the dimensions.
        assert_eq!(game.dim_boosts, 0);
        assert_eq!(game.dimensions[0].amount, Decimal::ZERO);
        assert_eq!(game.dimensions[0].bought, 0);
        // Cannot buy another without raising the cap.
        assert!(!game.can_buy_replicanti_galaxy());
    }

    #[test]
    fn chance_upgrade_steps_and_caps() {
        let mut game = unlocked_game();
        game.infinity_points = Decimal::new(1.0, 151);
        assert!(game.buy_replicanti_chance());
        assert_eq!(game.replicanti.chance, 0.02);
        assert_eq!(game.replicanti.chance_cost, Decimal::new(1.0, 165)); // 1e150×1e15

        // At 100% the upgrade is capped.
        game.replicanti.chance = 1.0;
        game.infinity_points = Decimal::MAX_VALUE;
        assert!(game.replicanti_chance_capped());
        assert!(!game.buy_replicanti_chance());
    }

    #[test]
    fn interval_upgrade_steps_and_floors() {
        let mut game = unlocked_game();
        game.infinity_points = Decimal::new(1.0, 141);
        assert!(game.buy_replicanti_interval());
        assert_eq!(game.replicanti.interval_ms, 900.0); // 1000 × 0.9
        assert_eq!(game.replicanti.interval_cost, Decimal::new(1.0, 150)); // 1e140×1e10

        // At the floor the upgrade is capped.
        game.replicanti.interval_ms = 50.0;
        game.infinity_points = Decimal::MAX_VALUE;
        assert!(game.replicanti_interval_capped());
        assert!(!game.buy_replicanti_interval());
    }

    #[test]
    fn galaxy_cap_cost_scales() {
        let mut game = unlocked_game();
        // count 0 → 1e170.
        assert_eq!(game.replicanti_galaxy_cost(), Decimal::new(1.0, 170));
        game.infinity_points = Decimal::new(1.0, 171);
        assert!(game.buy_replicanti_galaxy_cap());
        assert_eq!(game.replicanti.galaxy_cap, 1);
        // count 1 → 10^(170 + 25) = 1e195.
        assert_eq!(game.replicanti_galaxy_cost(), Decimal::new(1.0, 195));
    }

    #[test]
    fn mult_grows_with_amount_and_is_at_least_one() {
        let mut game = unlocked_game();
        // At amount 1, log2(1)^2 = 0 → clamped to 1.
        game.replicanti.amount = Decimal::ONE;
        assert_eq!(game.replicanti_mult(), Decimal::ONE);
        // At 2^10, log2 = 10 → mult = 100.
        game.replicanti.amount = Decimal::from_float(1024.0);
        assert_eq!(game.replicanti_mult(), Decimal::from_float(100.0));
    }

    #[test]
    fn replicanti_survive_a_crunch() {
        let mut game = unlocked_game();
        game.replicanti.amount = Decimal::from_float(1e6);
        game.replicanti.galaxies = 2;
        game.replicanti.chance = 0.05;

        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());

        // Replicanti are untouched by a Big Crunch (they reset only on Eternity).
        assert!(game.replicanti.unlocked);
        assert_eq!(game.replicanti.amount, Decimal::from_float(1e6));
        assert_eq!(game.replicanti.galaxies, 2);
        assert_eq!(game.replicanti.chance, 0.05);
    }
}
