//! Singularities & Singularity Milestones (part of Lai'tela, Feature 7.6).
//! Dark Energy condenses into Singularities at a cap; Singularities fund a
//! 30-entry milestone tree whose effects feed the Dark Matter Dimensions and a
//! few global systems. See `docs/design/2026-07-07-laitela.md` §2. Original:
//! `celestials/laitela/singularity.js` +
//! `secret-formula/celestials/singularity-milestones.js`.

use crate::state::GameState;

/// Static config for a Singularity Milestone.
pub struct MilestoneConfig {
    pub start: f64,
    /// Geometric step between completions (0 = a one-shot "unique" milestone).
    pub repeat: f64,
    /// Soft-nerf threshold on completions (0 = none).
    pub increase_threshold: f64,
    /// Completion cap (`f64::INFINITY` for unbounded).
    pub limit: f64,
}

const fn m(
    start: f64,
    repeat: f64,
    increase_threshold: f64,
    limit: f64,
) -> MilestoneConfig {
    MilestoneConfig {
        start,
        repeat,
        increase_threshold,
        limit,
    }
}

pub const MILESTONE_COUNT: usize = 30;

// Milestone ids (index into the catalogue), in source order.
pub const CONTINUUM_MULT: usize = 0;
pub const DARK_MATTER_MULT: usize = 1;
pub const DARK_ENERGY_MULT: usize = 2;
pub const DARK_DIM_COST_REDUCTION: usize = 3;
pub const SINGULARITY_MULT: usize = 4;
pub const DARK_DIM_INTERVAL_REDUCTION: usize = 5;
pub const IMPROVED_ASCENSION_DM: usize = 6;
pub const ASCENSION_INTERVAL_SCALING: usize = 7;
pub const AUTO_CONDENSE: usize = 8;
pub const DARK_DIM_AUTOBUYERS: usize = 9;
pub const ASCENSION_AUTOBUYERS: usize = 10;
pub const DARK_AUTOBUYER_SPEED: usize = 11;
pub const REALITY_DE_MULTIPLIER: usize = 12;
pub const IMPROVED_SINGULARITY_CAP: usize = 13;
pub const INTERVAL_COST_SCALING_REDUCTION: usize = 14;
pub const DARK_FROM_TESSERACTS: usize = 15;
pub const MULT_FROM_INFINITIED: usize = 16;
pub const DILATED_TIME_FROM_SINGULARITIES: usize = 17;
pub const DARK_FROM_GLYPH_LEVEL: usize = 18;
pub const GAMESPEED_FROM_SINGULARITIES: usize = 19;
pub const DARK_FROM_THEOREMS: usize = 20;
pub const DIM4_GENERATION: usize = 21;
pub const DARK_FROM_DM4: usize = 22;
pub const ANNIHILATION_AUTOBUYER: usize = 23;
pub const THEOREM_POWER_FROM_SINGULARITIES: usize = 24;
pub const DARK_FROM_GAMESPEED: usize = 25;
pub const GLYPH_LEVEL_FROM_SINGULARITIES: usize = 26;
pub const DARK_FROM_DILATED_TIME: usize = 27;
pub const INFINITIED_POW: usize = 28;
pub const TESSERACT_MULT_FROM_SINGULARITIES: usize = 29;

pub static MILESTONES: [MilestoneConfig; MILESTONE_COUNT] = [
    m(1.0, 125.0, 20.0, f64::INFINITY),  // 0 continuumMult
    m(2.0, 20.0, 30.0, f64::INFINITY),   // 1 darkMatterMult
    m(3.0, 120.0, 10.0, f64::INFINITY),  // 2 darkEnergyMult
    m(4.0, 40.0, 25.0, f64::INFINITY),   // 3 darkDimensionCostReduction
    m(50.0, 3000.0, 5.0, f64::INFINITY), // 4 singularityMult
    m(10.0, 100.0, 20.0, f64::INFINITY), // 5 darkDimensionIntervalReduction
    m(200000.0, 4000.0, 15.0, f64::INFINITY), // 6 improvedAscensionDM
    m(1.2e5, 2400.0, 0.0, 8.0),          // 7 ascensionIntervalScaling
    m(8.0, 80.0, 0.0, 8.0),              // 8 autoCondense
    m(30.0, 170.0, 0.0, 4.0),            // 9 darkDimensionAutobuyers
    m(1e8, 140.0, 0.0, 4.0),             // 10 ascensionAutobuyers
    m(45.0, 650.0, 0.0, 8.0),            // 11 darkAutobuyerSpeed
    m(1500.0, 10000.0, 0.0, 6.0),        // 12 realityDEMultiplier
    m(150.0, 10000.0, 0.0, 4.0),         // 13 improvedSingularityCap
    m(130000.0, 50000.0, 0.0, 5.0),      // 14 intervalCostScalingReduction
    m(80.0, 0.0, 0.0, 1.0),              // 15 darkFromTesseracts
    m(3000.0, 0.0, 0.0, 1.0),            // 16 multFromInfinitied
    m(8e4, 0.0, 0.0, 1.0),               // 17 dilatedTimeFromSingularities
    m(3e6, 0.0, 0.0, 1.0),               // 18 darkFromGlyphLevel
    m(8e7, 0.0, 0.0, 1.0),               // 19 gamespeedFromSingularities
    m(3e9, 0.0, 0.0, 1.0),               // 20 darkFromTheorems
    m(5e11, 0.0, 0.0, 1.0),              // 21 dim4Generation
    m(5e12, 0.0, 0.0, 1.0),              // 22 darkFromDM4
    m(4e18, 0.0, 0.0, 1.0),              // 23 annihilationAutobuyer
    m(3e21, 0.0, 0.0, 1.0),              // 24 theoremPowerFromSingularities
    m(8e22, 0.0, 0.0, 1.0),              // 25 darkFromGamespeed
    m(3e24, 0.0, 0.0, 1.0),              // 26 glyphLevelFromSingularities
    m(8e33, 0.0, 0.0, 1.0),              // 27 darkFromDilatedTime
    m(3e38, 0.0, 0.0, 1.0),              // 28 infinitiedPow
    m(2.5e45, 0.0, 0.0, 1.0),            // 29 tesseractMultFromSingularities
];

