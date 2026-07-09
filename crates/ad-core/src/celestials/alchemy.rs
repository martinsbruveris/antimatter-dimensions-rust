//! Glyph Alchemy (part of Ra, Feature 7.5) — 21 resources fed by *refining*
//! Glyphs and combined by reactions. See `docs/design/2026-07-07-ra.md` §4.
//! Original: `celestials/ra/alchemy.js` + `secret-formula/celestials/alchemy.js`
//! + the refinement path in `glyphs/glyph-purge-handler.js`.
//!
//! **Scope.** Resource storage, caps, the reaction engine (yield / actual-yield /
//! priority / combine), refinement input from the Glyph sacrifice path, and the
//! effect readers wired at their engine sites. `unpredictability`'s Poisson
//! re-trigger is modelled by its mean (no RNG divergence). The `reality` resource
//! → Reality-Glyph creation, and the `boundless`/`multiversal` effects (tesseract
//! strength / Reality amplification — inert targets) are deferred.

use crate::state::GameState;

/// `ALCHEMY_RESOURCE` ids (constants.js).
pub const POWER: usize = 0;
pub const INFINITY: usize = 1;
pub const TIME: usize = 2;
pub const REPLICATION: usize = 3;
pub const DILATION: usize = 4;
pub const CARDINALITY: usize = 5;
pub const ETERNITY: usize = 6;
pub const DIMENSIONALITY: usize = 7;
pub const INFLATION: usize = 8;
pub const ALTERNATION: usize = 9;
pub const EFFARIG: usize = 10;
pub const SYNERGISM: usize = 11;
pub const MOMENTUM: usize = 12;
pub const DECOHERENCE: usize = 13;
pub const EXPONENTIAL: usize = 14;
pub const FORCE: usize = 15;
pub const UNCOUNTABILITY: usize = 16;
pub const BOUNDLESS: usize = 17;
pub const MULTIVERSAL: usize = 18;
pub const UNPREDICTABILITY: usize = 19;
pub const REALITY: usize = 20;

pub const ALCHEMY_COUNT: usize = 21;

/// The global cap on any resource (`Ra.alchemyResourceCap`).
pub const ALCHEMY_RESOURCE_CAP: f64 = 25000.0;

/// One alchemy resource's saved state (`player.celestials.ra.alchemy[id]`).
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AlchemyResource {
    #[cfg_attr(feature = "serde", serde(default))]
    pub amount: f64,
    /// Whether this resource's reaction is turned on (`reaction`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub reaction: bool,
}

/// Static per-resource config.
struct AlchemyConfig {
    is_base: bool,
    /// Effarig-pet level at which the resource unlocks.
    unlocked_at: u32,
    /// Reagents `(resource_id, cost_per_reaction)`; empty for base resources.
    reagents: &'static [(usize, f64)],
}

const fn base(unlocked_at: u32) -> AlchemyConfig {
    AlchemyConfig {
        is_base: true,
        unlocked_at,
        reagents: &[],
    }
}
const fn adv(unlocked_at: u32, reagents: &'static [(usize, f64)]) -> AlchemyConfig {
    AlchemyConfig {
        is_base: false,
        unlocked_at,
        reagents,
    }
}

