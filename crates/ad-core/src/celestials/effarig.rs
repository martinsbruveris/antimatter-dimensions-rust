//! Effarig (Feature 7.2) — a multi-stage Reality with a Relic-Shard economy.
//! See `docs/design/2026-07-06-celestials.md` §2. Original:
//! `celestials/effarig.js` + `secret-formula/celestials/effarig.js`.
//!
//! **Scope.** The run experience is ported in full: the three stages
//! (Infinity → Eternity → Reality) with their prestige-hook unlocks + exits,
//! the dilation-like nerfs (AD multiplier, tickspeed, glyph-level cap), and the
//! infinity-stage IP handling. Relic Shards are gained on each Reality. The
//! persistent rewards are wired: the Infinity stage's Replicanti-cap raise +
//! `bonusRG` (`replicanti.rs`, dead while Doomed via `isDisabled("effarig")`)
//! and the Eternity stage's Eternities-generate-Infinities term
//! (`passive_prestige_gen`). Glyph set saves remain a QoL cut.

use crate::state::GameState;
use break_infinity::Decimal;

/// Effarig unlocks (`secret-formula/celestials/effarig.js`), bought with Relic
/// Shards (adjuster/glyphFilter/setSaves/run) or earned by completing a run
/// stage (infinity/eternity/reality). `id` is the save bit.
pub const EFFARIG_UNLOCK_ADJUSTER: u8 = 0;
pub const EFFARIG_UNLOCK_GLYPH_FILTER: u8 = 1;
pub const EFFARIG_UNLOCK_SET_SAVES: u8 = 2;
pub const EFFARIG_UNLOCK_RUN: u8 = 3;
pub const EFFARIG_UNLOCK_INFINITY: u8 = 4;
pub const EFFARIG_UNLOCK_ETERNITY: u8 = 5;
pub const EFFARIG_UNLOCK_REALITY: u8 = 6;

/// The relic-shard costs of the four purchasable unlocks, `(id, cost)`.
pub const EFFARIG_UNLOCK_COSTS: [(u8, f64); 4] = [
    (EFFARIG_UNLOCK_ADJUSTER, 1e7),
    (EFFARIG_UNLOCK_GLYPH_FILTER, 2e8),
    (EFFARIG_UNLOCK_SET_SAVES, 3e9),
    (EFFARIG_UNLOCK_RUN, 5e11),
];

/// Effarig's Reality stages (`EFFARIG_STAGES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffarigStage {
    Infinity = 1,
    Eternity = 2,
    Reality = 3,
    Completed = 4,
}

/// `player.celestials.effarig`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EffarigState {
    /// Relic Shards (`relicShards`), the Effarig currency.
    #[cfg_attr(feature = "serde", serde(default))]
    pub relic_shards: f64,
    /// Unlock bits (`unlockBits`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub unlock_bits: u32,
    /// Whether Effarig's Reality is running (`run`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub run: bool,
    /// Glyph-level factor weights (`glyphWeights`: ep/repl/dt/eternities,
    /// summing 100; the shard "adjuster" unlock exposes them).
    #[cfg_attr(feature = "serde", serde(default = "default_glyph_weights"))]
    pub glyph_weights: [f64; 4],
}

/// serde/struct default: all four weights equal (the identity adjustment).
fn default_glyph_weights() -> [f64; 4] {
    [25.0; 4]
}

impl Default for EffarigState {
    fn default() -> Self {
        Self {
            relic_shards: 0.0,
            unlock_bits: 0,
            run: false,
            glyph_weights: default_glyph_weights(),
        }
    }
}

impl EffarigState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether unlock `id` is owned.
    pub fn unlock_bought(&self, id: u8) -> bool {
        self.unlock_bits & (1u32 << id) != 0
    }
}

impl GameState {
    // --- Unlocks / stages -------------------------------------------------------

    /// Whether Effarig's Reality is unlocked (the `run` unlock, id 3).
    pub fn effarig_run_unlocked(&self) -> bool {
        self.celestials.effarig.unlock_bought(EFFARIG_UNLOCK_RUN)
    }

