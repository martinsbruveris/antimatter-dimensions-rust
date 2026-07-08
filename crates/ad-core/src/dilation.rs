//! Time Dilation (Features 5.1–5.2): a special Eternity run where every
//! multiplier is exponentially compressed, rewarding **Tachyon Particles**
//! that passively produce **Dilated Time**, which buys Dilation Upgrades and
//! **Tachyon Galaxies** (free galaxies for the tickspeed formula).
//!
//! Mirrors `src/core/dilation.js`,
//! `secret-formula/eternity/dilation-upgrades.js`,
//! `secret-formula/eternity/time-studies/dilation-time-studies.js`, and the
//! `updateTachyonGalaxies` / `getTTPerSecond` blocks of `game.js`. The Pelle
//! upgrades (ids 11–15) are out of frontier. See
//! `docs/design/2026-07-04-dilation.md`.

use break_infinity::Decimal;

use crate::state::GameState;

/// Base dilation penalty exponent (`dilatedValueOf`); ×1.05 with the
/// `dilationPenalty` upgrade.
const DILATION_PENALTY: f64 = 0.75;

/// The base Tachyon-Galaxy threshold (1000 DT) and its per-galaxy multiplier
/// base (`getTachyonGalaxyMult` at factor 1: `1 + 3.65·f + 0.35`).
const TG_BASE_THRESHOLD: f64 = 1000.0;

/// TT cost of the dilation studies 1–6 (study 6 is "Unlock Reality").
const DILATION_STUDY_COSTS: [f64; 6] = [5000.0, 1e6, 1e7, 1e8, 1e9, 1.0];

/// The all-time TT requirement to unlock Dilation
/// (`TimeStudy.dilation.totalTimeTheoremRequirement`).
pub(crate) const DILATION_TT_REQUIREMENT: f64 = 12_900.0;

/// Rebuyable Dilation Upgrades (ids 1–3): base DT cost and per-purchase step.
const DIL_REBUYABLE_BASE_COST: [f64; 3] = [1e4, 1e6, 1e7];
const DIL_REBUYABLE_INCREMENT: [f64; 3] = [10.0, 100.0, 20.0];
/// `galaxyThreshold`'s purchase cap (the 38th purchase floors the factor at 0).
const GALAXY_THRESHOLD_CAP: u32 = 38;

/// One-time Dilation Upgrade DT costs, ids 4–10 (`cost` per id − 4).
const DIL_UPGRADE_COSTS: [f64; 7] = [5e6, 1e9, 5e7, 2e12, 1e10, 1e11, 1e15];

/// Dilation state (`player.dilation`). Persists across Eternities; reset only
/// on Reality (out of frontier).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DilationState {
    /// Bought dilation studies (`dilation.studies`, ids 1–5 in frontier). Not
    /// cleared by a time-study respec.
    pub studies: Vec<u8>,
    /// Whether a dilated Eternity is running.
    pub active: bool,
    /// Tachyon Particles.
    pub tachyon_particles: Decimal,
    /// Dilated Time.
    pub dilated_time: Decimal,
    /// DT needed for the next Tachyon Galaxy (stored like the original).
    pub next_threshold: Decimal,
    /// Tachyon Galaxies earned from DT thresholds (monotonic).
    pub base_tachyon_galaxies: u32,
    /// Total Tachyon Galaxies (base × doubling, softcapped past 1000 — can be
    /// fractional there, hence `f64`).
    pub total_tachyon_galaxies: f64,
    /// One-time upgrades (ids 4–10), bit `1 << id`.
    pub upgrades: u32,
    /// Rebuyable purchase counts (ids 1–3: dtGain / galaxyThreshold /
    /// tachyonGain).
    pub rebuyables: [u32; 3],
    /// EP held when TP were last rewarded (`dilation.lastEP`, a display
    /// record).
    pub last_ep: Decimal,
}

impl DilationState {
    pub fn new() -> Self {
        Self {
            studies: Vec::new(),
            active: false,
            tachyon_particles: Decimal::ZERO,
            dilated_time: Decimal::ZERO,
            next_threshold: Decimal::from_float(TG_BASE_THRESHOLD),
            base_tachyon_galaxies: 0,
            total_tachyon_galaxies: 0.0,
            upgrades: 0,
            rebuyables: [0; 3],
            last_ep: Decimal::ZERO,
        }
    }
}

