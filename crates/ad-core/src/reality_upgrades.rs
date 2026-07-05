//! Reality Upgrades (Feature 6.4): 5 rebuyable "Amplifiers" plus 20 one-time
//! upgrades bought with Reality Machines. One-time upgrades must first meet a
//! requirement (tracked in `upgReqs`) checked at prestige events / per tick.
//!
//! Mirrors `src/core/reality-upgrades.js` and
//! `secret-formula/reality/reality-upgrades.js`; costs use the original's
//! hybrid scaling (`getHybridCostScaling`: geometric below 1e30, then
//! linearly-growing multiplier). RU13 (TD/EP autobuyers) and RU25 (Reality
//! autobuyer) are deferred with the automation they'd improve; the
//! requirement-lock QoL (`reqLock`) round-trips but isn't enforced. See
//! `design-docs/2026-07-05-reality.md`.

use break_infinity::Decimal;

use crate::state::GameState;

/// Rebuyables (ids 1–5): base cost, cost multiplier, effect base.
const REBUYABLE_INITIAL_COST: [f64; 5] = [1.0, 1.0, 2.0, 2.0, 3.0];
const REBUYABLE_COST_MULT: [f64; 5] = [30.0, 30.0, 30.0, 30.0, 50.0];
const REBUYABLE_EFFECT: [f64; 5] = [3.0, 3.0, 3.0, 3.0, 5.0];

/// One-time upgrade RM costs, ids 6–25.
const UPGRADE_COSTS: [f64; 20] = [
    15.0, 15.0, 15.0, 15.0, 15.0, // 6–10
    50.0, 50.0, 50.0, 50.0, 50.0, // 11–15
    1500.0, 1500.0, 1500.0, 1500.0, 1500.0, // 16–20
    100_000.0, 100_000.0, 100_000.0, 100_000.0, 100_000.0, // 21–25
];

/// `LinearMultiplierScaling.logTotalMultiplierAfterPurchases` (math.js): the
/// natural log of the combined multiplier after `count` purchases where the
/// per-purchase ratio grows linearly (`base + n·growth`).
fn linear_scaling_log_total(base: f64, growth: f64, count: f64) -> f64 {
    if count == 0.0 {
        return 0.0;
    }
    let k = growth / base;
    let u = k * count;
    (1.0 / k + count - 0.5) * u.ln_1p() + count * (base.ln() - 1.0)
        - k * u / (12.0 * (1.0 + u))
}

/// `getCostWithLinearCostScaling`: geometric up to `scaling_start`, then the
/// linearly-growing multiplier takes over.
pub(crate) fn linear_cost_scaling(
    purchases: f64,
    scaling_start: f64,
    initial_cost: f64,
    cost_mult: f64,
    cost_mult_growth: f64,
) -> f64 {
    let pre_purchases = ((scaling_start / initial_cost).ln() / cost_mult.ln())
        .floor()
        .max(0.0);
    let pre_cost = (cost_mult.powf(pre_purchases.min(purchases)) * initial_cost).ceil();
    let post = linear_scaling_log_total(
        cost_mult,
        cost_mult_growth,
        (purchases - pre_purchases).max(0.0),
    )
    .exp();
    pre_cost * post
}

impl GameState {
    // --- Rebuyables (ids 1–5) ---------------------------------------------------

    /// Purchase count of rebuyable `id` (1–5).
    pub fn reality_rebuyable_count(&self, id: u8) -> u32 {
        self.reality.rebuyables[(id - 1) as usize]
    }

    /// The RM cost of rebuyable `id` (`getHybridCostScaling`; the frontier
    /// never exceeds f64 range here — the exponential branch starts at 1e309).
    pub fn reality_rebuyable_cost(&self, id: u8) -> Decimal {
        let i = (id - 1) as usize;
        Decimal::from_float(linear_cost_scaling(
            self.reality.rebuyables[i] as f64,
            1e30,
            REBUYABLE_INITIAL_COST[i],
            REBUYABLE_COST_MULT[i],
            REBUYABLE_COST_MULT[i] / 10.0,
        ))
    }