    /// Whether Effarig itself is available — Teresa's `effarig` threshold is met
    /// (`Effarig.isUnlocked` via `TeresaUnlocks.effarig`).
    pub fn effarig_unlocked(&self) -> bool {
        self.teresa_effarig_unlocked()
    }

    /// `EffarigUnlock.reality.isUnlocked`: completing Effarig's Reality stage
    /// unlocks the Effarig glyph type.
    pub fn effarig_glyphs_unlocked(&self) -> bool {
        self.celestials
            .effarig
            .unlock_bought(EFFARIG_UNLOCK_REALITY)
    }

    /// `EffarigUnlock.infinity.canBeApplied`: the Infinity-stage reward
    /// (Replicanti cap / max-RG from Infinities) — its effect dies while
    /// Doomed (`Pelle.isDisabled("effarig")`).
    pub fn effarig_infinity_unlock_applies(&self) -> bool {
        self.celestials
            .effarig
            .unlock_bought(EFFARIG_UNLOCK_INFINITY)
            && !self.is_doomed()
    }

    /// `EffarigUnlock.eternity.isUnlocked`: completing Effarig's Eternity
    /// stage (Eternities generate Infinities; IP uncapped in the run).
    pub fn effarig_eternity_unlocked(&self) -> bool {
        self.celestials
            .effarig
            .unlock_bought(EFFARIG_UNLOCK_ETERNITY)
    }

    /// `Effarig.currentStage`: the lowest stage whose unlock bit is unset.
    pub fn effarig_current_stage(&self) -> EffarigStage {
        let e = &self.celestials.effarig;
        if !e.unlock_bought(EFFARIG_UNLOCK_INFINITY) {
            EffarigStage::Infinity
        } else if !e.unlock_bought(EFFARIG_UNLOCK_ETERNITY) {
            EffarigStage::Eternity
        } else if !e.unlock_bought(EFFARIG_UNLOCK_REALITY) {
            EffarigStage::Reality
        } else {
            EffarigStage::Completed
        }
    }

    /// Buy a Relic-Shard unlock by id (adjuster/glyphFilter/setSaves/run).
    /// Returns whether it happened.
    pub fn effarig_buy_unlock(&mut self, id: u8) -> bool {
        let Some(&(_, cost)) = EFFARIG_UNLOCK_COSTS.iter().find(|(i, _)| *i == id)
        else {
            return false;
        };
        if self.celestials.effarig.unlock_bought(id)
            || self.celestials.effarig.relic_shards < cost
        {
            return false;
        }
        self.celestials.effarig.relic_shards -= cost;
        self.celestials.effarig.unlock_bits |= 1u32 << id;
        true
    }

    // --- Run nerfs (dilation-like) ----------------------------------------------

    /// `Effarig.nerfFactor(power)`: `3·(1 − c/(c + √(pLog10(power))))`, where the
    /// stage constant `c` is 1500 / 29.29 / 25 for Infinity / Eternity / Reality.
    pub(crate) fn effarig_nerf_factor(&self, power: Decimal) -> f64 {
        let c = match self.effarig_current_stage() {
            EffarigStage::Infinity => 1500.0,
            EffarigStage::Eternity => 29.29,
            _ => 25.0,
        };
        3.0 * (1.0 - c / (c + power.pos_log10().sqrt()))
    }

    /// `Effarig.multDilation`: `0.25 + 0.25·nerfFactor(infinityPower)`.
    fn effarig_mult_dilation(&self) -> f64 {
        0.25 + 0.25 * self.effarig_nerf_factor(self.infinity_power)
    }

    /// `Effarig.tickDilation`: `0.7 + 0.1·nerfFactor(timeShards)`.
    fn effarig_tick_dilation(&self) -> f64 {
        0.7 + 0.1 * self.effarig_nerf_factor(self.time_shards)
    }

    /// `Effarig.multiplier(mult)`: `10^(pLog10(mult)^multDilation)` — compresses
    /// the final AD multiplier (the run's replacement for the dilation/V stage).
    pub(crate) fn effarig_multiplier(&self, mult: Decimal) -> Decimal {
        let base = mult.pos_log10();
        Decimal::pow10(base.powf(self.effarig_mult_dilation()))
    }