/// The 21 resource configs, indexed by id.
static ALCHEMY: [AlchemyConfig; ALCHEMY_COUNT] = [
    base(2),                                                           // 0 power
    base(3),                                                           // 1 infinity
    base(4),                                                           // 2 time
    base(5),                                                           // 3 replication
    base(6),                                                           // 4 dilation
    adv(8, &[(TIME, 8.0), (REPLICATION, 7.0)]),                        // 5 cardinality
    adv(9, &[(TIME, 11.0), (INFINITY, 4.0)]),                          // 6 eternity
    adv(10, &[(POWER, 10.0), (INFINITY, 5.0)]), // 7 dimensionality
    adv(11, &[(POWER, 9.0), (DILATION, 6.0)]),  // 8 inflation
    adv(12, &[(REPLICATION, 5.0), (DILATION, 10.0)]), // 9 alternation
    base(7),                                    // 10 effarig
    adv(13, &[(EFFARIG, 3.0), (REPLICATION, 16.0), (INFINITY, 14.0)]), // 11 synergism
    adv(15, &[(EFFARIG, 11.0), (POWER, 4.0), (TIME, 20.0)]), // 12 momentum
    adv(14, &[(EFFARIG, 13.0), (ALTERNATION, 8.0)]), // 13 decoherence
    adv(18, &[(INFLATION, 18.0), (SYNERGISM, 3.0)]), // 14 exponential
    adv(17, &[(DIMENSIONALITY, 7.0), (MOMENTUM, 8.0)]), // 15 force
    adv(19, &[(INFINITY, 20.0), (EFFARIG, 6.0), (CARDINALITY, 16.0)]), // 16 uncountability
    adv(20, &[(ETERNITY, 13.0), (INFLATION, 18.0)]),                   // 17 boundless
    adv(16, &[(ALTERNATION, 16.0), (DECOHERENCE, 3.0)]),               // 18 multiversal
    adv(
        21,
        &[(EFFARIG, 15.0), (DECOHERENCE, 3.0), (SYNERGISM, 10.0)],
    ), // 19 unpredictability
    adv(
        25,
        &[
            (EXPONENTIAL, 1.0),
            (FORCE, 1.0),
            (UNCOUNTABILITY, 1.0),
            (BOUNDLESS, 1.0),
            (MULTIVERSAL, 1.0),
            (UNPREDICTABILITY, 1.0),
        ],
    ), // 20 reality
];

/// Base resource id → `highest_refinement_value` index (power/infinity/time/
/// replication/dilation/effarig).
pub fn refinement_index(id: usize) -> Option<usize> {
    match id {
        POWER => Some(0),
        INFINITY => Some(1),
        TIME => Some(2),
        REPLICATION => Some(3),
        DILATION => Some(4),
        EFFARIG => Some(5),
        _ => None,
    }
}

impl GameState {
    // --- Availability -----------------------------------------------------------

    /// Whether Glyph Alchemy is unlocked (`Ra.unlocks.unlockGlyphAlchemy`).
    pub fn alchemy_unlocked(&self) -> bool {
        self.ra_unlock_active(crate::celestials::ra::RA_UNLOCK_GLYPH_ALCHEMY)
    }

    /// Whether resource `id` is unlocked (Effarig pet level ≥ its threshold).
    pub fn alchemy_resource_unlocked(&self, id: usize) -> bool {
        self.alchemy_unlocked()
            && self.ra_pet_level(crate::celestials::ra::PET_EFFARIG)
                >= ALCHEMY[id].unlocked_at
    }

    pub fn alchemy_is_base(&self, id: usize) -> bool {
        ALCHEMY[id].is_base
    }

    pub fn alchemy_unlocked_at(&self, id: usize) -> u32 {
        ALCHEMY[id].unlocked_at
    }

    pub fn alchemy_amount(&self, id: usize) -> f64 {
        self.celestials.ra.alchemy[id].amount
    }

    /// The resource cap: base = `min(25000, highestRefinementValue)`; advanced =
    /// `min(reagent caps)`.
    pub fn alchemy_cap(&self, id: usize) -> f64 {
        if ALCHEMY[id].is_base {
            let hi = refinement_index(id)
                .map(|i| self.celestials.ra.highest_refinement_value[i])
                .unwrap_or(0.0);
            ALCHEMY_RESOURCE_CAP.min(hi)
        } else {
            ALCHEMY[id]
                .reagents
                .iter()
                .map(|&(r, _)| self.alchemy_cap(r))
                .fold(f64::INFINITY, f64::min)
        }
    }

