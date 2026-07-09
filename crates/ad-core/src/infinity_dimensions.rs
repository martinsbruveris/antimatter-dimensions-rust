//! Infinity Dimensions (Feature 3.1): 8 tiers bought with Infinity Points that
//! produce Infinity Power — an `^7` multiplier to every Antimatter Dimension. IDs
//! persist across a Big Crunch (only their `amount` and the Infinity Power reset);
//! a full reset waits for Eternity. See `docs/design/2026-07-03-infinity-dimensions.md`.

use break_infinity::Decimal;

use crate::state::GameState;

/// Number of infinity dimension tiers.
pub const INFINITY_DIMENSION_COUNT: usize = 8;

/// `thisEternity.maxAM` needed to unlock each tier (0-indexed).
const ID_UNLOCK_AM: [Decimal; 8] = [
    Decimal::new_unchecked(1.0, 1100),
    Decimal::new_unchecked(1.0, 1900),
    Decimal::new_unchecked(1.0, 2400),
    Decimal::new_unchecked(1.0, 10500),
    Decimal::new_unchecked(1.0, 30000),
    Decimal::new_unchecked(1.0, 45000),
    Decimal::new_unchecked(1.0, 54000),
    Decimal::new_unchecked(1.0, 60000),
];

/// Base IP cost of each tier (0-indexed).
const ID_BASE_COST: [Decimal; 8] = [
    Decimal::new_unchecked(1.0, 8),
    Decimal::new_unchecked(1.0, 9),
    Decimal::new_unchecked(1.0, 10),
    Decimal::new_unchecked(1.0, 20),
    Decimal::new_unchecked(1.0, 140),
    Decimal::new_unchecked(1.0, 200),
    Decimal::new_unchecked(1.0, 250),
    Decimal::new_unchecked(1.0, 280),
];

/// IP cost multiplier per purchase (0-indexed).
const ID_COST_MULT: [f64; 8] = [1e3, 1e6, 1e8, 1e10, 1e15, 1e20, 1e25, 1e30];

/// Production multiplier granted per 10 owned (per purchase), 0-indexed.
const ID_POWER_MULT: [f64; 8] = [50.0, 30.0, 10.0, 5.0, 5.0, 5.0, 5.0, 5.0];

/// IP required for the tier-1 unlock, in addition to the antimatter requirement
/// (`hasIPUnlock`, pre-Eternity).
const ID1_IP_REQUIREMENT: Decimal = Decimal::new_unchecked(1.0, 8);

/// Purchase hardcap for tiers 1–7 (tier 8 is uncapped). Each purchase is 10 IDs.
const ID_PURCHASE_CAP: u64 = 2_000_000;

/// The Antimatter-Dimension multiplier exponent applied to Infinity Power
/// (`InfinityDimensions.powerConversionRate`, pre-glyphs).
const POWER_CONVERSION_RATE: f64 = 7.0;

/// One infinity dimension tier's mutable state.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InfinityDimension {
    /// Current amount (grows from higher-tier production; reset to `base_amount` on
    /// a Big Crunch).
    pub amount: Decimal,
    /// `10 × purchases` — the bought base. Persists across a Big Crunch. Drives the
    /// per-tier multiplier (`powerMultiplier^(base_amount/10)`).
    pub base_amount: u64,
    /// Next-purchase IP cost. Persists across a Big Crunch.
    pub cost: Decimal,
    /// Whether the tier is unlocked. Persists across a Big Crunch.
    pub is_unlocked: bool,
}

impl InfinityDimension {
    /// A fresh tier (0-indexed): locked, unbought, at its base cost.
    pub fn new(tier: usize) -> Self {
        Self {
            amount: Decimal::ZERO,
            base_amount: 0,
            cost: ID_BASE_COST[tier],
            is_unlocked: false,
        }
    }

    /// Number of purchases made (each gives 10 IDs).
    pub fn purchases(&self) -> u64 {
        self.base_amount / 10
    }
}

impl GameState {
    /// The purchase cap for tier `t` (uncapped for tier 8).
    fn id_purchase_cap(tier: usize) -> u64 {
        if tier == 7 {
            u64::MAX
        } else {
            ID_PURCHASE_CAP
        }
    }

    /// Whether tier `t` is at its purchase cap.
    pub fn id_is_capped(&self, tier: usize) -> bool {
        // Enslaved's Reality caps every Infinity Dimension at 1 purchase.
        let cap = if self.celestials.enslaved.run {
            1
        } else {
            Self::id_purchase_cap(tier)
        };
        self.infinity_dimensions[tier].purchases() >= cap
    }