    /// Rebuyable `id`'s effect multiplier (`effect^count`).
    pub fn reality_rebuyable_effect(&self, id: u8) -> Decimal {
        let i = (id - 1) as usize;
        Decimal::from_float(REBUYABLE_EFFECT[i])
            .pow(&Decimal::from(self.reality.rebuyables[i] as u64))
    }

    /// Buy one purchase of rebuyable `id`.
    pub fn buy_reality_rebuyable(&mut self, id: u8) -> bool {
        if !(1..=5).contains(&id) {
            return false;
        }
        let cost = self.reality_rebuyable_cost(id);
        if self.reality.machines < cost {
            return false;
        }
        self.reality.machines -= cost;
        self.reality.rebuyables[(id - 1) as usize] += 1;
        true
    }

    // --- One-time upgrades (ids 6–25) ---------------------------------------------

    /// The RM cost of one-time upgrade `id`.
    pub fn reality_upgrade_cost(id: u8) -> Decimal {
        if (6..=25).contains(&id) {
            Decimal::from_float(UPGRADE_COSTS[(id - 6) as usize])
        } else {
            Decimal::ZERO
        }
    }

    /// Whether upgrade `id`'s unlock requirement has been met
    /// (`isAvailableForPurchase`, the `upgReqs` bit).
    pub fn reality_upgrade_req_met(&self, id: u8) -> bool {
        self.reality.upg_reqs & (1u32 << id) != 0
    }

    /// Whether one-time upgrade `id` can be bought now.
    pub fn can_buy_reality_upgrade(&self, id: u8) -> bool {
        (6..=25).contains(&id)
            && !self.reality_upgrade_bought(id)
            && self.reality_upgrade_req_met(id)
            && self.reality.machines >= Self::reality_upgrade_cost(id)
    }

    /// Buy one-time upgrade `id`, with the original's purchase side effects.
    pub fn buy_reality_upgrade(&mut self, id: u8) -> bool {
        if !self.can_buy_reality_upgrade(id) {
            return false;
        }
        self.reality.machines -= Self::reality_upgrade_cost(id);
        self.reality.upgrade_bits |= 1u32 << id;
        if id == 10 {
            self.apply_rupg10_impl();
        }
        if id == 20 {
            self.unlock_second_black_hole();
        }
        true
    }

    // --- Requirement checks ---------------------------------------------------------

    /// Whether requirement checks run at all (`tryUnlock`'s `realityReached`).
    fn reality_reached(&self) -> bool {
        self.reality_unlocked() || self.dilation_study_bought(6)
    }

    fn try_unlock_reality_upgrade(&mut self, id: u8, condition: bool) {
        if condition && !self.reality_upgrade_req_met(id) {
            self.reality.upg_reqs |= 1u32 << id;
        }
    }

    /// Requirement checks fired at ETERNITY_RESET_BEFORE — before rewards
    /// and before `noEternities` is cleared.
    pub(crate) fn check_reality_upgrade_reqs_on_eternity_before(&mut self) {
        if !self.reality_reached() {
            return;
        }
        // RU6: first manual Eternity without Replicanti Galaxies.
        self.try_unlock_reality_upgrade(
            6,
            self.requirement_checks.eternity_no_rg
                && self.requirement_checks.reality_no_eternities,
        );
        // RU8: Eternity without auto-achievements.
        self.try_unlock_reality_upgrade(8, !self.reality.gained_auto_achievements);
        // RU10: first manual Eternity with ≥ 1e400 IP.
        self.try_unlock_reality_upgrade(
            10,
            self.infinity_points.exponent() >= 400
                && self.requirement_checks.reality_no_eternities,
        );
    }

