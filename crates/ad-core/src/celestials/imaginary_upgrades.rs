//! Imaginary Machines + Imaginary Upgrades (Feature 6.4-late / gate for 7.6).
//! Imaginary Machines are gained once RM approaches its cap; the 25 Imaginary
//! Upgrades (10 rebuyable + 15 one-time, requirement-gated) unlock the endgame —
//! most importantly upgrade 15 (Lai'tela + Continuum), 16–18 (Dark Matter
//! Dimensions), 19 (Annihilation), and 25 (Pelle). See
//! `docs/design/2026-07-07-laitela.md` §0. Original: `imaginary-upgrades.js` +
//! `secret-formula/reality/imaginary-upgrades.js` + `machines.js`.
//!
//! **Scope.** The iM currency + gain, the rebuyable costs/effects, and the
//! one-time costs + purchase + the observable requirement auto-checks for the
//! Lai'tela-relevant upgrades (15/16/17/18/19/20/21/25). The deep requirement
//! upgrades (11–14, 22–24) store their bit but their requirements need unbuilt
//! records and never auto-satisfy (documented). `canLock` re-lock is out of
//! frontier.

use crate::state::GameState;
use break_infinity::Decimal;

/// `(initial_cost, cost_mult, effect)` for rebuyable ids 1–10.
pub const IMAGINARY_REBUYABLES: [(f64, f64, f64); 10] = [
    (3.0, 60.0, 0.15),   // 1 Temporal Intensifier (RU1 amplifier +0.15)
    (4.0, 60.0, 0.15),   // 2 Replicative Intensifier
    (1.0, 40.0, 0.4),    // 3 Eternal Intensifier
    (5.0, 80.0, 0.15),   // 4 Superluminal Intensifier
    (1.0, 30.0, 0.6),    // 5 Boundless Intensifier
    (1e4, 500.0, 1e100), // 6 Elliptic Materiality (RM cap ×1e100/buy — decimal)
    (2e5, 500.0, 200.0), // 7 Runic Assurance (+200 instability start)
    (1e7, 800.0, 1e5), // 8 Hyperbolic Apeirogon (ID ×1e100000/buy — decimal, see below)
    (1e9, 1000.0, 0.03), // 9 Cosmic Filament (galaxy strength +3%)
    (8e9, 2000.0, 1.0), // 10 Entropic Condensing (singularity gain +1/buy)
];

/// One-time upgrade costs, ids 11–25.
pub const IMAGINARY_ONETIME_COSTS: [(u8, f64); 15] = [
    (11, 5e7),
    (12, 5e7),
    (13, 5e7),
    (14, 5e7),
    (15, 1e9),
    (16, 3.5e9),
    (17, 6e9),
    (18, 1.5e10),
    (19, 2.8e10),
    (20, 3e12),
    (21, 1e13),
    (22, 1.5e14),
    (23, 6e14),
    (24, 6e14),
    (25, 1.6e15),
];

impl GameState {
    // --- Imaginary Machines -----------------------------------------------------

    /// `MachineHandler.baseIMCap` from the uncapped RM.
    fn base_im_cap(&self) -> f64 {
        let log_rm = self.uncapped_rm().pos_log10();
        (log_rm - 1000.0).max(0.0).powi(2) * (log_rm - 100000.0).max(1.0).powf(0.2)
    }

    /// The iM cap right now (`currentIMCap = player.reality.iMCap × IU13`).
    pub fn imaginary_machine_cap(&self) -> f64 {
        self.reality.im_cap * self.imaginary_upgrade_13_effect()
    }

    /// Imaginary Upgrade 13's cap multiplier:
    /// `1 + totalRebuyables/20 + totalSinglePurchase/2`.
    fn imaginary_upgrade_13_effect(&self) -> f64 {
        if !self.imaginary_upgrade_bought(13) {
            return 1.0;
        }
        let rebuyables: u32 = self.reality.imaginary_rebuyables.iter().sum();
        let one_time = self.reality.imaginary_upgrade_bits.count_ones();
        1.0 + rebuyables as f64 / 20.0 + one_time as f64 / 2.0
    }