    /// `Effarig.tickspeed`: compress the tickspeed interval `base` via
    /// `10^(-((3 + log10(1/base))^tickDilation))`.
    pub(crate) fn effarig_tickspeed(&self, base: Decimal) -> Decimal {
        // `base.reciprocal().log10() = -log10(base)`.
        let b = (3.0 - base.log10()).max(0.0);
        Decimal::pow10(-(b.powf(self.effarig_tick_dilation())))
    }

    /// `Effarig.glyphLevelCap`: 100 / 1500 / 2000 by current stage.
    pub fn effarig_glyph_level_cap(&self) -> u32 {
        match self.effarig_current_stage() {
            EffarigStage::Infinity => 100,
            EffarigStage::Eternity => 1500,
            _ => 2000,
        }
    }

    // --- Relic Shards -----------------------------------------------------------

    /// `Effarig.glyphEffectAmount`: the count of distinct effect bits across the
    /// equipped (non-companion) glyphs. Our frontier only generates basic
    /// types, so the generated/non-generated split collapses to one popcount.
    fn effarig_glyph_effect_amount(&self) -> u32 {
        let mask = self
            .active_glyphs_without_companion()
            .iter()
            .fold(0u32, |acc, g| acc | g.effects);
        mask.count_ones()
    }

    /// `Effarig.shardsGained`: `floor((EP.exponent/7500)^glyphEffectAmount)`
    /// (the Alchemy factor is out of frontier → 1). Gained on each Reality once
    /// Teresa's `effarig` unlock is owned.
    pub fn effarig_shards_gained(&self) -> f64 {
        if !self.effarig_unlocked() {
            return 0.0;
        }
        let exp = self.eternity_points.exponent() as f64;
        let amount = self.effarig_glyph_effect_amount();
        // Ra's Alchemy `effarig` resource multiplies Relic-Shard gain.
        (exp / 7500.0).powi(amount as i32).floor() * self.alchemy_effarig_mult()
    }

    /// `giveRealityRewards`: add the run's Relic Shards on a rewarded Reality.
    pub(crate) fn effarig_gain_relic_shards(&mut self, multiplier: f64) {
        if !self.effarig_unlocked() {
            return;
        }
        // An amplified Reality multiplies the shard payout (`gainedShards ×
        // multiplier` in `giveRealityRewards`).
        self.celestials.effarig.relic_shards +=
            self.effarig_shards_gained() * multiplier;
    }

    // --- Stage completion hooks -------------------------------------------------

    /// `bigCrunchCheckUnlocks`: completing the Infinity stage inside the run
    /// unlocks it and forces a reward-free Reality exit.
    pub(crate) fn effarig_on_big_crunch(&mut self) {
        if self.celestials.effarig.run
            && !self
                .celestials
                .effarig
                .unlock_bought(EFFARIG_UNLOCK_INFINITY)
        {
            self.celestials.effarig.unlock_bits |= 1u32 << EFFARIG_UNLOCK_INFINITY;
            self.reset_reality();
        }
    }

    /// The Eternity hook: completing the Eternity stage inside the run unlocks
    /// it and forces a reward-free Reality exit.
    pub(crate) fn effarig_on_eternity(&mut self) {
        if self.celestials.effarig.run
            && !self
                .celestials
                .effarig
                .unlock_bought(EFFARIG_UNLOCK_ETERNITY)
        {
            self.celestials.effarig.unlock_bits |= 1u32 << EFFARIG_UNLOCK_ETERNITY;
            self.reset_reality();
        }
    }