    /// Requirement checks fired at ETERNITY_RESET_AFTER (EP includes the
    /// just-awarded gain; it persists through the reset).
    pub(crate) fn check_reality_upgrade_reqs_on_eternity_after(&mut self) {
        if !self.reality_reached() {
            return;
        }
        // RU9: Eternity for 1e4000 EP with exactly one level-3+ glyph equipped.
        let active: Vec<(u32, f64, u32)> = self
            .active_glyphs_without_companion()
            .iter()
            .map(|g| (g.level, g.strength, g.effects.count_ones()))
            .collect();
        self.try_unlock_reality_upgrade(
            9,
            self.eternity_points.exponent() >= 4000
                && active.len() == 1
                && active[0].0 >= 3,
        );
        // RU12: Eternity for 1e70 EP without EC1 completions.
        self.try_unlock_reality_upgrade(
            12,
            self.eternity_points.exponent() >= 70
                && self.eternity_challenge_completions(1) == 0,
        );
        // RU13: Eternity for 1e4000 EP without TD5–8.
        self.try_unlock_reality_upgrade(
            13,
            self.eternity_points.exponent() >= 4000
                && self.time_dimensions[4..8]
                    .iter()
                    .all(|d| d.amount == Decimal::ZERO),
        );
        // RU15: 1e10 EP without the ×5 EP upgrade.
        self.try_unlock_reality_upgrade(
            15,
            self.eternity_points.exponent() >= 10 && self.epmult_upgrades == 0,
        );
        // RU25: best-ever EP ≥ 1e11111.
        self.try_unlock_reality_upgrade(
            25,
            self.records.best_reality.best_ep.exponent() >= 11111,
        );
    }

    /// Requirement checks fired on a Big Crunch (before the reset).
    pub(crate) fn check_reality_upgrade_reqs_on_crunch(&mut self) {
        if !self.reality_reached() {
            return;
        }
        // RU7: first Infinity with at most 1 Antimatter Galaxy.
        self.try_unlock_reality_upgrade(
            7,
            self.galaxies <= 1 && self.requirement_checks.reality_no_infinities,
        );
    }

    /// Requirement checks fired on a Reality (before the reset).
    pub(crate) fn check_reality_upgrade_reqs_on_reality(&mut self) {
        if !self.reality_reached() {
            return;
        }
        let active: Vec<(u32, f64, u32)> = self
            .active_glyphs_without_companion()
            .iter()
            .map(|g| (g.level, g.strength, g.effects.count_ones()))
            .collect();
        // RU16: 4 equipped glyphs of uncommon+ rarity.
        self.try_unlock_reality_upgrade(
            16,
            active.iter().filter(|g| g.1 >= 1.5).count() == 4,
        );
        // RU17: 4 equipped glyphs with ≥ 2 effects each.
        self.try_unlock_reality_upgrade(
            17,
            active.iter().filter(|g| g.2 >= 2).count() == 4,
        );
        // RU18: 4 equipped glyphs at level ≥ 10.
        self.try_unlock_reality_upgrade(
            18,
            active.iter().filter(|g| g.0 >= 10).count() == 4,
        );
        // RU19: 30+ total glyphs.
        let total_glyphs = self
            .reality
            .glyphs
            .active
            .iter()
            .chain(self.reality.glyphs.inventory.iter())
            .filter(|g| g.kind != crate::GlyphType::Companion)
            .count();
        self.try_unlock_reality_upgrade(19, total_glyphs >= 30);
        // RU23: Reality in under 15 minutes of game time.
        self.try_unlock_reality_upgrade(
            23,
            self.records.this_reality.time_ms < 15.0 * 60_000.0,
        );
        // RU24: Reality for 5000 RM with no glyphs equipped.
        let gained = self.gained_reality_machines();
        self.try_unlock_reality_upgrade(
            24,
            gained >= Decimal::from_float(5000.0) && active.is_empty(),
        );
    }