impl GameState {
    // --- Singularities ----------------------------------------------------------

    /// `Singularity.cap` = `200·10^capIncreases`.
    pub fn singularity_cap(&self) -> f64 {
        200.0 * 10f64.powi(self.celestials.laitela.singularity_cap_increases as i32)
    }

    /// `Singularity.gainPerCapIncrease` (`improvedSingularityCap`, default 11).
    fn singularity_gain_per_cap_increase(&self) -> f64 {
        self.singularity_milestone_effect_or(IMPROVED_SINGULARITY_CAP, 11.0)
    }

    /// `Singularity.singularitiesGained`.
    pub fn singularities_gained(&self) -> f64 {
        let caps = self.celestials.laitela.singularity_cap_increases as i32;
        let base = self.singularity_gain_per_cap_increase().powi(caps);
        let mult = self.singularity_milestone_effect_or(SINGULARITY_MULT, 1.0);
        // `1 + ImaginaryUpgrade(10).effect` (Entropic Condensing).
        let im10 = 1.0 + self.imaginary_rebuyable_effect(10);
        (base * mult * im10).floor()
    }

    pub fn singularity_cap_reached(&self) -> bool {
        self.celestials.laitela.dark_energy >= self.singularity_cap()
    }

    /// `Singularity.perform`: condense Dark Energy into Singularities.
    pub fn condense_singularity(&mut self) -> bool {
        if !self.singularity_cap_reached() {
            return false;
        }
        // SINGULARITY_RESET_BEFORE achievements (174).
        self.check_singularity_before_achievements();
        let gained = self.singularities_gained();
        self.celestials.laitela.dark_energy = 0.0;
        self.celestials.laitela.singularities += gained;
        // SINGULARITY_RESET_AFTER achievements (177).
        self.check_singularity_after_achievements();
        true
    }

    pub fn singularity_increase_cap(&mut self) {
        if self.celestials.laitela.singularity_cap_increases < 50 {
            self.celestials.laitela.singularity_cap_increases += 1;
        }
    }
    pub fn singularity_decrease_cap(&mut self) {
        if self.celestials.laitela.singularity_cap_increases > 0 {
            self.celestials.laitela.singularity_cap_increases -= 1;
        }
    }

    // --- Milestones -------------------------------------------------------------

    /// Whether milestone `id` is unlocked (`singularities ≥ start`).
    pub fn singularity_milestone_unlocked(&self, id: usize) -> bool {
        self.celestials.laitela.singularities >= MILESTONES[id].start
    }

    /// `SingularityMilestone.completions` — the (nerf-softcapped, limited)
    /// completion count.
    pub fn singularity_milestone_completions(&self, id: usize) -> u32 {
        let cfg = &MILESTONES[id];
        let sing = self.celestials.laitela.singularities;
        if cfg.repeat == 0.0 {
            // Unique: one completion once unlocked.
            return if sing >= cfg.start { 1 } else { 0 };
        }
        if sing < cfg.start {
            return 0;
        }
        let raw = 1.0 + (sing.ln() - cfg.start.ln()) / cfg.repeat.ln();
        // Soft-nerf past the threshold: 1/3 rate.
        let nerfed = if cfg.increase_threshold == 0.0 || raw < cfg.increase_threshold {
            raw
        } else {
            cfg.increase_threshold + (raw - cfg.increase_threshold) / 3.0
        };
        (nerfed.floor().min(cfg.limit)) as u32
    }

