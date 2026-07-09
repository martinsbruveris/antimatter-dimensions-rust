//! Replicanti (Feature 3.2): self-replicating entities unlocked with Infinity
//! Points. They grow continuously, multiply all Infinity Dimensions
//! (`replicanti_mult`), and can be spent on **Replicanti Galaxies** — resets that
//! add a galaxy to the tickspeed formula. Replicanti persist across a Big Crunch
//! (reset only on Eternity, a later feature).
//!
//! Pre-Eternity the mechanic simplifies sharply: `isUncapped` (TS192/Pelle) is
//! always false, so Replicanti stay capped at `Number.MAX_VALUE`, the over-cap
//! interval scaling never runs, and the speed multiplier / `extra` terms are ×1/×0.
//! See `docs/design/2026-07-03-replicanti.md`.

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

/// Interval scale factor per `1.8e308` of Replicanti above the cap
/// (`ReplicantiGrowth.scaleFactor`, the pre-alchemy 1.2 default).
const OVER_CAP_SCALE_FACTOR: f64 = 1.2;

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

    /// Whether Replicanti may exceed the 1.8e308 cap (`Replicanti.isUncapped`,
    /// TS192). Enslaved's Reality locks TS192.
    pub fn replicanti_uncapped(&self) -> bool {
        self.time_study_bought(192) && !self.celestials.enslaved.run
    }

    /// The total Replicanti speed multiplier (`totalReplicantiSpeedMult`):
    /// TS62 (×3), TS213 (×20), and TS132's ×1.5 (Achievement 134's pre-cap ×2
    /// is a later feature).
    fn replicanti_speed_mult(&self) -> f64 {
        let mut mult = 1.0;
        if self.time_study_bought(62) {
            mult *= 3.0;
        }
        if self.time_study_bought(213) {
            mult *= 20.0;
        }
        if self.time_study_bought(132) {
            // The PASS perk (31): TS132 also makes Replicanti ×3 faster.
            mult *= if self.perk_bought(31) { 3.0 } else { 1.5 };
        }
        // The `replicationspeed` glyph effect (a Decimal in the original;
        // frontier magnitudes fit f64).
        mult *= self.glyph_effect_replicationspeed().to_f64();
        // RU2 (Replicative Amplifier): ×3 per purchase.
        mult *= self.reality_rebuyable_effect(2).to_f64();
        // RU6: ×(1 + RG/50); RU23: fastest-reality boost (cap ×180).
        if self.reality_upgrade_bought(6) {
            mult *= 1.0 + self.replicanti.galaxies as f64 / 50.0;
        }
        if self.reality_upgrade_bought(23) {
            let best_minutes =
                (self.records.best_reality.time_ms / 60_000.0).clamp(1.0 / 12.0, 15.0);
            mult *= (15.0 / best_minutes).min(180.0);
        }
        // Ra: Alchemy `replication` speed × and the `continuousTTBoost.replicanti`
        // boost (both fold into the replicanti speed multiplier).
        mult *= self.alchemy_replication_speed();
        mult *= self.ra_tt_boost_replicanti().to_f64();
        mult
    }

    /// The effective reproduction interval (`getReplicantiInterval`): the base
    /// interval, ×10 under TS133 (pre-Achievement-138) or while over the cap,
    /// scaled up by ×1.2 per 1.8e308 above the cap, divided by the speed
    /// multiplier.
    fn replicanti_effective_interval(&self, over_cap: bool) -> f64 {
        let mut interval = self.replicanti.interval_ms;
        // Achievement 138 removes the TS133 ×10 downside.
        if (self.time_study_bought(133) && !self.achievement_unlocked(138)) || over_cap {
            interval *= 10.0;
        }
        if over_cap {
            let increases = (self.replicanti.amount.log10() - REPLICANTI_CAP.log10())
                / REPLICANTI_CAP.log10();
            interval *= OVER_CAP_SCALE_FACTOR.powf(increases);
        }
        interval /= self.replicanti_speed_mult();
        // Achievement 134: Replicanti grow 2× faster while under the cap.
        if !over_cap && self.achievement_unlocked(134) {
            interval /= 2.0;
        }
        // V's Reality squares the (post-speed) Replicanti interval.
        if self.celestials.v.run {
            interval = interval.powi(2);
        }
        interval
    }

    /// Grow Replicanti over `dt_ms` (`replicantiLoop`'s continuous "fast gain"
    /// path; the binomial/Poisson randomness at tiny amounts is dropped as a
    /// faithful aggregate). With TS192 the amount may pass the 1.8e308 cap;
    /// past it, growth follows the original's slowed formula
    /// `ln(new/old) = ln(1 + gain·s)/s` with `s = log10(1.2)/308.25`.
    pub fn tick_replicanti(&mut self, dt_ms: f64) {
        if !self.replicanti.unlocked {
            return;
        }
        let over_cap =
            self.replicanti_uncapped() && self.replicanti.amount > REPLICANTI_CAP;
        let interval = self.replicanti_effective_interval(over_cap);
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
        // Natural-log growth this tick: ticks × ln(1 + chance).
        let mut gain_ln = whole * (1.0 + self.replicanti.chance).ln();

        if !self.replicanti_uncapped() {
            let growth = Decimal::from_float(1.0 + self.replicanti.chance)
                .pow(&Decimal::from_float(whole));
            self.replicanti.amount =
                (self.replicanti.amount * growth).min(&REPLICANTI_CAP);
            return;
        }

        // Uncapped (TS192): spend gain up to the cap at full rate first.
        if self.replicanti.amount < REPLICANTI_CAP {
            let to_cap_ln =
                (REPLICANTI_CAP / self.replicanti.amount.max(&Decimal::ONE)).ln();
            if gain_ln <= to_cap_ln {
                let growth = Decimal::from_float(gain_ln).exp();
                self.replicanti.amount *= growth;
                return;
            }
            self.replicanti.amount = REPLICANTI_CAP;
            gain_ln -= to_cap_ln;
        }
        // Over the cap growth slows: post-scale per the original's formula.
        let post_scale = OVER_CAP_SCALE_FACTOR.log10() / REPLICANTI_CAP.log10();
        let new_ln =
            (1.0 + gain_ln * post_scale).ln() / post_scale + self.replicanti.amount.ln();
        self.replicanti.amount = Decimal::from_float(new_ln).exp();
    }

    /// Total galaxies feeding the tickspeed formula (`effectiveBaseGalaxies`):
    /// antimatter galaxies plus Replicanti Galaxies — RGs strengthened by
    /// TS132 (+40%) / TS133 (+50%) and joined by the "extra" RGs from
    /// TS225/TS226 (which the strength boosts do not affect).
    pub fn effective_galaxies(&self) -> u32 {
        let mut rgs = self.replicanti.galaxies as f64;
        let mut strength = 1.0;
        if self.time_study_bought(132) {
            strength += 0.4;
        }
        if self.time_study_bought(133) {
            strength += 0.5;
        }
        rgs *= strength;
        // EC8's reward: the RGs within the bought cap are strengthened by
        // Infinity Power (`nonActivePathReplicantiGalaxies × EC8 reward`).
        if self.ec_completed(8) {
            let in_cap = self.replicanti.galaxies.min(self.replicanti.galaxy_cap);
            rgs += in_cap as f64 * self.ec8_reward_rg_strength();
        }
        // Tachyon Galaxies are free galaxies too (`freeGalaxies`); the
        // fractional past-1000 part is floored here.
        let tachyon = self.dilation.total_tachyon_galaxies as u32;
        self.galaxies + rgs as u32 + self.extra_replicanti_galaxies() + tachyon
    }

    /// The "extra" Replicanti Galaxies (`Replicanti.galaxies.extra`): TS225
    /// (from the Replicanti amount's exponent) + TS226 (from the bought cap).
    pub fn extra_replicanti_galaxies(&self) -> u32 {
        let mut extra = 0;
        if self.time_study_bought(225) {
            extra += (self.replicanti.amount.exponent().max(0) / 1000) as u32;
        }
        if self.time_study_bought(226) {
            extra += self.replicanti.galaxy_cap / 15;
        }
        extra
    }

    /// Max purchasable Replicanti Galaxies (`Replicanti.galaxies.max`): the
    /// bought cap plus TS131's +50%.
    pub fn replicanti_galaxy_max(&self) -> u32 {
        let mut max = self.replicanti.galaxy_cap;
        if self.time_study_bought(131) {
            max += self.replicanti.galaxy_cap / 2;
        }
        max
    }

    /// Replicanti's multiplier to all Infinity Dimensions: `log2(max(amount, 1))^2`,
    /// clamped to ≥ 1 (`replicantiMult`, dropping the later TS/glyph terms). Folded
    /// into `id_common_multiplier` while `unlocked && amount > 1`.
    pub fn replicanti_mult(&self) -> Decimal {
        let log2 = self.replicanti.amount.max(&Decimal::ONE).log10()
            / std::f64::consts::LOG10_2;
        let mut mult = Decimal::from_float(log2 * log2);
        // TS21 adds `amount^0.032`; TS102 multiplies by `5^RGs`.
        if self.time_study_bought(21) {
            mult += self
                .replicanti
                .amount
                .max(&Decimal::ONE)
                .pow(&Decimal::from_float(0.032));
        }
        if self.time_study_bought(102) {
            mult *= Decimal::from_float(5.0)
                .pow(&Decimal::from(self.replicanti.galaxies as u64));
        }
        // The `replicationpow` glyph power on the clamped total.
        let mut mult = mult.max(&Decimal::ONE);
        let pow = self.glyph_effect_replicationpow();
        if pow != 1.0 {
            mult = mult.pow(&Decimal::from_float(pow));
        }
        mult
    }

    // --- Replicanti Galaxies -------------------------------------------------

    /// Whether a Replicanti Galaxy can be bought: amount at the cap and below the
    /// bought-galaxy cap (`Replicanti.galaxies.canBuyMore`).
    pub fn can_buy_replicanti_galaxy(&self) -> bool {
        self.replicanti.amount >= REPLICANTI_CAP
            && self.replicanti.galaxies < self.replicanti_galaxy_max()
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
        // Achievement 126: an RG divides Replicanti by 1.8e308 per galaxy gained
        // instead of resetting them to 1 (not while Doomed).
        self.replicanti.amount = if self.achievement_unlocked(126) && !self.is_doomed() {
            Decimal::pow10(
                self.replicanti.amount.log10() - Decimal::NUMBER_MAX_VALUE.log10(),
            )
        } else {
            Decimal::ONE
        };
        self.replicanti.galaxies += 1;
        // `player.requirementChecks.eternity.noRG = false` (spoils Reality
        // Upgrade 6's requirement for this eternity).
        self.requirement_checks.eternity_no_rg = false;
        // replicantiNoReset milestone (40 eternities): the RG no longer wipes
        // Dimension Boosts / dimensions / antimatter (`addReplicantiGalaxies`).
        if !self.eternity_milestone_reached(40) {
            self.dim_boosts = 0;
            // `softReset(0, true, true)` — forced (ANR does not apply).
            self.dim_boost_reset_forced();
        }
        true
    }

    // --- Upgrades ------------------------------------------------------------

    /// Whether the chance upgrade is capped (rounded chance ≥ 100%).
    pub fn replicanti_chance_capped(&self) -> bool {
        (self.replicanti.chance * 100.0).round() / 100.0 >= CHANCE_CAP
    }

    /// Whether EC8's Replicanti-upgrade budget still allows a purchase.
    fn ec8_repl_budget_ok(&self) -> bool {
        !self.ec_running(8) || self.eterc8_repl > 0
    }

    /// Spend one unit of EC8's Replicanti-upgrade budget (while it runs).
    fn ec8_spend_repl_budget(&mut self) {
        if self.ec_running(8) {
            self.eterc8_repl -= 1;
        }
    }

    /// Whether the chance upgrade can be bought (not capped, affordable).
    pub fn can_buy_replicanti_chance(&self) -> bool {
        !self.replicanti_chance_capped()
            && self.infinity_points >= self.replicanti.chance_cost
            && self.ec8_repl_budget_ok()
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
        self.ec8_spend_repl_budget();
        true
    }

    /// The interval upgrade floor (`Effects.min(50, TimeStudy(22))`): 50 ms,
    /// or 1 ms with TS22.
    fn replicanti_interval_floor(&self) -> f64 {
        if self.time_study_bought(22) {
            1.0
        } else {
            INTERVAL_CAP_MS
        }
    }

    /// Whether the interval upgrade is capped (interval at the floor).
    pub fn replicanti_interval_capped(&self) -> bool {
        self.replicanti.interval_ms <= self.replicanti_interval_floor()
    }

    /// Whether the interval upgrade can be bought (not capped, affordable).
    pub fn can_buy_replicanti_interval(&self) -> bool {
        !self.replicanti_interval_capped()
            && self.infinity_points >= self.replicanti.interval_cost
            && self.ec8_repl_budget_ok()
    }

    /// Buy one interval upgrade: `×0.9` ms (floored at 50 ms), cost ×1e10.
    pub fn buy_replicanti_interval(&mut self) -> bool {
        if !self.can_buy_replicanti_interval() {
            return false;
        }
        self.infinity_points -= self.replicanti.interval_cost;
        self.replicanti.interval_cost *= Decimal::from_float(INTERVAL_COST_MULT);
        self.replicanti.interval_ms =
            (self.replicanti.interval_ms * 0.9).max(self.replicanti_interval_floor());
        self.ec8_spend_repl_budget();
        true
    }

    /// The galaxy-cap upgrade's current IP cost, derived from `galaxy_cap`
    /// (`baseCostAfterCount`, ignoring the ≥100 distant / ≥1000 remote scaling far
    /// past our frontier): `10^(170 + 25·count + 5·count·(count−1)/2)`.
    pub fn replicanti_galaxy_cost(&self) -> Decimal {
        let count = self.replicanti.galaxy_cap as f64;
        // EC6 massively cheapens the upgrade (log increments 25/5 → 2/2).
        let (base_inc, scaling) = if self.ec_running(6) {
            (2.0, 2.0)
        } else {
            (25.0, 5.0)
        };
        let log_cost = 170.0 + base_inc * count + scaling * count * (count - 1.0) / 2.0;
        let mut cost = Decimal::pow10(log_cost);
        // TS233: cheaper based on the current Replicanti amount.
        if self.time_study_bought(233) {
            cost /= self
                .replicanti
                .amount
                .max(&Decimal::ONE)
                .pow(&Decimal::from_float(0.3));
        }
        cost
    }

    /// Whether the galaxy-cap upgrade can be bought (affordable).
    pub fn can_buy_replicanti_galaxy_cap(&self) -> bool {
        self.infinity_points >= self.replicanti_galaxy_cost()
            && self.ec8_repl_budget_ok()
    }

    /// Buy one galaxy-cap upgrade: `+1` max Replicanti Galaxy.
    pub fn buy_replicanti_galaxy_cap(&mut self) -> bool {
        let cost = self.replicanti_galaxy_cost();
        if self.infinity_points < cost {
            return false;
        }
        self.infinity_points -= cost;
        self.replicanti.galaxy_cap += 1;
        self.ec8_spend_repl_budget();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::constants::BIG_CRUNCH_THRESHOLD;

    #[test]
    fn replicanti_timer_carries_sub_interval_phase_across_a_load() {
        let mut base = GameState::new();
        base.replicanti.unlocked = true;
        base.replicanti.amount = Decimal::from_float(1e50);
        base.replicanti.chance = 0.5;
        base.replicanti.interval_ms = 100.0;
        assert_eq!(base.replicanti_effective_interval(false), 100.0);

        // A fresh (zero) timer: (50 + 0) / 100 = 0.5 of an interval — no growth.
        let mut g0 = base.clone();
        g0.tick_replicanti(50.0);
        assert_eq!(g0.replicanti.amount, Decimal::from_float(1e50));

        // A near-full timer completes an interval this tick: (50 + 60) / 100 = 1.1,
        // so Replicanti grow by ×(1 + chance).
        let mut g1 = base.clone();
        g1.replicanti.timer_ms = 60.0;
        g1.tick_replicanti(50.0);
        let ratio = g1.replicanti.amount.to_f64() / 1.5e50;
        assert!((ratio - 1.0).abs() < 1e-9, "{}", g1.replicanti.amount);
    }

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
    fn crunch_resets_replicanti_amount_and_galaxies() {
        // `secondSoftReset`: a Big Crunch resets the amount to 1 and RGs to 0
        // (the upgrades persist). Achievement 95 / TS33 keep some (below).
        let mut game = unlocked_game();
        game.replicanti.amount = Decimal::from_float(1e6);
        game.replicanti.galaxies = 2;
        game.replicanti.chance = 0.05;

        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());

        assert!(game.replicanti.unlocked);
        assert_eq!(game.replicanti.amount, Decimal::ONE);
        assert_eq!(game.replicanti.galaxies, 0);
        assert_eq!(game.replicanti.chance, 0.05);
    }

    #[test]
    fn ts33_keeps_half_the_rgs_on_crunch() {
        let mut game = unlocked_game();
        game.studies = vec![33];
        game.replicanti.galaxies = 5;
        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert_eq!(game.replicanti.galaxies, 2);
    }

    #[test]
    fn ts192_lets_replicanti_pass_the_cap() {
        let mut game = unlocked_game();
        game.studies = vec![192];
        game.replicanti.chance = 1.0;
        game.replicanti.interval_ms = 50.0;
        game.replicanti.amount = REPLICANTI_CAP;
        game.tick_replicanti(10_000.0);
        assert!(game.replicanti.amount > REPLICANTI_CAP);

        // Without TS192 the amount stays clamped.
        let mut capped = unlocked_game();
        capped.replicanti.chance = 1.0;
        capped.replicanti.interval_ms = 50.0;
        capped.replicanti.amount = REPLICANTI_CAP;
        capped.tick_replicanti(10_000.0);
        assert_eq!(capped.replicanti.amount, REPLICANTI_CAP);
    }
}