    /// Per-tick requirement checks (the original's GAME_TICK_AFTER set).
    pub(crate) fn check_reality_upgrade_reqs_on_tick(&mut self) {
        if !self.reality_reached() || self.reality.upgrade_bits == u32::MAX {
            return;
        }
        // RU11: 1e12 Banked Infinities.
        self.try_unlock_reality_upgrade(11, self.infinities_banked.exponent() >= 12);
        // RU14: 1e7 Eternities.
        self.try_unlock_reality_upgrade(14, self.eternities >= Decimal::from_float(1e7));
        // RU20: 100 days play time after unlocking the Black Hole.
        self.try_unlock_reality_upgrade(
            20,
            self.black_holes.holes[0].unlocked
                && self.records.total_time_played_ms
                    - self.records.time_played_at_bh_unlock_ms
                    >= 100.0 * 86_400_000.0,
        );
        // RU21: 2800 total galaxies of all types.
        let total = self.replicanti.galaxies as f64
            + self.galaxies as f64
            + self.dilation.total_tachyon_galaxies;
        self.try_unlock_reality_upgrade(21, total >= 2800.0);
        // RU22: 1e28000 Time Shards.
        self.try_unlock_reality_upgrade(22, self.time_shards.exponent() >= 28_000);
    }

    /// Per-tick continuous generation from RU11 (10%/s of the crunch Infinity
    /// gain) and RU14 (Eternities equal to the Reality count per second).
    pub(crate) fn tick_reality_upgrade_generation(&mut self, dt_ms: f64) {
        let dt_s = dt_ms / 1000.0;
        if self.reality_upgrade_bought(11) {
            let gain = self.gained_infinities() * Decimal::from_float(0.1 * dt_s);
            self.infinities += gain;
        }
        if self.reality_upgrade_bought(14) && self.reality.realities > 0 {
            self.eternities += Decimal::from_float(self.reality.realities as f64 * dt_s);
        }
    }

    /// `applyRUPG10`: the start-of-reality package. Also run when the upgrade
    /// is first purchased.
    pub(crate) fn apply_rupg10_impl(&mut self) {
        // Complete all Normal Challenges (re-granting autobuyer rewards).
        for id in 1..=crate::NORMAL_CHALLENGE_COUNT {
            self.complete_challenge(id);
        }
        // Maxed AD/Tickspeed autobuyers (interval 100 ms, unlocked).
        for ab in self.autobuyers.dimensions.iter_mut() {
            ab.is_bought = true;
            ab.interval_ms = 100.0;
        }
        self.autobuyers.tickspeed.is_bought = true;
        self.autobuyers.tickspeed.interval_ms = 100.0;
        for ab in [
            &mut self.autobuyers.dim_boost,
            &mut self.autobuyers.galaxy,
            &mut self.autobuyers.big_crunch,
        ] {
            ab.interval_ms = 100.0;
        }
        self.dim_boosts = self.dim_boosts.max(4);
        self.galaxies = self.galaxies.max(1);
        self.broke_infinity = true;
        self.eternities = self.eternities.max(&Decimal::from_float(100.0));
        if self.replicanti.amount < Decimal::ONE {
            self.replicanti.amount = Decimal::ONE;
        }
        self.replicanti.unlocked = true;
        self.apply_eu1();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn realitied_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game.reality.machines = Decimal::from_float(1e6);
        game
    }