    /// `giveRealityRewards`: a rewarded Reality inside the run unlocks the
    /// Reality stage (the Effarig glyph type; the type generation itself is
    /// deferred).
    pub(crate) fn effarig_complete_stage(&mut self) {
        if !self
            .celestials
            .effarig
            .unlock_bought(EFFARIG_UNLOCK_REALITY)
        {
            self.celestials.effarig.unlock_bits |= 1u32 << EFFARIG_UNLOCK_REALITY;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effarig_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        // Teresa's effarig threshold (unlocks Effarig).
        game.celestials.teresa.unlock_bits |= 1 << 3;
        game
    }

    #[test]
    fn infinity_unlock_raises_the_replicanti_cap() {
        let mut game = effarig_game();
        assert_eq!(game.replicanti_cap(), Decimal::NUMBER_MAX_VALUE);
        assert_eq!(game.effarig_bonus_rg(), 0);
        game.celestials.effarig.unlock_bits |= 1 << EFFARIG_UNLOCK_INFINITY;
        game.infinities = Decimal::new(1.0, 10);
        // cap = (1e10)^30 × 1.8e308 → bonusRG = floor(log10(cap)/308.25 − 1).
        let cap = game.replicanti_cap();
        assert!(cap > Decimal::NUMBER_MAX_VALUE);
        assert_eq!(game.effarig_bonus_rg(), 0); // 608/308 − 1 < 1
        game.infinities = Decimal::new(1.0, 40);
        assert_eq!(game.effarig_bonus_rg(), 3); // (1200+308)/308.25 − 1 → 3
                                                // Doomed kills the effect (`Pelle.isDisabled("effarig")`).
        game.celestials.pelle.doomed = true;
        assert_eq!(game.replicanti_cap(), Decimal::NUMBER_MAX_VALUE);
    }

    #[test]
    fn eternity_unlock_generates_infinities_from_eternities() {
        let mut game = effarig_game();
        game.celestials.effarig.unlock_bits |= 1 << EFFARIG_UNLOCK_ETERNITY;
        game.eternities = Decimal::from_float(100.0);
        game.passive_prestige_gen(1000.0);
        // gainedInfinities (1) × eternities (100) × 1 s.
        assert_eq!(game.infinities, Decimal::from_float(100.0));
    }

    #[test]
    fn buying_unlocks_spends_relic_shards() {
        let mut game = effarig_game();
        game.celestials.effarig.relic_shards = 1e8;
        assert!(game.effarig_buy_unlock(EFFARIG_UNLOCK_ADJUSTER)); // 1e7
        assert!((game.celestials.effarig.relic_shards - 9e7).abs() < 1.0);
        // Can't afford the 5e11 run unlock yet.
        assert!(!game.effarig_buy_unlock(EFFARIG_UNLOCK_RUN));
    }

    #[test]
    fn stage_advances_with_unlock_bits() {
        let mut game = effarig_game();
        assert_eq!(game.effarig_current_stage(), EffarigStage::Infinity);
        game.celestials.effarig.unlock_bits |= 1 << EFFARIG_UNLOCK_INFINITY;
        assert_eq!(game.effarig_current_stage(), EffarigStage::Eternity);
        game.celestials.effarig.unlock_bits |= 1 << EFFARIG_UNLOCK_ETERNITY;
        assert_eq!(game.effarig_current_stage(), EffarigStage::Reality);
        game.celestials.effarig.unlock_bits |= 1 << EFFARIG_UNLOCK_REALITY;
        assert_eq!(game.effarig_current_stage(), EffarigStage::Completed);
    }

    #[test]
    fn glyph_level_cap_by_stage() {
        let mut game = effarig_game();
        assert_eq!(game.effarig_glyph_level_cap(), 100);
        game.celestials.effarig.unlock_bits |= 1 << EFFARIG_UNLOCK_INFINITY;
        assert_eq!(game.effarig_glyph_level_cap(), 1500);
        game.celestials.effarig.unlock_bits |= 1 << EFFARIG_UNLOCK_ETERNITY;
        assert_eq!(game.effarig_glyph_level_cap(), 2000);
    }

    #[test]
    fn multiplier_compresses_large_values() {
        let mut game = effarig_game();
        game.celestials.effarig.run = true;
        let big = Decimal::new(1.0, 1000);
        let compressed = game.effarig_multiplier(big);
        // The compression brings 1e1000 well below itself.
        assert!(compressed < big);
        assert!(compressed > Decimal::ONE);
    }

    #[test]
    fn infinity_stage_completion_unlocks_and_exits() {
        let mut game = effarig_game();
        game.celestials.effarig.run = true;
        game.effarig_on_big_crunch();
        assert!(game
            .celestials
            .effarig
            .unlock_bought(EFFARIG_UNLOCK_INFINITY));
        // The forced reset exited the run.
        assert!(!game.celestials.effarig.run);
    }
}