    /// Whether tier `t` can be unlocked now: peak antimatter this eternity has
    /// reached its requirement (and, for tier 1, enough Infinity Points).
    pub fn can_unlock_infinity_dimension(&self, tier: usize) -> bool {
        if self.infinity_dimensions[tier].is_unlocked {
            return false;
        }
        // The IDR perk (51) removes the antimatter requirement.
        let am_ok = self.perk_bought(51)
            || self.records.this_eternity.max_am >= ID_UNLOCK_AM[tier];
        let ip_ok = tier != 0 || self.infinity_points >= ID1_IP_REQUIREMENT;
        am_ok && ip_ok
    }

    /// Unlock tier `t` if its requirements are met. Returns whether it is unlocked
    /// afterwards.
    pub fn unlock_infinity_dimension(&mut self, tier: usize) -> bool {
        if self.infinity_dimensions[tier].is_unlocked {
            return true;
        }
        if !self.can_unlock_infinity_dimension(tier) {
            return false;
        }
        self.infinity_dimensions[tier].is_unlocked = true;
        true
    }

    /// Whether tier `t` can be bought now (unlocked, affordable, not capped;
    /// `InfinityDimensions.canBuy` blocks purchases in EC2/EC10 and once EC8's
    /// budget is spent).
    pub fn id_available_for_purchase(&self, tier: usize) -> bool {
        if self.ec_running(2) || self.ec_running(10) {
            return false;
        }
        if self.ec_running(8) && self.eterc8_ids <= 0 {
            return false;
        }
        let d = &self.infinity_dimensions[tier];
        d.is_unlocked && self.infinity_points >= d.cost && !self.id_is_capped(tier)
    }

    /// Buy one purchase (10 IDs) of tier `t` — or unlock it first if it is locked.
    /// Returns whether anything happened.
    pub fn buy_infinity_dimension(&mut self, tier: usize) -> bool {
        if !self.infinity_dimensions[tier].is_unlocked {
            return self.unlock_infinity_dimension(tier);
        }
        if !self.id_available_for_purchase(tier) {
            return false;
        }
        let cost = self.infinity_dimensions[tier].cost;
        self.infinity_points -= cost;
        // EC12's reward softens the per-purchase cost multiplier.
        let mult = Decimal::from_float(ID_COST_MULT[tier].powf(self.ec12_id_cost_pow()));
        // EC8: each purchase spends the run's ID budget.
        if self.ec_running(8) {
            self.eterc8_ids -= 1;
        }
        let d = &mut self.infinity_dimensions[tier];
        d.cost = (cost * mult).round();
        d.amount += Decimal::from_float(10.0);
        d.base_amount += 10;
        true
    }

    /// Buy as many purchases of tier `t` as affordable, up to the cap.
    pub fn buy_max_infinity_dimension(&mut self, tier: usize) -> u64 {
        // Unlock first if needed.
        if !self.infinity_dimensions[tier].is_unlocked {
            self.unlock_infinity_dimension(tier);
        }
        let mut count = 0;
        while self.buy_infinity_dimension(tier) {
            count += 1;
        }
        count
    }

    /// Buy-max every Infinity Dimension (the "Max All" affordance).
    pub fn buy_max_all_infinity_dimensions(&mut self) {
        for tier in 0..INFINITY_DIMENSION_COUNT {
            self.buy_max_infinity_dimension(tier);
        }
    }