    #[test]
    fn rebuyable_costs_scale_geometrically_below_1e30() {
        let mut game = realitied_game();
        assert_eq!(game.reality_rebuyable_cost(1), Decimal::ONE);
        assert!(game.buy_reality_rebuyable(1));
        assert_eq!(game.reality_rebuyable_cost(1), Decimal::from_float(30.0));
        assert!(game.buy_reality_rebuyable(1));
        assert_eq!(game.reality_rebuyable_cost(1), Decimal::from_float(900.0));
        assert_eq!(game.reality_rebuyable_effect(1), Decimal::from_float(9.0));
        // RU5 (Boundless): base 3, ×50, effect 5.
        assert_eq!(game.reality_rebuyable_cost(5), Decimal::from_float(3.0));
        assert!(game.buy_reality_rebuyable(5));
        assert_eq!(game.reality_rebuyable_cost(5), Decimal::from_float(150.0));
        assert_eq!(game.reality_rebuyable_effect(5), Decimal::from_float(5.0));
    }

    #[test]
    fn one_time_upgrades_need_their_requirement() {
        let mut game = realitied_game();
        assert!(!game.can_buy_reality_upgrade(6));
        // A no-RG manual eternity meets RU6.
        game.check_reality_upgrade_reqs_on_eternity_before();
        assert!(game.reality_upgrade_req_met(6));
        assert!(game.buy_reality_upgrade(6));
        assert!(game.reality_upgrade_bought(6));
        assert_eq!(game.reality.machines, Decimal::from_float(1e6 - 15.0));
        // Once bought, can't rebuy.
        assert!(!game.can_buy_reality_upgrade(6));
    }

    #[test]
    fn ru7_needs_low_galaxies_and_no_infinities() {
        let mut game = realitied_game();
        game.galaxies = 2;
        game.check_reality_upgrade_reqs_on_crunch();
        assert!(!game.reality_upgrade_req_met(7));
        game.galaxies = 1;
        game.requirement_checks.reality_no_infinities = false;
        game.check_reality_upgrade_reqs_on_crunch();
        assert!(!game.reality_upgrade_req_met(7));
        game.requirement_checks.reality_no_infinities = true;
        game.check_reality_upgrade_reqs_on_crunch();
        assert!(game.reality_upgrade_req_met(7));
    }

    #[test]
    fn rupg10_grants_the_starting_package() {
        let mut game = realitied_game();
        game.infinity_points = Decimal::new(1.0, 400);
        game.check_reality_upgrade_reqs_on_eternity_before();
        assert!(game.reality_upgrade_req_met(10));
        assert!(game.buy_reality_upgrade(10));
        assert!(game.challenge_completed(12));
        assert!(game.broke_infinity);
        assert_eq!(game.dim_boosts, 4);
        assert_eq!(game.galaxies, 1);
        assert_eq!(game.eternities, Decimal::from_float(100.0));
        assert!(game.replicanti.unlocked);
        assert!(game.autobuyers.dimensions[0].is_bought);
        assert_eq!(game.autobuyers.dimensions[0].interval_ms, 100.0);
    }

    #[test]
    fn continuous_generation_from_ru11_and_ru14() {
        let mut game = realitied_game();
        game.reality.upgrade_bits |= (1 << 11) | (1 << 14);
        game.reality.realities = 3;
        game.tick_reality_upgrade_generation(1000.0);
        // RU11: 10% of gained_infinities (1) per second.
        assert_eq!(game.infinities, Decimal::from_float(0.1));
        // RU14: +realities eternities per second.
        assert_eq!(game.eternities, Decimal::from_float(3.0));
    }

    #[test]
    fn glyph_requirements_check_on_reality() {
        let mut game = realitied_game();
        for i in 0..4u32 {
            game.reality.glyphs.active.push(crate::Glyph {
                id: i + 1,
                idx: i,
                kind: crate::GlyphType::Power,
                strength: 2.0,
                level: 12,
                raw_level: 12,
                effects: (1 << 16) | (1 << 17),
            });
        }
        game.check_reality_upgrade_reqs_on_reality();
        assert!(game.reality_upgrade_req_met(16));
        assert!(game.reality_upgrade_req_met(17));
        assert!(game.reality_upgrade_req_met(18));
        // RU24 needs *no* glyphs equipped.
        assert!(!game.reality_upgrade_req_met(24));
    }
}