    // --- Effect readers (wired at engine sites) ---------------------------------

    /// The raw stored amount used for an effect (0 when Pelle disables alchemy —
    /// Pelle unbuilt, so always the amount).
    fn amt(&self, id: usize) -> f64 {
        self.celestials.ra.alchemy[id].amount
    }

    /// Antimatter-/Infinity-/Time-Dimension multiplier power (power/infinity/time):
    /// `1 + amount/200000`.
    pub(crate) fn alchemy_dimension_power(&self, id: usize) -> f64 {
        if self.alchemy_resource_unlocked(id) {
            1.0 + self.amt(id) / 200000.0
        } else {
            1.0
        }
    }

    /// `replication`: replicanti-speed ×`10^(amount/1000)`.
    pub(crate) fn alchemy_replication_speed(&self) -> f64 {
        if self.alchemy_resource_unlocked(REPLICATION) {
            10f64.powf(self.amt(REPLICATION) / 1000.0)
        } else {
            1.0
        }
    }

    /// `dilation`: DT-production ×`10^(amount/2000)`.
    pub(crate) fn alchemy_dilation_mult(&self) -> f64 {
        if self.alchemy_resource_unlocked(DILATION) {
            10f64.powf(self.amt(DILATION) / 2000.0)
        } else {
            1.0
        }
    }

    /// `dimensionality`: all-Dimensions ×`10^(5·amount)` (log10, for a Decimal).
    pub(crate) fn alchemy_dimensionality_log10(&self) -> f64 {
        if self.alchemy_resource_unlocked(DIMENSIONALITY) {
            5.0 * self.amt(DIMENSIONALITY)
        } else {
            0.0
        }
    }

    /// `effarig`: Relic-Shard gain ×`10^(amount/2500)`.
    pub(crate) fn alchemy_effarig_mult(&self) -> f64 {
        if self.alchemy_resource_unlocked(EFFARIG) {
            10f64.powf(self.amt(EFFARIG) / 2500.0)
        } else {
            1.0
        }
    }

    /// `synergism`: reaction efficiency `0.3 + 1.3·√(amount/25000)`, capped at 1
    /// unless Achievement 175 lifts the cap.
    pub(crate) fn alchemy_synergism(&self) -> f64 {
        if self.alchemy_resource_unlocked(SYNERGISM) {
            let raw = 0.3 + 1.3 * (self.amt(SYNERGISM) / 25000.0).sqrt();
            if self.achievement_unlocked(175) {
                raw
            } else {
                raw.min(1.0)
            }
        } else {
            // Reactions still run before synergism unlocks — efficiency 0.3.
            0.3
        }
    }

    /// `decoherence`: refine-spill fraction `0.15·√(amount/25000)`.
    pub(crate) fn alchemy_decoherence(&self) -> f64 {
        if self.alchemy_resource_unlocked(DECOHERENCE) {
            0.15 * (self.amt(DECOHERENCE) / 25000.0).sqrt()
        } else {
            0.0
        }
    }

    /// `exponential`: IP × `replicanti^(effect)`, effect `10·(amount/10000)²`.
    pub(crate) fn alchemy_exponential(&self) -> f64 {
        if self.alchemy_resource_unlocked(EXPONENTIAL) {
            10.0 * (self.amt(EXPONENTIAL) / 10000.0).powi(2)
        } else {
            0.0
        }
    }

    /// `force`: AD × `RM^(effect)`, effect `5·amount`.
    pub(crate) fn alchemy_force(&self) -> f64 {
        if self.alchemy_resource_unlocked(FORCE) {
            5.0 * self.amt(FORCE)
        } else {
            0.0
        }
    }