    /// Approach the iM cap over real time (`gainedImaginaryMachines`), after
    /// ratcheting the stored base cap (`MachineHandler.updateIMCap`).
    pub(crate) fn tick_imaginary_machines(&mut self, dt_ms: f64) {
        let base_cap = self.base_im_cap();
        if base_cap > self.reality.im_cap {
            self.reality.im_cap = base_cap;
        }
        let cap = self.imaginary_machine_cap();
        if cap <= 0.0 {
            return;
        }
        // scaleTime = 60 / iU20 effect (iU20 → ×10 speed).
        let scale = 60.0
            / if self.imaginary_upgrade_bought(20) {
                10.0
            } else {
                1.0
            };
        let im = self.reality.imaginary_machines.to_f64();
        let gained = (cap - im) * (1.0 - 2f64.powf(-dt_ms / 1000.0 / scale));
        if gained > 0.0 {
            self.reality.imaginary_machines += Decimal::from_float(gained);
            self.reality.max_im =
                self.reality.max_im.max(&self.reality.imaginary_machines);
        }
    }

    // --- Upgrade queries --------------------------------------------------------

    /// Whether one-time Imaginary Upgrade `id` (11–25) is bought.
    pub fn imaginary_upgrade_bought(&self, id: u8) -> bool {
        self.reality.imaginary_upgrade_bits & (1u32 << id) != 0
    }

    pub fn imaginary_rebuyable_count(&self, id: u8) -> u32 {
        self.reality.imaginary_rebuyables[(id - 1) as usize]
    }

    /// The additive rebuyable effect `effect × count` (ids 1–5, 7, 9, 10).
    pub(crate) fn imaginary_rebuyable_effect(&self, id: u8) -> f64 {
        let (_, _, effect) = IMAGINARY_REBUYABLES[(id - 1) as usize];
        effect * self.imaginary_rebuyable_count(id) as f64
    }

    /// The Imaginary-Upgrade 8 Infinity-Dimension multiplier (`1e100000^count`).
    pub(crate) fn imaginary_upgrade_id_mult(&self) -> Decimal {
        let count = self.imaginary_rebuyable_count(8);
        if count == 0 {
            Decimal::ONE
        } else {
            Decimal::new(1.0, 100_000 * count as i64)
        }
    }

    pub fn imaginary_rebuyable_cost(&self, id: u8) -> f64 {
        let (initial, mult, _) = IMAGINARY_REBUYABLES[(id - 1) as usize];
        initial * mult.powi(self.imaginary_rebuyable_count(id) as i32)
    }