    /// The all-tier Infinity-Dimension multiplier: the completed IC1 reward
    /// (`×1.3^(IC completed count)`), the IC6 reward (`tickspeed_per_second^0.0005`),
    /// and the Replicanti multiplier while Replicanti are unlocked and above 1.
    /// (The achievement/time-study terms are later features.)
    pub fn id_common_multiplier(&self) -> Decimal {
        let mut mult = Decimal::ONE;
        // Achievement 75: the global achievement bonus also affects Infinity
        // Dimensions (`infinityDimensionCommonMultiplier` `timesEffectsOf`).
        if self.achievement_unlocked(75) {
            mult *= self.achievement_power();
        }
        if self.infinity_challenge_completed(1) {
            let completed = (1..=8u8)
                .filter(|&id| self.infinity_challenge_completed(id))
                .count();
            mult *= Decimal::from_float(1.3_f64.powi(completed as i32));
        }
        if self.infinity_challenge_completed(6) {
            mult *= self.tickspeed_effect().pow(&Decimal::from_float(0.0005));
        }
        if self.replicanti.unlocked && self.replicanti.amount > Decimal::ONE {
            mult *= self.replicanti_mult();
        }
        // TS82: Dimension Boosts affect Infinity Dimensions.
        if self.time_study_bought(82) {
            let boosts = self.dim_boosts as f64;
            mult *= Decimal::from_float(1.0000109)
                .pow(&Decimal::from_float(boosts * boosts))
                .min(&Decimal::new_unchecked(1.0, 10_000_000));
        }
        // TS92: based on the fastest Eternity (2^(60/max(t, 2)), cap 2^30).
        if self.time_study_bought(92) {
            let best_secs = (self.records.best_eternity.time_ms / 1000.0).max(2.0);
            let capped = Decimal::from_float(2.0)
                .pow(&Decimal::from_float(60.0 / best_secs))
                .min(&Decimal::from_float(2f64.powi(30)));
            mult *= capped;
        }
        // TS162: flat ×1e11.
        if self.time_study_bought(162) {
            mult *= Decimal::new_unchecked(1.0, 11);
        }
        // EC4's reward: ID multiplier from unspent IP.
        if self.ec_completed(4) {
            let completions = self.eternity_challenge_completions(4) as f64;
            mult *= self
                .infinity_points
                .max(&Decimal::ONE)
                .pow(&Decimal::from_float(0.003 + completions * 0.002))
                .min(&Decimal::new_unchecked(1.0, 200));
        }
        // Eternity Upgrades 1–3 (unspent EP / eternities / IC records).
        mult *= self.eternity_upgrade_id_mult();
        // EC9's reward: ID multiplier from Time Shards.
        if self.ec_completed(9) {
            let completions = self.eternity_challenge_completions(9) as f64;
            mult *= self
                .time_shards
                .max(&Decimal::ONE)
                .pow(&Decimal::from_float(completions * 0.1))
                .min(&Decimal::new_unchecked(1.0, 400));
        }
        // Ra Alchemy `dimensionality` (all-dim ×10^(5·amount)).
        mult *= Decimal::pow10(self.alchemy_dimensionality_log10());
        // Imaginary Upgrade 8 (Hyperbolic Apeirogon): ×1e100000 per purchase.
        mult *= self.imaginary_upgrade_id_mult();
        mult
    }

    /// Tier `t`'s production multiplier: `commonMult × powerMultiplier^purchases`
    /// (plus TS72's sacrifice term on the 4th dimension).
    pub fn id_multiplier(&self, tier: usize) -> Decimal {
        let purchases = self.infinity_dimensions[tier].purchases();
        // ID8's per-purchase multiplier is boosted by infinity-glyph sacrifice.
        let mut power_mult = ID_POWER_MULT[tier];
        if tier == INFINITY_DIMENSION_COUNT - 1 {
            power_mult *= self.glyph_sac_infinity_effect();
        }
        let mut mult = self.id_common_multiplier()
            * Decimal::from_float(power_mult).pow(&Decimal::from(purchases));
        // Achievement 94: double the 1st Infinity Dimension (doubles Infinity
        // Power gain).
        if tier == 0 && self.achievement_unlocked(94) {
            mult *= Decimal::from_float(2.0);
        }
        // EC2's reward: 1st-ID multiplier from Infinity Power.
        if tier == 0 && self.ec_completed(2) {
            let completions = self.eternity_challenge_completions(2) as f64;
            mult *= self
                .infinity_power
                .max(&Decimal::ONE)
                .pow(&Decimal::from_float(1.5 / (700.0 - completions * 100.0)))
                .min(&Decimal::new_unchecked(1.0, 100));
        }
        // TS72: sacrifice affects the 4th Infinity Dimension (greatly reduced).
        if tier == 3 && self.time_study_bought(72) {
            mult *= self
                .sacrifice_multiplier()
                .pow(&Decimal::from_float(0.04))
                .max(&Decimal::ONE)
                .min(&Decimal::new_unchecked(1.0, 30_000));
        }
        // The `infinitypow` glyph power, then Time Dilation compresses the
        // final multiplier.
        let infinitypow = self.glyph_effect_infinitypow();
        if infinitypow != 1.0 {
            mult = mult.pow(&Decimal::from_float(infinitypow));
        }
        // Ra Alchemy `infinity` (ID `^(1 + amount/200000)`) then `momentumValue`.
        let alch_inf =
            self.alchemy_dimension_power(crate::celestials::alchemy::INFINITY);
        if alch_inf != 1.0 {
            mult = mult.pow(&Decimal::from_float(alch_inf));
        }
        let momentum = self.ra_momentum_value();
        if momentum != 1.0 {
            mult = mult.pow(&Decimal::from_float(momentum));
        }
        if self.dilation.active {
            mult = self.dilated_value_of(mult);
        }
        mult
    }