    /// `uncountability`: passive Realities & Perk Points per second,
    /// `160·√(amount/25000)`. Deferred: `realities` is a `u32` in our engine, so
    /// fractional passive Reality generation needs an accumulator (out of
    /// frontier — Effarig level 19 + huge alchemy is well past reach).
    #[allow(dead_code)]
    pub(crate) fn alchemy_uncountability(&self) -> f64 {
        if self.alchemy_resource_unlocked(UNCOUNTABILITY) {
            160.0 * (self.amt(UNCOUNTABILITY) / 25000.0).sqrt()
        } else {
            0.0
        }
    }

    /// `inflation`: AD multipliers above this value are raised `^1.05`. The value
    /// is `10^(6e9 − 3e5·amount)` (returned as a log10 threshold).
    pub(crate) fn alchemy_inflation_log10(&self) -> Option<f64> {
        if self.alchemy_resource_unlocked(INFLATION) {
            Some(6e9 - 3e5 * self.amt(INFLATION))
        } else {
            None
        }
    }

    /// `momentum`'s effect cap `1 + amount/125000` (the growing all-dim power's
    /// ceiling); see `Ra.momentumValue`.
    pub(crate) fn alchemy_momentum_cap(&self) -> f64 {
        1.0 + self.amt(MOMENTUM) / 125000.0
    }

    pub(crate) fn alchemy_momentum_unlocked(&self) -> bool {
        self.alchemy_resource_unlocked(MOMENTUM)
    }

    /// `unpredictability`: chance for a reaction to run again `amount/(10714.28+
    /// amount)`.
    fn alchemy_unpredictability(&self) -> f64 {
        if self.alchemy_resource_unlocked(UNPREDICTABILITY) {
            let a = self.amt(UNPREDICTABILITY);
            a / (10714.28 + a)
        } else {
            0.0
        }
    }

    // --- Reactions --------------------------------------------------------------

    /// `AlchemyReaction.reactionYield` for product `id` (0 if not a reaction or a
    /// reagent is locked).
    fn reaction_yield(&self, id: usize) -> f64 {
        let cfg = &ALCHEMY[id];
        if cfg.is_base || !self.alchemy_resource_unlocked(id) {
            return 0.0;
        }
        if cfg
            .reagents
            .iter()
            .any(|&(r, _)| !self.alchemy_resource_unlocked(r))
        {
            return 0.0;
        }
        let min_reagent = cfg
            .reagents
            .iter()
            .map(|&(r, _)| self.amt(r))
            .fold(f64::INFINITY, f64::min);
        let forcing_factor = (min_reagent - self.amt(id)) / 100.0;
        let total_yield = cfg
            .reagents
            .iter()
            .map(|&(r, cost)| self.amt(r) / cost)
            .fold(f64::INFINITY, f64::min);
        total_yield.min(forcing_factor.max(1.0))
    }

    /// `AlchemyReaction.actualYield`: cap the yield so no reagent drops below the
    /// product amount.
    fn reaction_actual_yield(&self, id: usize) -> f64 {
        let base_yield = self.reaction_yield(id);
        if base_yield == 0.0 {
            return 0.0;
        }
        let max_from = self.reaction_base_production(id)
            * base_yield
            * self.reaction_efficiency(id);
        let prod_before = self.amt(id);
        let prod_after = prod_before + max_from;
        let mut capped = base_yield;
        for &(r, cost) in ALCHEMY[id].reagents {
            let reagent_before = self.amt(r);
            let reagent_after = reagent_before - base_yield * cost;
            let diff_before = reagent_before - prod_before;
            let diff_after = reagent_after - prod_after;
            if diff_before != diff_after {
                capped =
                    capped.min(base_yield * diff_before / (diff_before - diff_after));
            }
        }
        capped.max(0.0)
    }

    /// `AlchemyReaction.priority`: the largest remaining reagent after the
    /// reaction (descending priority means fewer downstream reductions).
    fn reaction_priority(&self, id: usize) -> f64 {
        let ay = self.reaction_actual_yield(id);
        // The original seeds this with `Glyphs.levelCap`; it only serves as an
        // upper bound and reagents cap at 25000, so any large value works.
        let mut max_reagent: f64 = 1e9;
        for &(r, cost) in ALCHEMY[id].reagents {
            let after = self.amt(r) - cost * ay;
            max_reagent = max_reagent.min(after);
        }
        max_reagent
    }