    /// The milestone's effect value (using `completions` for repeatables and the
    /// live formula for uniques), or `default` if it isn't unlocked.
    pub(crate) fn singularity_milestone_effect_or(
        &self,
        id: usize,
        default: f64,
    ) -> f64 {
        if !self.singularity_milestone_unlocked(id) {
            return default;
        }
        let c = self.singularity_milestone_completions(id) as f64;
        let sing = self.celestials.laitela.singularities;
        match id {
            CONTINUUM_MULT => c * 0.03,
            DARK_MATTER_MULT => 1.5f64.powf(c),
            DARK_ENERGY_MULT => 2f64.powf(c),
            DARK_DIM_COST_REDUCTION => 0.4f64.powf(c),
            SINGULARITY_MULT => 2f64.powf(c),
            DARK_DIM_INTERVAL_REDUCTION => 0.6f64.powf(c),
            IMPROVED_ASCENSION_DM => 100.0 * c,
            ASCENSION_INTERVAL_SCALING => 1200.0 - 50.0 * c,
            AUTO_CONDENSE => {
                [f64::INFINITY, 1.3, 1.22, 1.15, 1.1, 1.06, 1.03, 1.01, 1.0]
                    [(c as usize).min(8)]
            }
            DARK_DIM_AUTOBUYERS | ASCENSION_AUTOBUYERS | ANNIHILATION_AUTOBUYER => c,
            DARK_AUTOBUYER_SPEED => {
                [30.0, 20.0, 15.0, 10.0, 5.0, 3.0, 2.0, 1.0, 0.0][(c as usize).min(8)]
            }
            REALITY_DE_MULTIPLIER => {
                (1.0 + 0.05 * c).powi(self.celestials.laitela.difficulty_tier as i32)
            }
            IMPROVED_SINGULARITY_CAP => 11.0 + c,
            INTERVAL_COST_SCALING_REDUCTION => 1.0 - 0.03 * c,
            // Uniques.
            DARK_FROM_TESSERACTS => 1.1f64.powf(self.tesseract_effective_count()),
            MULT_FROM_INFINITIED => {
                (self.infinities_total().pos_log10() / 1000.0).max(1.0)
            }
            DILATED_TIME_FROM_SINGULARITIES => 1.0 + (sing.log10() / 100.0).min(0.35),
            DARK_FROM_GLYPH_LEVEL => {
                (((self.records.best_reality.glyph_level as f64 - 15000.0) / 2000.0)
                    .max(1.0))
                .sqrt()
            }
            GAMESPEED_FROM_SINGULARITIES => sing.log10().powi(3).max(1.0),
            DARK_FROM_THEOREMS => {
                (((self.time_theorems.pos_log10() - 1000.0) / 50.0).max(1.0)).sqrt()
            }
            DIM4_GENERATION => self.celestials.laitela.dark_matter_mult,
            DARK_FROM_DM4 => self.celestials.laitela.dimensions[3]
                .amount
                .pow(&break_infinity::Decimal::from_float(0.03))
                .to_f64()
                .max(1.0),
            THEOREM_POWER_FROM_SINGULARITIES => 1.0 + (sing + 1.0).log10() / 70.0,
            DARK_FROM_GAMESPEED => {
                ((self.game_speed_factor() / 1e120).log10() / 40.0).max(1.0)
            }
            GLYPH_LEVEL_FROM_SINGULARITIES => {
                1.0 + ((sing.log10() - 20.0) / 30.0).max(0.0)
            }
            DARK_FROM_DILATED_TIME => 1.6f64.powf(
                (self.dilation.dilated_time + break_infinity::Decimal::ONE).pos_log10()
                    / 1000.0,
            ),
            INFINITIED_POW => 1.0 + (sing + 1.0).log10() / 300.0,
            TESSERACT_MULT_FROM_SINGULARITIES => 1.0 + sing.log10() / 80.0,
            _ => default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn laitela_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game
    }

    #[test]
    fn condense_at_cap_grants_singularities() {
        let mut game = laitela_game();
        assert_eq!(game.singularity_cap(), 200.0);
        game.celestials.laitela.dark_energy = 200.0;
        assert!(game.singularity_cap_reached());
        assert!(game.condense_singularity());
        assert_eq!(game.celestials.laitela.singularities, 1.0);
        assert_eq!(game.celestials.laitela.dark_energy, 0.0);
    }

    #[test]
    fn milestone_completions_scale_with_singularities() {
        let mut game = laitela_game();
        // darkMatterMult: start 2, repeat 20.
        assert_eq!(game.singularity_milestone_completions(DARK_MATTER_MULT), 0);
        game.celestials.laitela.singularities = 2.0;
        assert_eq!(game.singularity_milestone_completions(DARK_MATTER_MULT), 1);
        game.celestials.laitela.singularities = 40.0; // 2·20^1
        assert_eq!(game.singularity_milestone_completions(DARK_MATTER_MULT), 2);
    }

    #[test]
    fn unique_milestone_is_one_shot() {
        let mut game = laitela_game();
        assert_eq!(
            game.singularity_milestone_effect_or(GAMESPEED_FROM_SINGULARITIES, 1.0),
            1.0
        );
        game.celestials.laitela.singularities = 1e8; // start 8e7
        assert_eq!(
            game.singularity_milestone_completions(GAMESPEED_FROM_SINGULARITIES),
            1
        );
    }
}