    /// Tier `t`'s production per second (`amount × multiplier`; EC-modified).
    pub fn id_production_per_second(&self, tier: usize) -> Decimal {
        let d = &self.infinity_dimensions[tier];
        if !d.is_unlocked {
            return Decimal::ZERO;
        }
        // EC2/EC10: Infinity Dimensions are disabled.
        if self.ec_running(2) || self.ec_running(10) {
            return Decimal::ZERO;
        }
        // EC11: production without any multiplier.
        if self.ec_running(11) {
            return d.amount;
        }
        // Lai'tela's Reality disables dimensions above `maxAllowedDimension`.
        if self.laitela_dimension_disabled((tier + 1) as u32) {
            return Decimal::ZERO;
        }
        let mut production = d.amount * self.id_multiplier(tier);
        // EC7: Tickspeed directly applies to Infinity Dimensions.
        if self.ec_running(7) {
            production *= self.tickspeed_effect();
        }
        production
    }

    /// The Antimatter-Dimension multiplier from Infinity Power:
    /// `infinity_power ^ (7 + infinityrate glyph effect)`, clamped to ≥ 1.
    /// Read in `dimension_multiplier`.
    pub fn infinity_power_ad_multiplier(&self) -> Decimal {
        let rate = POWER_CONVERSION_RATE + self.glyph_effect_infinityrate();
        self.infinity_power
            .pow(&Decimal::from_float(rate))
            .max(&Decimal::ONE)
    }

    /// Advance Infinity Dimension production: each tier feeds the tier below
    /// (`diff/10`), and the 1st tier produces Infinity Power (`diff`). Mirrors
    /// `InfinityDimensions.tick`.
    pub fn tick_infinity_dimensions(&mut self, dt_ms: f64) {
        let dt_s = dt_ms / 1000.0;
        let dt10 = Decimal::from_float(dt_s / 10.0);
        // The original produces sequentially top-down, so each tier's production
        // reads the amount the tier above *just* added — the chain compounds within
        // the tick. Recompute the per-second rate inside the loop (as the AD loop
        // does) rather than snapshotting every tier's rate up front.
        for tier in (1..INFINITY_DIMENSION_COUNT).rev() {
            let produced = self.id_production_per_second(tier) * dt10;
            self.infinity_dimensions[tier - 1].amount += produced;
        }
        // The 1st Infinity Dimension then produces Infinity Power (`diff`) from its
        // just-updated amount — or, under EC7, 7th Antimatter Dimensions.
        let id1_prod = self.id_production_per_second(0);
        if self.ec_running(7) {
            self.dimensions[6].amount += id1_prod * Decimal::from_float(dt_s);
        } else {
            self.infinity_power += id1_prod * Decimal::from_float(dt_s);
        }
        // Track the peak 1st Infinity Dimension amount this Reality
        // (`reality.maxID1 = maxID1.clampMin(ID1.amount)`; gates Imaginary Up. 15).
        self.requirement_checks.reality_max_id1 = self
            .requirement_checks
            .reality_max_id1
            .max(&self.infinity_dimensions[0].amount);
    }