    fn reaction_base_production(&self, id: usize) -> f64 {
        if id == REALITY {
            1.0
        } else {
            5.0
        }
    }

    fn reaction_efficiency(&self, id: usize) -> f64 {
        if id == REALITY {
            1.0
        } else {
            self.alchemy_synergism()
        }
    }

    /// `AlchemyReaction.combineReagents` for product `id`: consume reagents and
    /// add product, once (the `unpredictability` re-trigger is folded into the
    /// per-Reality call count via its mean).
    fn combine_reagents(&mut self, id: usize, times: u32) {
        if !self.celestials.ra.alchemy[id].reaction || self.reaction_yield(id) == 0.0 {
            return;
        }
        let cap = self.alchemy_cap(id);
        let base_production = self.reaction_base_production(id);
        for _ in 0..times {
            let yield_ = self.reaction_actual_yield(id);
            let efficiency = self.reaction_efficiency(id);
            for &(r, cost) in ALCHEMY[id].reagents {
                self.celestials.ra.alchemy[r].amount -= yield_ * cost;
            }
            let effective = (yield_ * base_production * efficiency).max(0.05);
            let new_amount =
                (self.celestials.ra.alchemy[id].amount + effective).min(cap);
            self.celestials.ra.alchemy[id].amount = new_amount;
        }
    }