    pub fn imaginary_upgrade_cost(&self, id: u8) -> f64 {
        IMAGINARY_ONETIME_COSTS
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, c)| *c)
            .unwrap_or(f64::INFINITY)
    }

    // --- Purchasing -------------------------------------------------------------

    pub fn buy_imaginary_rebuyable(&mut self, id: u8) -> bool {
        if !(1..=10).contains(&id) {
            return false;
        }
        let cost = self.imaginary_rebuyable_cost(id);
        if self.reality.imaginary_machines < Decimal::from_float(cost) {
            return false;
        }
        self.reality.imaginary_machines -= Decimal::from_float(cost);
        self.reality.imaginary_rebuyables[(id - 1) as usize] += 1;
        true
    }

    /// Per-tick requirement latching for the deep Imaginary Upgrades
    /// (`checkRequirement` on GAME_TICK_AFTER → `imaginaryUpgReqs`). The
    /// reset-event checks (11/12) run from the Reality reset hooks.
    pub(crate) fn check_imaginary_upgrade_reqs_on_tick(&mut self) {
        // 13: ≥ Number.MAX_VALUE *projected* RM inside Enslaved's Reality —
        // `uncappedRM × (simulatedRealityCount(false) + 1)`, matching the
        // amplified value the Reality button displays.
        let projected_mult = break_infinity::Decimal::from_float(
            self.simulated_reality_count_raw().floor() + 1.0,
        );
        if self.celestials.enslaved.run
            && self.uncapped_rm() * projected_mult
                >= break_infinity::Decimal::NUMBER_MAX_VALUE
        {
            self.reality.imaginary_upg_reqs |= 1 << 13;
        }
        // 14: tickspeed ≥ 1e7.5e10 /sec inside EC5.
        if self.ec_running(5) {
            let per_second = break_infinity::Decimal::from_float(1000.0)
                / self.current_tickspeed_ms();
            if per_second.exponent() >= 75_000_000_000 {
                self.reality.imaginary_upg_reqs |= 1 << 14;
            }
        }
        // 22: 1e1.5e11 AM in Effarig's Reality with ≥ 4 Cursed Glyphs
        // (maxGlyphs < −10; cursed glyphs are unmodelled, so this stays
        // unreachable but the check is wired).
        if self.celestials.effarig.run
            && self.requirement_checks.reality_max_glyphs < -10
            && self.antimatter.exponent() >= 150_000_000_000
        {
            self.reality.imaginary_upg_reqs |= 1 << 22;
        }
        // 23: glyph level ≥ 20000 in Ra's Reality with no equipped glyphs.
        if self.celestials.ra.run
            && self.requirement_checks.reality_max_glyphs <= 0
            && self.gained_glyph_level().actual_level >= 20_000
        {
            self.reality.imaginary_upg_reqs |= 1 << 23;
        }
        // 24: 13000 Antimatter Galaxies in Ra's Reality with a fully inverted
        // Black Hole.
        if self.celestials.ra.run
            && self.requirement_checks.reality_slowest_bh <= 1e-300
            && self.galaxies >= 13_000
        {
            self.reality.imaginary_upg_reqs |= 1 << 24;
        }
    }

    /// The Reality-reset requirement checks for Imaginary Upgrades 11
    /// (REALITY_RESET_AFTER: 1e90 total Relic Shards) and 12
    /// (REALITY_RESET_BEFORE: a level-9000 Glyph with a single weight at 100).
    pub(crate) fn check_imaginary_upgrade_reqs_on_reality_before(&mut self) {
        if self.celestials.effarig.glyph_weights.contains(&100.0)
            && self.gained_glyph_level().actual_level >= 9_000
        {
            self.reality.imaginary_upg_reqs |= 1 << 12;
        }
    }

    pub(crate) fn check_imaginary_upgrade_reqs_on_reality_after(&mut self) {
        if self.celestials.effarig.relic_shards >= 1e90 {
            self.reality.imaginary_upg_reqs |= 1 << 11;
        }
    }

    /// Whether one-time upgrade `id`'s requirement is currently satisfied.
    /// The Lai'tela-adjacent ones (15–21, 25) are observable live; the deep
    /// ones (11–14, 22–24) latch into `imaginaryUpgReqs` at their events.
    pub fn imaginary_upgrade_available(&self, id: u8) -> bool {
        if self.imaginary_upgrade_bought(id) {
            return false;
        }
        if (11..=14).contains(&id) || (22..=24).contains(&id) {
            return self.reality.imaginary_upg_reqs & (1 << id) != 0;
        }
        match id {
            // 15: reach 1e1.5e12 AM with no ID1 this Reality.
            15 => {
                self.requirement_checks.reality_max_id1 == break_infinity::Decimal::ZERO
                    && self.antimatter.exponent() as f64 >= 1.5e12
            }
            // 16: destabilize Lai'tela to ≤ dim 6 (difficulty ≥ 2).
            16 => self.laitela_max_allowed_dimension() <= 6,
            // 17: auto-condense ≥ 20 singularities at once.
            17 => {
                self.singularities_gained() >= 20.0
                    && self.celestials.laitela.dark_energy >= self.singularity_cap()
            }
            // 18: 80000 total galaxies.
            18 => {
                (self.galaxies as u64
                    + self.replicanti.galaxies as u64
                    + self.extra_replicanti_galaxies() as u64
                    + self.dilation.total_tachyon_galaxies as u64)
                    >= 80000
            }
            // 19: tickspeed continuum ≥ 3.85e6 with ≤ 8 studies this Reality.
            19 => {
                self.requirement_checks.reality_max_studies <= 8
                    && self.tickspeed_continuum_value() >= 3.85e6
            }
            // 20: continuum increase ≥ 100% (matter factor ≥ 2).
            20 => self.matter_extra_purchase_factor() >= 2.0,
            // 21: reach 1e7.4e12 AM with continuum disabled all Reality.
            21 => {
                self.requirement_checks.reality_no_continuum
                    && self.antimatter.pos_log10() >= 7.4e12
            }
            // 25: reach Reality in Lai'tela fully destabilized (Pelle unlock).
            25 => {
                self.laitela_is_running()
                    && self.laitela_max_allowed_dimension() == 0
                    && self.dilation_study_bought(6)
            }
            _ => false,
        }
    }

    /// Buy a one-time Imaginary Upgrade (requirement met + iM cost). Seeds the
    /// Dark Matter Dimension for 15–18 (`onPurchase`).
    pub fn buy_imaginary_upgrade(&mut self, id: u8) -> bool {
        if !self.imaginary_upgrade_available(id) {
            return false;
        }
        let cost = self.imaginary_upgrade_cost(id);
        if self.reality.imaginary_machines < Decimal::from_float(cost) {
            return false;
        }
        self.reality.imaginary_machines -= Decimal::from_float(cost);
        self.reality.imaginary_upgrade_bits |= 1u32 << id;
        // Upgrades 15–18 seed the corresponding DMD amount to 1.
        if (15..=18).contains(&id) {
            self.celestials.laitela.dimensions[(id - 15) as usize].amount = Decimal::ONE;
        }
        // Upgrade 22 raises every Glyph Sacrifice total to 1e100.
        if id == 22 {
            for slot in self.reality.glyphs.sac.iter_mut() {
                *slot = 1e100;
            }
        }
        true
    }

    /// Imaginary Upgrade 11's Time-Dimension power:
    /// `1 + log10(log10(totalAntimatter)) / 100`.
    pub(crate) fn imaginary_upgrade_11_td_pow(&self) -> f64 {
        if !self.imaginary_upgrade_bought(11) {
            return 1.0;
        }
        1.0 + self.total_antimatter.pos_log10().max(1.0).log10().max(0.0) / 100.0
    }

    /// Imaginary Upgrade 14's per-purchase-multiplier power (^1.5 on the
    /// AD buy-ten, ID, and TD per-purchase multipliers).
    pub(crate) fn imaginary_upgrade_14_pow(&self) -> f64 {
        if self.imaginary_upgrade_bought(14) {
            1.5
        } else {
            1.0
        }
    }

    /// `DimBoost.imaginaryBoosts`: free Dimension Boosts from Imaginary
    /// Upgrade 12 (`2e4 × total rebuyables`), scaled by IU23's tesseract
    /// effect (`floor(0.25 × Tesseracts.effectiveCount²)`; ×1 while IU23 is
    /// unbought, the original's `effectOrDefault(1)`). Zero inside Ra's Reality.
    pub(crate) fn imaginary_dim_boosts(&self) -> f64 {
        if self.celestials.ra.run || !self.imaginary_upgrade_bought(12) {
            return 0.0;
        }
        let rebuyables: u32 = self.reality.imaginary_rebuyables.iter().sum();
        let iu23 = if self.imaginary_upgrade_bought(23) {
            (0.25 * self.tesseract_effective_count().powi(2)).floor()
        } else {
            1.0
        };
        2e4 * rebuyables as f64 * iu23
    }

    /// `DimBoost.totalBoosts`: purchased boosts plus the imaginary free ones.
    pub(crate) fn total_dim_boosts(&self) -> f64 {
        self.dim_boosts as f64 + self.imaginary_dim_boosts()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_requirements_latch_at_their_events() {
        let mut game = GameState::new();
        game.reality.realities = 1;
        // 11: 1e90 relic shards, latched on the after-reality hook.
        game.celestials.effarig.relic_shards = 1e91;
        game.check_imaginary_upgrade_reqs_on_reality_after();
        assert!(game.imaginary_upgrade_available(11));

        // 24: 13000 galaxies + full inversion inside Ra's Reality (per tick).
        game.celestials.ra.run = true;
        game.requirement_checks.reality_slowest_bh = 1e-300;
        game.galaxies = 13_000;
        game.check_imaginary_upgrade_reqs_on_tick();
        assert!(game.imaginary_upgrade_available(24));
        // The latch persists after the momentary condition passes.
        game.galaxies = 0;
        assert!(game.imaginary_upgrade_available(24));
    }

    #[test]
    fn iu13_multiplies_the_ratcheted_cap() {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game.reality.im_cap = 1000.0;
        assert_eq!(game.imaginary_machine_cap(), 1000.0);
        game.reality.imaginary_upgrade_bits |= 1 << 13;
        game.reality.imaginary_rebuyables[0] = 20; // +1 from rebuyables
                                                   // 1 + 20/20 + 1/2 (one one-time upgrade bought) = 2.5.
        assert_eq!(game.imaginary_machine_cap(), 2500.0);
    }

    #[test]
    fn iu12_grants_free_dim_boosts() {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game.reality.imaginary_upgrade_bits |= 1 << 12;
        game.reality.imaginary_rebuyables[0] = 3;
        assert_eq!(game.imaginary_dim_boosts(), 6e4);
        assert_eq!(game.total_dim_boosts(), 6e4);
        // All 8 dimensions unlock from the free boosts.
        assert_eq!(game.unlocked_dimensions(), 8);
        // Inside Ra's Reality the free boosts vanish.
        game.celestials.ra.run = true;
        assert_eq!(game.imaginary_dim_boosts(), 0.0);
    }

    #[test]
    fn glyph_weights_adjust_the_level_formula() {
        let mut game = GameState::new();
        game.eternity_points = Decimal::new(1.0, 8000);
        game.records.this_reality.max_ep = Decimal::new(1.0, 8000);
        game.records.this_reality.max_replicanti = Decimal::new(1.0, 100);
        game.records.this_reality.max_dt = Decimal::new(1.0, 100);
        let equal = game.gained_glyph_level_exact();
        // A full skew *hurts* here — the preScale punishes zeroed factors
        // (matching the original's design comment) — but it must change the
        // outcome.
        game.celestials.effarig.glyph_weights = [100.0, 0.0, 0.0, 0.0];
        let skewed = game.gained_glyph_level_exact();
        assert!(
            skewed < equal,
            "skewed={skewed} equal={equal} (full skew is nerfed by preScale)"
        );
        // Equal weights are the identity: restoring them restores the level.
        game.celestials.effarig.glyph_weights = [25.0; 4];
        assert!((game.gained_glyph_level_exact() - equal).abs() < 1e-9);
    }

    #[test]
    fn rebuyable_cost_and_effect_scale() {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game.reality.imaginary_machines = Decimal::from_float(1000.0);
        assert_eq!(game.imaginary_rebuyable_cost(3), 1.0); // initial
        assert!(game.buy_imaginary_rebuyable(3));
        assert_eq!(game.imaginary_rebuyable_cost(3), 40.0); // ×40
        assert!((game.imaginary_rebuyable_effect(3) - 0.4).abs() < 1e-9);
    }

    #[test]
    fn imaginary_upgrade_15_unlocks_laitela_and_seeds_dmd() {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game.reality.imaginary_machines = Decimal::from_float(1e10);
        // Requirement: huge AM, no ID1.
        game.antimatter = Decimal::new(1.0, 1_600_000_000_000);
        assert!(game.imaginary_upgrade_available(15));
        assert!(game.buy_imaginary_upgrade(15));
        assert!(game.laitela_unlocked());
        assert_eq!(game.celestials.laitela.dimensions[0].amount, Decimal::ONE);
    }
}