    /// Big-Crunch reset for Infinity Dimensions: Infinity Power → 0 and each tier's
    /// `amount` → its `base_amount` (purchases/cost/unlock persist). Mirrors
    /// `InfinityDimensions.resetAmount`.
    pub(crate) fn reset_infinity_dimension_amounts(&mut self) {
        self.infinity_power = Decimal::ZERO;
        for d in self.infinity_dimensions.iter_mut() {
            d.amount = Decimal::from(d.base_amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::constants::BIG_CRUNCH_THRESHOLD;

    #[test]
    fn infinity_dimension_production_compounds_within_a_tick() {
        let mut game = GameState::new();
        game.infinity_dimensions[0].is_unlocked = true;
        game.infinity_dimensions[1].is_unlocked = true;
        game.infinity_dimensions[0].base_amount = 100;
        game.infinity_dimensions[1].base_amount = 100;
        game.infinity_dimensions[0].amount = Decimal::from_float(1e10);
        game.infinity_dimensions[1].amount = Decimal::from_float(1e10);

        let dt_ms = 50.0;
        let dt10 = Decimal::from_float(dt_ms / 1000.0 / 10.0);
        let dt_s = Decimal::from_float(dt_ms / 1000.0);

        // Replicate the sequential (compounding) chain by hand: ID2 feeds ID1, then
        // the *updated* ID1 feeds Infinity Power.
        let mut reference = game.clone();
        let id2_prod = reference.id_production_per_second(1) * dt10;
        reference.infinity_dimensions[0].amount += id2_prod;
        let expected_power =
            reference.infinity_power + reference.id_production_per_second(0) * dt_s;

        game.tick_infinity_dimensions(dt_ms);

        let ratio = game.infinity_power.to_f64() / expected_power.to_f64();
        assert!((ratio - 1.0).abs() < 1e-12, "ratio={ratio}");
        // The 1st Infinity Dimension grew this tick (the compounding source).
        assert!(game.infinity_dimensions[0].amount > Decimal::from_float(1e10));
    }

    fn game_with_id1_unlockable() -> GameState {
        let mut game = GameState::new();
        game.records.this_eternity.max_am = Decimal::new(1.0, 1100);
        game.infinity_points = Decimal::new(1.0, 12);
        game
    }

    #[test]
    fn id1_requires_antimatter_and_ip_to_unlock() {
        let mut game = GameState::new();
        game.infinity_points = Decimal::new(1.0, 12);
        // Antimatter requirement not yet met.
        assert!(!game.can_unlock_infinity_dimension(0));
        game.records.this_eternity.max_am = Decimal::new(1.0, 1100);
        assert!(game.can_unlock_infinity_dimension(0));
        // Without the 1e8 IP, tier 1 stays locked.
        game.infinity_points = Decimal::new(1.0, 7);
        assert!(!game.can_unlock_infinity_dimension(0));
    }

    #[test]
    fn buying_unlocks_then_purchases_in_tens() {
        let mut game = game_with_id1_unlockable();
        // First buy unlocks.
        assert!(game.buy_infinity_dimension(0));
        assert!(game.infinity_dimensions[0].is_unlocked);
        assert_eq!(game.infinity_dimensions[0].base_amount, 0);

        // Next buy spends 1e8 IP and grants 10 IDs; cost ×1e3.
        let ip_before = game.infinity_points;
        assert!(game.buy_infinity_dimension(0));
        assert_eq!(game.infinity_dimensions[0].base_amount, 10);
        assert_eq!(
            game.infinity_dimensions[0].amount,
            Decimal::from_float(10.0)
        );
        assert_eq!(game.infinity_points, ip_before - Decimal::new(1.0, 8));
        assert_eq!(game.infinity_dimensions[0].cost, Decimal::new(1.0, 11)); // 1e8×1e3
    }

    #[test]
    fn id_multiplier_scales_with_purchases() {
        let mut game = game_with_id1_unlockable();
        game.buy_infinity_dimension(0); // unlock
        game.infinity_points = Decimal::new(1.0, 40);
        game.buy_infinity_dimension(0); // 1 purchase
        game.buy_infinity_dimension(0); // 2 purchases
                                        // powerMultiplier 50 ^ 2 purchases.
        assert_eq!(game.id_multiplier(0), Decimal::from_float(50.0 * 50.0));
    }

    #[test]
    fn id1_produces_infinity_power_which_boosts_ads() {
        let mut game = game_with_id1_unlockable();
        game.buy_infinity_dimension(0); // unlock
        game.buy_infinity_dimension(0); // 10 IDs
        assert_eq!(game.infinity_power, Decimal::ZERO);

        game.tick_infinity_dimensions(1000.0);
        assert!(game.infinity_power > Decimal::ZERO);
        // Infinity Power gives an AD multiplier ≥ 1.
        assert!(game.infinity_power_ad_multiplier() >= Decimal::ONE);
    }

    #[test]
    fn crunch_resets_power_and_amount_but_keeps_purchases() {
        let mut game = game_with_id1_unlockable();
        game.buy_infinity_dimension(0); // unlock
        game.buy_infinity_dimension(0); // 10 IDs, base_amount 10
        game.tick_infinity_dimensions(1000.0); // grow power + (nothing feeds ID1)
        game.infinity_power = Decimal::from_float(1e5);
        game.infinity_dimensions[0].amount = Decimal::from_float(1e9);

        game.antimatter = BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());

        // Power reset; amount back to the bought base; purchases/unlock kept.
        assert_eq!(game.infinity_power, Decimal::ZERO);
        assert_eq!(
            game.infinity_dimensions[0].amount,
            Decimal::from_float(10.0)
        );
        assert_eq!(game.infinity_dimensions[0].base_amount, 10);
        assert!(game.infinity_dimensions[0].is_unlocked);
    }
}