    /// `Ra.applyAlchemyReactions`: run every active reaction once per Reality,
    /// in descending priority order. Gated on Effarig's memories being unlocked
    /// (`Ra.unlocks.effarigUnlock`). `unpredictability` adds `1 + mean` extra
    /// runs (its Poisson mean), modelled deterministically.
    pub(crate) fn apply_alchemy_reactions(&mut self) {
        if !self.ra_unlock_active(crate::celestials::ra::RA_UNLOCK_EFFARIG_MEMORIES) {
            return;
        }
        // Sort reaction ids by descending priority.
        let mut ids: Vec<usize> = (0..ALCHEMY_COUNT)
            .filter(|&id| !ALCHEMY[id].is_base)
            .collect();
        ids.sort_by(|&a, &b| {
            self.reaction_priority(b)
                .partial_cmp(&self.reaction_priority(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // Poisson mean for the re-trigger: p/(1-p) extra events on average.
        let p = self.alchemy_unpredictability();
        let times = if p >= 1.0 {
            2
        } else {
            1 + (p / (1.0 - p)).round() as u32
        };
        for id in ids {
            self.combine_reagents(id, times);
        }
    }

    /// Toggle a reaction on/off (UI command).
    pub fn alchemy_toggle_reaction(&mut self, id: usize) -> bool {
        if ALCHEMY[id].is_base || !self.alchemy_resource_unlocked(id) {
            return false;
        }
        let r = &mut self.celestials.ra.alchemy[id].reaction;
        *r = !*r;
        true
    }

    // --- Refinement (Glyph → resource) -----------------------------------------

    /// `glyphRawRefinementGain`: `0.05 · level³/1e8 · rarity/100`. `rarity` is
    /// the 0–100 percentage from the glyph strength.
    pub(crate) fn glyph_raw_refinement_gain(&self, level: u32, rarity: f64) -> f64 {
        if !self.alchemy_unlocked() {
            return 0.0;
        }
        let glyph_max = (level as f64).powi(3) / 1e8;
        0.05 * glyph_max * (rarity / 100.0)
    }

    /// Refine a generated glyph of `base_resource_id` (its type's resource) with
    /// the given `level`/`rarity`, ratcheting the cap and spilling `decoherence`
    /// to the other base resources. Returns the amount gained (0 if the resource
    /// is locked). Called from the Glyph sacrifice path.
    pub(crate) fn alchemy_refine_glyph(
        &mut self,
        base_resource_id: usize,
        level: u32,
        rarity: f64,
    ) -> f64 {
        if !self.alchemy_resource_unlocked(base_resource_id) {
            return 0.0;
        }
        let raw = self.glyph_raw_refinement_gain(level, rarity);
        let highest = raw / 0.05;
        // Effective cap: max(current cap, cap-after-this-glyph), clamped to 25000.
        let current_cap = self.alchemy_cap(base_resource_id);
        let effective_cap = current_cap.max(highest).min(ALCHEMY_RESOURCE_CAP);
        let until_cap = effective_cap - self.amt(base_resource_id);
        let gain = raw.clamp(0.0, until_cap.max(0.0));
        self.celestials.ra.alchemy[base_resource_id].amount += gain;
        // Decoherence spill to the other base resources.
        let deco = self.alchemy_decoherence();
        if deco > 0.0 {
            let spill = raw * deco;
            for other in [POWER, INFINITY, TIME, REPLICATION, DILATION] {
                if other != base_resource_id {
                    let max_res = self.alchemy_cap(other).max(self.amt(other));
                    let na = (self.amt(other) + spill).min(max_res);
                    self.celestials.ra.alchemy[other].amount = na;
                }
            }
        }
        // Ratchet the refinement value for the base resource.
        if let Some(idx) = refinement_index(base_resource_id) {
            let hi = &mut self.celestials.ra.highest_refinement_value[idx];
            *hi = hi.max(highest);
        }
        gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::celestials::ra::{PET_EFFARIG, RA_UNLOCK_GLYPH_ALCHEMY};

    fn alchemy_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        // Unlock Alchemy + push the Effarig pet high enough for several resources.
        game.celestials.ra.unlock_bits |= 1 << RA_UNLOCK_GLYPH_ALCHEMY;
        game.celestials.ra.unlock_bits |=
            1 << crate::celestials::ra::RA_UNLOCK_EFFARIG_MEMORIES;
        game.celestials.ra.pets[PET_EFFARIG].level = 25;
        game
    }

    #[test]
    fn base_resource_cap_tracks_refinement_value() {
        let mut game = alchemy_game();
        assert_eq!(game.alchemy_cap(POWER), 0.0);
        // Refine a level-1000 100%-rarity power glyph.
        let gain = game.alchemy_refine_glyph(POWER, 1000, 100.0);
        assert!(gain > 0.0);
        // Cap now equals the highest refinement value (< 25000 for level 1000).
        assert!(game.alchemy_cap(POWER) > 0.0);
        assert!(game.alchemy_amount(POWER) > 0.0);
    }

    #[test]
    fn reaction_produces_from_reagents() {
        let mut game = alchemy_game();
        // Fill the eternity reagents (time id2, infinity id1) and enable it.
        game.celestials.ra.alchemy[TIME].amount = 1000.0;
        game.celestials.ra.alchemy[INFINITY].amount = 1000.0;
        game.celestials.ra.highest_refinement_value = [25000.0; 6];
        game.celestials.ra.alchemy[ETERNITY].reaction = true;
        let before = game.alchemy_amount(ETERNITY);
        game.apply_alchemy_reactions();
        assert!(game.alchemy_amount(ETERNITY) > before);
        // Reagents were consumed.
        assert!(game.alchemy_amount(TIME) < 1000.0);
    }

    #[test]
    fn effect_readers_scale_with_amount() {
        let mut game = alchemy_game();
        game.celestials.ra.alchemy[POWER].amount = 200000.0;
        game.celestials.ra.highest_refinement_value[0] = 25000.0;
        // 1 + 200000/200000 = 2.
        assert!((game.alchemy_dimension_power(POWER) - 2.0).abs() < 1e-9);
    }
}