impl Default for DilationState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// Whether Time Dilation is unlocked (`PlayerProgress.dilationUnlocked`).
    pub fn dilation_unlocked(&self) -> bool {
        self.dilation.studies.contains(&1)
    }

    /// Whether dilation study `id` (1–6) is bought.
    pub fn dilation_study_bought(&self, id: u8) -> bool {
        self.dilation.studies.contains(&id)
    }

    /// The TT cost of dilation study `id`.
    pub fn dilation_study_cost(id: u8) -> f64 {
        if (1..=6).contains(&id) {
            DILATION_STUDY_COSTS[(id - 1) as usize]
        } else {
            0.0
        }
    }

    /// Whether dilation study `id` can be bought (`DilationTimeStudyState
    /// .canBeBought`): study 1 needs any of TS231–234, EC11+EC12 fully
    /// completed, and ≥ 12900 all-time TT; studies 2–5 (TD5–8) chain off it;
    /// study 6 (Unlock Reality) needs the TD8 study, `1e4000` peak EP this
    /// reality, and the achievement/first-perk requirement.
    pub fn can_buy_dilation_study(&self, id: u8) -> bool {
        if !(1..=6).contains(&id) || self.dilation_study_bought(id) {
            return false;
        }
        if self.time_theorems < Decimal::from_float(Self::dilation_study_cost(id)) {
            return false;
        }
        match id {
            1 => {
                let ts = [231u16, 232, 233, 234]
                    .iter()
                    .any(|&s| self.time_study_bought(s));
                // The DILR perk (53) waives the EC and total-TT requirements.
                if self.perk_bought(53) {
                    return ts;
                }
                let ecs = self.eternity_challenge_completions(11)
                    >= crate::eternity_challenges::EC_MAX_COMPLETIONS
                    && self.eternity_challenge_completions(12)
                        >= crate::eternity_challenges::EC_MAX_COMPLETIONS;
                let tt =
                    self.max_theorem >= Decimal::from_float(DILATION_TT_REQUIREMENT);
                ts && ecs && tt
            }
            2 => self.dilation_unlocked(),
            6 => {
                self.dilation_study_bought(5)
                    && self.records.this_reality.max_ep.exponent() >= 4000
                    && self.reality_study_achievements_ok()
            }
            _ => self.dilation_study_bought(id - 1),
        }
    }

    /// Buy dilation study `id`. Returns whether it happened.
    pub fn buy_dilation_study(&mut self, id: u8) -> bool {
        if !self.can_buy_dilation_study(id) {
            return false;
        }
        self.time_theorems -= Decimal::from_float(Self::dilation_study_cost(id));
        self.dilation.studies.push(id);
        if id == 1 {
            // The DU1/DU2 perks auto-unlock Dilation Upgrade rows 2/3, and
            // STP grants 10 starting Tachyon Particles.
            if self.perk_bought(42) {
                for upgrade in [4u8, 5, 6] {
                    self.dilation.upgrades |= 1 << upgrade;
                }
            }
            if self.perk_bought(43) {
                for upgrade in [7u8, 8, 9] {
                    self.dilation.upgrades |= 1 << upgrade;
                }
            }
            if self.perk_bought(17) {
                self.dilation.tachyon_particles = self
                    .dilation
                    .tachyon_particles
                    .max(&Decimal::from_float(10.0));
            }
        }
        true
    }

    // --- The dilated run -------------------------------------------------------

    /// Start a dilated Eternity (`startDilatedEternity`): a rewarded-if-at-goal
    /// Eternity, then `dilation.active = true`. Returns whether it happened.
    pub fn start_dilated_eternity(&mut self) -> bool {
        if !self.dilation_unlocked() || self.dilation.active {
            return false;
        }
        // 136: "This is what I have to do to get rid of dilation?".
        self.unlock_achievement(136);
        if self.can_eternity() {
            self.eternity();
        } else {
            // `switchingDilation` forces the reset below the goal (no rewards).
            self.eternity_reset();
        }
        self.dilation.active = true;
        // Pelle's Dilation Strike unlocks the Paradox rift (and resets records).
        self.pelle_trigger_strike(5);
        true
    }

    /// Exit the dilated Eternity (`eternity(…, { switchingDilation: true })`):
    /// rewarded at the goal (TP included), a plain forced reset below it.
    pub fn exit_dilation(&mut self) -> bool {
        if !self.dilation.active {
            return false;
        }
        if self.can_eternity() {
            self.eternity();
        } else {
            self.eternity_reset();
        }
        true
    }

    /// `dilatedValueOf`: compress a multiplier to
    /// `10^(sign(log10) · |log10|^penalty)`.
    pub fn dilated_value_of(&self, value: Decimal) -> Decimal {
        let log10 = value.log10();
        let mut penalty = DILATION_PENALTY;
        // The `dilationPenalty` upgrade (9) raises the exponent ×1.05.
        if self.dilation_upgrade_bought(9) {
            penalty *= 1.05;
        }
        Decimal::pow10(log10.signum() * log10.abs().powf(penalty))
    }

    /// The base TP a dilated run's peak antimatter is worth
    /// (`getBaseTP × tachyonGainMultiplier`); 0 below the Eternity goal.
    pub fn tp_for_antimatter(&self, antimatter: Decimal) -> Decimal {
        let mut base = Decimal::from_float(antimatter.pos_log10() / 400.0)
            .pow(&Decimal::from_float(1.5));
        // Enslaved's Reality nerfs base TP (`getBaseTP`: `^tachyonNerf`).
        if self.celestials.enslaved.run {
            base = base.pow(&Decimal::from_float(
                crate::celestials::enslaved::TACHYON_NERF,
            ));
        }
        base * self.tachyon_gain_multiplier()
    }

    /// `tachyonGainMultiplier`: ×3 per `tachyonGain` purchase, times the
    /// dilation-glyph sacrifice effect and Reality Upgrades 4/8/15.
    pub fn tachyon_gain_multiplier(&self) -> Decimal {
        let mut mult = Decimal::from_float(3.0)
            .pow(&Decimal::from(self.dilation.rebuyables[2] as u64));
        mult *= Decimal::from_float(self.glyph_sac_dilation_effect());
        // RU4 (Superluminal Amplifier): ×3 per purchase.
        mult *= self.reality_rebuyable_effect(4);
        // RU8: ×√(achievement power).
        if self.reality_upgrade_bought(8) {
            mult *= self
                .achievement_power()
                .pow(&Decimal::from_float(0.5))
                .max(&Decimal::ONE);
        }
        // RU15: ×max(√log10(epMult)/9, 1).
        if self.reality_upgrade_bought(15) {
            let factor = (self.ep_mult_effect().pos_log10().sqrt() / 9.0).max(1.0);
            mult *= Decimal::from_float(factor);
        }
        // Achievement 132: TP gain multiplier from Antimatter Galaxies.
        mult *= self.achievement_132_dilation_mult();
        mult
    }

    /// Achievement 132's shared TP/DT multiplier (`1.22 × max(galaxies^0.04, 1)`).
    fn achievement_132_dilation_mult(&self) -> Decimal {
        if self.achievement_unlocked(132) {
            Decimal::from_float(1.22 * (self.galaxies as f64).powf(0.04).max(1.0))
        } else {
            Decimal::ONE
        }
    }

    /// TP a dilated Eternity would grant now, over the current amount
    /// (`getTachyonGain`); 0 when below the Eternity goal.
    pub fn tachyon_gain(&self) -> Decimal {
        if !self.can_eternity() {
            return Decimal::ZERO;
        }
        (self.tp_for_antimatter(self.records.this_eternity.max_am)
            - self.dilation.tachyon_particles)
            .max(&Decimal::ZERO)
    }

    /// `rewardTP()`: bump TP to the run's value (gated on the Eternity goal)
    /// and record the EP held.
    pub(crate) fn reward_tp(&mut self) {
        if !self.can_eternity() {
            return;
        }
        let tp = self.tp_for_antimatter(self.records.this_eternity.max_am);
        self.dilation.tachyon_particles = self.dilation.tachyon_particles.max(&tp);
        self.dilation.last_ep = self.eternity_points;
    }

    // --- Dilated Time & Tachyon Galaxies ----------------------------------------

    /// `getDilationGainPerSecond`: `TP × 2^dtGain × RU1 × dilationDT glyph ×
    /// the replicanti-count glyph term`.
    pub fn dilation_gain_per_second(&self) -> Decimal {
        let mut rate = self.dilation.tachyon_particles
            * Decimal::from_float(2.0)
                .pow(&Decimal::from(self.dilation.rebuyables[0] as u64));
        // RU1 (Temporal Amplifier): ×3 per purchase.
        rate *= self.reality_rebuyable_effect(1);
        // Achievement 132: DT rate multiplier from Antimatter Galaxies.
        rate *= self.achievement_132_dilation_mult();
        // Achievement 137: ×2 Dilated Time while Dilated.
        if self.achievement_unlocked(137) && self.dilation.active {
            rate *= Decimal::from_float(2.0);
        }
        // Ra: Alchemy `dilation`, `continuousTTBoost.dilatedTime`, and
        // `peakGamespeedDT` (all multiply the DT rate at the tachyon base).
        rate *= Decimal::from_float(self.alchemy_dilation_mult());
        rate *= self.ra_tt_boost_dilated_time();
        rate *= Decimal::from_float(self.ra_peak_gamespeed_dt());
        rate *= self.glyph_effect_dilation_dt();
        // replicationdtgain: ×max(log10(replicanti) · effect, 1).
        let dtgain = self.glyph_effect_replicationdtgain();
        if dtgain > 0.0 {
            let factor = (self.replicanti.amount.pos_log10() * dtgain).max(1.0);
            rate *= Decimal::from_float(factor);
        }
        // Enslaved's Reality nerfs the DT rate: `10^(log10(rate+1)^0.85 − 1)`.
        if self.celestials.enslaved.run && rate != Decimal::ZERO {
            let exp = (rate + Decimal::ONE).log10().powf(0.85) - 1.0;
            rate = Decimal::pow10(exp);
        }
        // V's Reality square-roots the DT rate.
        if self.celestials.v.run {
            rate = rate.pow(&Decimal::from_float(0.5));
        }
        rate
    }

    /// The Tachyon-Galaxy threshold multiplier (`getTachyonGalaxyMult`):
    /// `1 + (3.65 · 0.8^galaxyThreshold-purchases + 0.35) × glyph reduction`
    /// (factor floors to 0 at the 38th purchase).
    pub fn tachyon_galaxy_threshold_mult(&self) -> f64 {
        let bought = self.dilation.rebuyables[1];
        let factor = if bought < GALAXY_THRESHOLD_CAP {
            0.8f64.powi(bought as i32)
        } else {
            0.0
        };
        let glyph_reduction = self.glyph_effect_dilation_galaxy_threshold();
        1.0 + (3.65 * factor + 0.35) * glyph_reduction
    }

    /// Advance Dilated Time, Tachyon Galaxies, and the `ttGenerator` upgrade's
    /// TT stream (the game loop's dilation block). Runs every tick — DT
    /// generation is passive, not dilation-run-gated.
    pub(crate) fn tick_dilation(&mut self, dt_ms: f64) {
        if !self.dilation_unlocked() {
            return;
        }
        let dt_s = dt_ms / 1000.0;
        self.dilation.dilated_time +=
            self.dilation_gain_per_second() * Decimal::from_float(dt_s);

        self.update_tachyon_galaxies();

        // Ra boosts to TT generation: `continuousTTBoost.ttGen` (10^(5b)) and
        // `achievementTTMult` (Achievements.power).
        let tt_boost = self.ra_tt_boost_tt_gen() * self.ra_achievement_tt_mult();
        // `ttGenerator` (upgrade 10): TT += TP/20000 per second.
        if self.dilation_upgrade_bought(10) {
            let gain = self.dilation.tachyon_particles / Decimal::from_float(20_000.0)
                * Decimal::from_float(dt_s)
                * tt_boost;
            self.add_time_theorems_decimal(gain);
        }
        // The `dilationTTgen` glyph effect streams TT too — disabled inside
        // Teresa's and Enslaved's Realities (`getTTPerSecond`).
        if !self.celestials.teresa.run && !self.celestials.enslaved.run {
            let glyph_ttgen = self.glyph_effect_dilation_ttgen();
            if glyph_ttgen > 0.0 {
                self.add_time_theorems_decimal(
                    Decimal::from_float(glyph_ttgen * dt_s) * tt_boost,
                );
            }
        }
    }

    /// `updateTachyonGalaxies()`: bank threshold crossings into base TGs
    /// (monotonic), refresh the next threshold, and derive the total with the
    /// `doubleGalaxies` doubling (softcapped past 1000).
    pub(crate) fn update_tachyon_galaxies(&mut self) {
        let threshold_mult = self.tachyon_galaxy_threshold_mult();
        let dt = self.dilation.dilated_time;
        if dt >= Decimal::from_float(TG_BASE_THRESHOLD) {
            let crossings = 1
                + ((dt / Decimal::from_float(TG_BASE_THRESHOLD)).log10()
                    / threshold_mult.log10())
                .floor() as i64;
            self.dilation.base_tachyon_galaxies = self
                .dilation
                .base_tachyon_galaxies
                .max(crossings.max(0) as u32);
        }
        self.dilation.next_threshold = Decimal::from_float(TG_BASE_THRESHOLD)
            * Decimal::from_float(threshold_mult)
                .pow(&Decimal::from(self.dilation.base_tachyon_galaxies as u64));

        let double = if self.dilation_upgrade_bought(4) {
            2.0
        } else {
            1.0
        };
        let doubled = self.dilation.base_tachyon_galaxies as f64 * double;
        self.dilation.total_tachyon_galaxies =
            doubled.min(1000.0) + (doubled - 1000.0).max(0.0) / double;
    }

    // --- Dilation Upgrades (Feature 5.2) ----------------------------------------

    /// Whether one-time Dilation Upgrade `id` (4–10) is owned.
    pub fn dilation_upgrade_bought(&self, id: u8) -> bool {
        self.dilation.upgrades & (1 << id) != 0
    }

    /// Purchase count of rebuyable `id` (1–3).
    pub fn dilation_rebuyable_count(&self, id: u8) -> u32 {
        if (1..=3).contains(&id) {
            self.dilation.rebuyables[(id - 1) as usize]
        } else {
            0
        }
    }

    /// The DT cost of Dilation Upgrade `id` (rebuyables scale geometrically).
    pub fn dilation_upgrade_cost(&self, id: u8) -> Decimal {
        match id {
            1..=3 => {
                let i = (id - 1) as usize;
                Decimal::from_float(DIL_REBUYABLE_BASE_COST[i])
                    * Decimal::from_float(DIL_REBUYABLE_INCREMENT[i])
                        .pow(&Decimal::from(self.dilation.rebuyables[i] as u64))
            }
            4..=10 => Decimal::from_float(DIL_UPGRADE_COSTS[(id - 4) as usize]),
            _ => Decimal::MAX_VALUE,
        }
    }

    /// Whether `galaxyThreshold` (2) is at its 38-purchase cap.
    pub fn dilation_rebuyable_capped(&self, id: u8) -> bool {
        id == 2 && self.dilation.rebuyables[1] >= GALAXY_THRESHOLD_CAP
    }

    /// Whether Dilation Upgrade `id` can be bought now.
    pub fn can_buy_dilation_upgrade(&self, id: u8) -> bool {
        if !self.dilation_unlocked() || !(1..=10).contains(&id) {
            return false;
        }
        if (4..=10).contains(&id) && self.dilation_upgrade_bought(id) {
            return false;
        }
        if self.dilation_rebuyable_capped(id) {
            return false;
        }
        self.dilation.dilated_time >= self.dilation_upgrade_cost(id)
    }

    /// Buy Dilation Upgrade `id` (`buyDilationUpgrade`, single purchase).
    /// `galaxyThreshold` (2) resets DT and the Tachyon Galaxies.
    pub fn buy_dilation_upgrade(&mut self, id: u8) -> bool {
        if !self.can_buy_dilation_upgrade(id) {
            return false;
        }
        let cost = self.dilation_upgrade_cost(id);
        self.dilation.dilated_time -= cost;
        match id {
            1..=3 => {
                self.dilation.rebuyables[(id - 1) as usize] += 1;
                if id == 2 {
                    // The TGR perk (52) keeps Dilated Time on the reset.
                    if !self.perk_bought(52) {
                        self.dilation.dilated_time = Decimal::ZERO;
                    }
                    self.dilation.next_threshold =
                        Decimal::from_float(TG_BASE_THRESHOLD);
                    self.dilation.base_tachyon_galaxies = 0;
                    self.dilation.total_tachyon_galaxies = 0.0;
                }
                if id == 3 {
                    // The TP1–TP4 perks retroactively multiply TP on each
                    // ×3-TP purchase (`Effects.max` of the owned perks).
                    let factor = [(83u8, 3.0), (82, 2.5), (81, 2.0), (80, 1.5)]
                        .iter()
                        .find(|&&(perk, _)| self.perk_bought(perk))
                        .map(|&(_, f)| f)
                        .unwrap_or(1.0);
                    self.dilation.tachyon_particles *= Decimal::from_float(factor);
                }
            }
            _ => {
                self.dilation.upgrades |= 1 << id;
                // The doubling applies immediately (recomputed next tick too).
                if id == 4 {
                    self.update_tachyon_galaxies();
                }
            }
        }
        true
    }

    /// The `tdMultReplicanti` upgrade's TD multiplier:
    /// `10^(0.1·log10(replicantiMult))`, the excess past 1e9000 halved.
    pub(crate) fn dilation_td_mult_replicanti(&self) -> Decimal {
        let mut rep10 = self.replicanti_mult().pos_log10() * 0.1;
        if rep10 > 9000.0 {
            rep10 = 9000.0 + 0.5 * (rep10 - 9000.0);
        }
        Decimal::pow10(rep10)
    }

    /// Grant a `Decimal` amount of Time Theorems (fractional TT from the
    /// `ttGenerator` stream), updating the all-time max.
    pub(crate) fn add_time_theorems_decimal(&mut self, amount: Decimal) {
        self.time_theorems += amount;
        self.max_theorem = self.max_theorem.max(&self.time_theorems);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ETERNITY_GOAL;

    /// A state with dilation unlocked.
    fn dilation_game() -> GameState {
        let mut game = GameState::new();
        game.dilation.studies.push(1);
        game
    }

    #[test]
    fn dilation_study_gating() {
        let mut game = GameState::new();
        game.time_theorems = Decimal::from_float(20_000.0);
        game.max_theorem = Decimal::from_float(20_000.0);
        game.studies = vec![231];
        // EC11/EC12 not fully completed yet.
        assert!(!game.can_buy_dilation_study(1));
        game.eternity_challenges[10] = 5;
        game.eternity_challenges[11] = 5;
        assert!(game.buy_dilation_study(1));
        assert!(game.dilation_unlocked());
        assert_eq!(game.time_theorems, Decimal::from_float(15_000.0));
        // TD studies chain: 2 then 3, no skipping.
        game.time_theorems = Decimal::new(1.0, 10);
        assert!(!game.can_buy_dilation_study(3));
        assert!(game.buy_dilation_study(2));
        assert!(game.td_is_unlocked(4)); // TD5
        assert!(!game.td_is_unlocked(5));
        assert!(game.buy_dilation_study(3));
        assert!(game.td_is_unlocked(5)); // TD6
    }

    #[test]
    fn dilated_run_compresses_multipliers() {
        let mut game = dilation_game();
        assert!(game.start_dilated_eternity());
        assert!(game.dilation.active);

        // 1e100 compresses to 10^(100^0.75) ≈ 1e31.6.
        let dilated = game.dilated_value_of(Decimal::new(1.0, 100));
        assert!((dilated.log10() - 100f64.powf(0.75)).abs() < 1e-9);

        // The AD multiplier is far below its undilated value.
        game.dimensions[0].bought = 100;
        let mult = game.dimension_multiplier(0);
        game.dilation.active = false;
        let undilated = game.dimension_multiplier(0);
        assert!(mult < undilated);
    }

    #[test]
    fn exiting_at_goal_rewards_tp() {
        let mut game = dilation_game();
        game.start_dilated_eternity();
        game.records.this_eternity.max_ip = ETERNITY_GOAL;
        game.records.this_eternity.max_am = Decimal::new(1.0, 4000);
        assert!(game.exit_dilation());
        assert!(!game.dilation.active);
        // TP = (4000/400)^1.5 = 31.6...
        let expected = 10f64.powf(1.5);
        assert!((game.dilation.tachyon_particles.to_f64() - expected).abs() < 1e-9);
        // EP was also awarded (a rewarded eternity).
        assert!(game.eternity_points > Decimal::ZERO);
    }

    #[test]
    fn exiting_below_goal_gives_nothing() {
        let mut game = dilation_game();
        game.start_dilated_eternity();
        game.records.this_eternity.max_am = Decimal::new(1.0, 4000);
        assert!(game.exit_dilation());
        assert_eq!(game.dilation.tachyon_particles, Decimal::ZERO);
    }

    #[test]
    fn tp_generate_dt_and_tachyon_galaxies() {
        let mut game = dilation_game();
        game.dilation.tachyon_particles = Decimal::from_float(100.0);
        game.tick_dilation(10_000.0);
        assert_eq!(game.dilation.dilated_time, Decimal::from_float(1000.0));
        // 1000 DT = the first Tachyon Galaxy.
        assert_eq!(game.dilation.base_tachyon_galaxies, 1);
        assert_eq!(game.dilation.total_tachyon_galaxies, 1.0);
        // Threshold mult 5 → next at 5000.
        assert_eq!(game.dilation.next_threshold, Decimal::from_float(5000.0));
        // TGs feed the tickspeed galaxy count.
        assert_eq!(game.effective_galaxies(), 1);
    }

    #[test]
    fn dt_gain_upgrade_doubles_rate() {
        let mut game = dilation_game();
        game.dilation.tachyon_particles = Decimal::ONE;
        game.dilation.dilated_time = Decimal::from_float(2e4);
        assert!(game.buy_dilation_upgrade(1));
        assert_eq!(game.dilation.rebuyables[0], 1);
        assert_eq!(game.dilation_gain_per_second(), Decimal::from_float(2.0));
        // Next cost ×10.
        assert_eq!(game.dilation_upgrade_cost(1), Decimal::from_float(1e5));
    }

    #[test]
    fn galaxy_threshold_upgrade_resets_dt_and_tgs() {
        let mut game = dilation_game();
        game.dilation.dilated_time = Decimal::from_float(1e7);
        game.update_tachyon_galaxies();
        assert!(game.dilation.base_tachyon_galaxies > 0);
        assert!(game.buy_dilation_upgrade(2));
        assert_eq!(game.dilation.dilated_time, Decimal::ZERO);
        assert_eq!(game.dilation.base_tachyon_galaxies, 0);
        assert_eq!(game.dilation.total_tachyon_galaxies, 0.0);
        // The threshold multiplier dropped: 1 + 3.65×0.8 + 0.35 = 4.27.
        assert!((game.tachyon_galaxy_threshold_mult() - 4.27).abs() < 1e-12);
    }

    #[test]
    fn double_galaxies_upgrade() {
        let mut game = dilation_game();
        game.dilation.dilated_time = Decimal::from_float(1e7);
        game.update_tachyon_galaxies();
        let base = game.dilation.base_tachyon_galaxies;
        assert!(game.buy_dilation_upgrade(4));
        assert_eq!(game.dilation.total_tachyon_galaxies, base as f64 * 2.0);
    }

    #[test]
    fn tt_generator_streams_theorems() {
        let mut game = dilation_game();
        game.dilation.tachyon_particles = Decimal::from_float(20_000.0);
        game.dilation.dilated_time = Decimal::new(1.0, 16);
        assert!(game.buy_dilation_upgrade(10));
        game.tick_dilation(1000.0);
        // 20000 TP / 20000 = 1 TT per second.
        assert_eq!(game.time_theorems, Decimal::ONE);
    }

    #[test]
    fn dilation_survives_eternity_but_run_ends() {
        let mut game = dilation_game();
        game.dilation.tachyon_particles = Decimal::from_float(5.0);
        game.dilation.dilated_time = Decimal::from_float(123.0);
        game.start_dilated_eternity();
        game.records.this_eternity.max_ip = ETERNITY_GOAL;
        assert!(game.eternity());
        // The run ends; TP/DT persist.
        assert!(!game.dilation.active);
        assert_eq!(game.dilation.dilated_time, Decimal::from_float(123.0));
        assert!(game.dilation.tachyon_particles >= Decimal::from_float(5.0));
    }
}
