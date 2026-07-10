//! Eternity Challenges (Feature 4.5): 12 challenges with up to 5 completions
//! each, entered through EC study slots in the Time Studies tree.
//!
//! Mirrors `src/core/eternity-challenge.js`,
//! `secret-formula/challenges/eternity-challenges.js`, and
//! `secret-formula/eternity/time-studies/ec-time-studies.js`. The run flow:
//! buy the EC's unlock study (TT cost + a secondary resource requirement,
//! waived on re-unlock via `requirementBits`), start the challenge (a forced
//! Eternity-style reset), reach the scaled IP goal, and Eternity to bank a
//! completion (which auto-respecs the tree). EC4/EC12 carry restrictions that
//! throw the player out when violated. Restriction/reward effects are applied
//! at their engine sites (each names its EC). See
//! `docs/design/2026-07-04-eternity.md` §5.

use break_infinity::Decimal;

use crate::state::GameState;

/// Number of Eternity Challenges.
pub const ETERNITY_CHALLENGE_COUNT: usize = 12;

/// Base max completions per challenge. EC1 inside Enslaved's Reality raises
/// this to 1000 (`maxCompletions`); use
/// [`GameState::ec_max_completions`] for the dynamic per-challenge value.
pub const EC_MAX_COMPLETIONS: u16 = 5;

/// EC1's raised completion cap inside Enslaved's Reality.
pub const EC1_ENSLAVED_MAX_COMPLETIONS: u16 = 1000;

/// TT cost of each EC's unlock study (`ec-time-studies.js`), 1-indexed via
/// [`ec_study_cost`].
const EC_STUDY_COSTS: [f64; ETERNITY_CHALLENGE_COUNT] = [
    30.0, 35.0, 40.0, 70.0, 130.0, 85.0, 115.0, 115.0, 415.0, 550.0, 1.0, 1.0,
];

/// The tree studies whose ownership admits each EC study (`requirement`,
/// `AT_LEAST_ONE`).
const EC_STUDY_REQUIREMENTS: [&[u16]; ETERNITY_CHALLENGE_COUNT] = [
    &[171],
    &[171],
    &[171],
    &[143],
    &[42],
    &[121],
    &[111],
    &[123],
    &[151],
    &[181],
    &[231, 232],
    &[233, 234],
];

/// EC11/EC12's forbidden dimension-path studies (their `secondary.path`).
const EC11_FORBIDDEN: &[u16] = &[72, 73];
const EC12_FORBIDDEN: &[u16] = &[71, 72];

/// Base IP goals and per-completion goal multipliers.
const EC_GOALS: [Decimal; ETERNITY_CHALLENGE_COUNT] = [
    Decimal::new_unchecked(1.0, 1800),
    Decimal::new_unchecked(1.0, 975),
    Decimal::new_unchecked(1.0, 600),
    Decimal::new_unchecked(1.0, 2750),
    Decimal::new_unchecked(1.0, 750),
    Decimal::new_unchecked(1.0, 850),
    Decimal::new_unchecked(1.0, 2000),
    Decimal::new_unchecked(1.0, 1300),
    Decimal::new_unchecked(1.0, 1750),
    Decimal::new_unchecked(1.0, 3000),
    Decimal::new_unchecked(1.0, 450),
    Decimal::new_unchecked(1.0, 110_000),
];
const EC_GOAL_INCREASES: [Decimal; ETERNITY_CHALLENGE_COUNT] = [
    Decimal::new_unchecked(1.0, 200),
    Decimal::new_unchecked(1.0, 175),
    Decimal::new_unchecked(1.0, 75),
    Decimal::new_unchecked(1.0, 550),
    Decimal::new_unchecked(1.0, 400),
    Decimal::new_unchecked(1.0, 250),
    Decimal::new_unchecked(1.0, 530),
    Decimal::new_unchecked(1.0, 900),
    Decimal::new_unchecked(1.0, 250),
    Decimal::new_unchecked(1.0, 300),
    Decimal::new_unchecked(1.0, 200),
    Decimal::new_unchecked(1.0, 12_000),
];

/// The prerequisite studies of EC `id`'s unlock study (any one suffices;
/// empty for an invalid id).
pub(crate) fn ec_study_prerequisites(id: u8) -> &'static [u16] {
    if (1..=ETERNITY_CHALLENGE_COUNT as u8).contains(&id) {
        EC_STUDY_REQUIREMENTS[(id - 1) as usize]
    } else {
        &[]
    }
}

/// The TT cost of EC `id`'s unlock study (0 for an invalid id).
pub fn ec_study_cost(id: u8) -> f64 {
    if (1..=ETERNITY_CHALLENGE_COUNT as u8).contains(&id) {
        EC_STUDY_COSTS[(id - 1) as usize]
    } else {
        0.0
    }
}

impl GameState {
    /// Completions of EC `id` (0 for an invalid id).
    pub fn eternity_challenge_completions(&self, id: u8) -> u16 {
        if (1..=ETERNITY_CHALLENGE_COUNT as u8).contains(&id) {
            self.eternity_challenges[(id - 1) as usize]
        } else {
            0
        }
    }

    /// Max completions of EC `id` (`maxCompletions`): 5, except EC1 inside
    /// Enslaved's Reality, which allows 1000.
    pub fn ec_max_completions(&self, id: u8) -> u16 {
        if id == 1 && self.celestials.enslaved.run {
            EC1_ENSLAVED_MAX_COMPLETIONS
        } else {
            EC_MAX_COMPLETIONS
        }
    }

    /// Whether EC `id` is the one currently running.
    pub fn ec_running(&self, id: u8) -> bool {
        self.eternity_challenge_current == id && id != 0
    }

    /// Whether any EC is running.
    pub fn any_ec_running(&self) -> bool {
        self.eternity_challenge_current != 0
    }

    /// Whether EC `id`'s reward is active (any completion).
    pub fn ec_completed(&self, id: u8) -> bool {
        self.eternity_challenge_completions(id) > 0
    }

    /// `EternityChallenges.completions` — total completions across EC1–12.
    pub fn total_ec_completions(&self) -> u32 {
        (1..=12)
            .map(|id| self.eternity_challenge_completions(id) as u32)
            .sum()
    }

    /// The IP goal of EC `id` at `completions` (`goalAtCompletions`).
    pub fn ec_goal_at(&self, id: u8, completions: u16) -> Decimal {
        let i = (id - 1) as usize;
        EC_GOALS[i]
            * EC_GOAL_INCREASES[i].pow(&Decimal::from(
                completions.min(self.ec_max_completions(id) - 1) as u64,
            ))
    }

    /// The current IP goal of EC `id`.
    pub fn ec_current_goal(&self, id: u8) -> Decimal {
        self.ec_goal_at(id, self.eternity_challenge_completions(id))
    }

    // --- The EC unlock study --------------------------------------------------

    /// The secondary resource requirement of EC `id`'s study at the current
    /// completion count: `(current, required)` in comparable `Decimal`s, or
    /// `None` for the path-restricted EC11/EC12.
    pub fn ec_secondary_requirement(&self, id: u8) -> Option<(Decimal, Decimal)> {
        let c = self.eternity_challenge_completions(id).min(4) as f64;
        let pair = match id {
            // EC1's requirement keeps scaling inside Enslaved's Reality
            // (`Math.min(completions, Enslaved.isRunning ? 999 : 4)`).
            1 => {
                let c1 = self
                    .eternity_challenge_completions(1)
                    .min(self.ec_max_completions(1) - 1) as f64;
                (
                    self.eternities,
                    Decimal::from_float(20_000.0 + c1 * 20_000.0),
                )
            }
            2 => (
                Decimal::from(self.total_tick_gained),
                Decimal::from_float(1300.0 + c * 150.0),
            ),
            3 => (
                self.dimensions[7].amount,
                Decimal::from_float(17_300.0 + c * 1250.0),
            ),
            4 => (
                self.infinities_total(),
                Decimal::from_float(1e8 + c * 2.5e7),
            ),
            5 => (
                Decimal::from(self.galaxies as u64),
                Decimal::from_float(160.0 + c * 14.0),
            ),
            6 => (
                Decimal::from(self.replicanti.galaxies as u64),
                Decimal::from_float(40.0 + c * 5.0),
            ),
            7 => (self.antimatter, Decimal::pow10(500_000.0 + c * 300_000.0)),
            8 => (self.infinity_points, Decimal::pow10(4000.0 + c * 1000.0)),
            9 => (self.infinity_power, Decimal::pow10(17_500.0 + c * 2000.0)),
            10 => (self.eternity_points, Decimal::pow10(100.0 + c * 20.0)),
            _ => return None,
        };
        Some(pair)
    }

    /// Whether EC `id`'s unlock study can be bought (`ECTimeStudyState
    /// .canBeBought`): affordable, no EC study held, tree requirement met, and
    /// the secondary requirement (waived once previously met, except EC11/12,
    /// which instead forbid the conflicting dimension paths).
    pub fn can_buy_ec_study(&self, id: u8) -> bool {
        if !(1..=ETERNITY_CHALLENGE_COUNT as u8).contains(&id) {
            return false;
        }
        if self.eternity_challenge_unlocked != 0 {
            return false;
        }
        if self.time_theorems < Decimal::from_float(ec_study_cost(id)) {
            return false;
        }
        let reqs = EC_STUDY_REQUIREMENTS[(id - 1) as usize];
        if !reqs.iter().any(|&s| self.time_study_bought(s)) {
            return false;
        }
        // Secondary requirements (`allSecondaryRequirementsMet`).
        match id {
            11 => !EC11_FORBIDDEN.iter().any(|&s| self.time_study_bought(s)),
            12 => !EC12_FORBIDDEN.iter().any(|&s| self.time_study_bought(s)),
            _ => {
                // The ECR perk (72) waives non-TT requirements outright;
                // otherwise they are waived once previously met.
                if self.perk_applies(72) {
                    return true;
                }
                if self.ec_requirement_bits & (1 << id) != 0 {
                    return true;
                }
                match self.ec_secondary_requirement(id) {
                    Some((current, required)) => current >= required,
                    None => true,
                }
            }
        }
    }

    /// Buy EC `id`'s unlock study. Returns whether it happened.
    pub fn buy_ec_study(&mut self, id: u8) -> bool {
        if !self.can_buy_ec_study(id) {
            return false;
        }
        self.time_theorems -= Decimal::from_float(ec_study_cost(id));
        self.eternity_challenge_unlocked = id;
        self.ec_requirement_bits |= 1 << id;
        true
    }

    // --- Run flow --------------------------------------------------------------

    /// Whether EC `id` can be started (its study is held and it isn't running).
    pub fn can_start_eternity_challenge(&self, id: u8) -> bool {
        self.eternity_challenge_unlocked == id && !self.ec_running(id) && id != 0
    }

    /// Start EC `id` (`EternityChallengeState.start`): an Eternity (rewarded if
    /// at the goal, keeping the study slot) followed by the challenge reset.
    pub fn start_eternity_challenge(&mut self, id: u8) -> bool {
        if !self.can_start_eternity_challenge(id) {
            return false;
        }
        if self.can_eternity() {
            // `eternity(false, auto, { enteringEC: true })`: rewards granted,
            // respec suppressed.
            self.eternity_with_options(true);
        }
        self.eternity_challenge_current = id;
        // Starting EC12 resets the slowest-inversion tracker (IU24's gate).
        if id == 12 {
            self.requirement_checks.reality_slowest_bh = 1.0;
        }
        self.start_ec_reset();
        true
    }

    /// Exit the running EC (`EternityChallengeState.exit`): flag a respec and
    /// force an unrewarded Eternity reset.
    pub fn exit_eternity_challenge(&mut self) -> bool {
        if !self.any_ec_running() {
            return false;
        }
        self.eternity_challenge_current = 0;
        self.respec = true;
        self.eternity_reset();
        true
    }

    /// `startEternityChallenge()`: the same layer reset as an Eternity, minus
    /// the reward/autobuyer/break handling (those belong to `eternity()`).
    fn start_ec_reset(&mut self) {
        let current = self.eternity_challenge_current;
        self.eternity_reset_core();
        self.eternity_challenge_current = current;
    }

    /// EC4/EC12's mid-run restrictions (`isWithinRestriction`): EC4 caps the
    /// Infinities this run, EC12 the game time spent.
    pub fn ec_within_restriction(&self, id: u8) -> bool {
        let completions = self.eternity_challenge_completions(id) as f64;
        match id {
            4 => {
                let limit = (16.0 - 4.0 * completions).max(0.0);
                self.infinities <= Decimal::from_float(limit)
            }
            12 => {
                let limit = (10.0 - 2.0 * completions).max(1.0) / 10.0;
                self.records.this_eternity.time_ms / 1000.0 < limit
            }
            _ => true,
        }
    }

    /// `tryFail`: exit the running EC if its restriction is violated. Returns
    /// whether it failed. EC4 is checked on each Big Crunch, EC12 each tick.
    pub(crate) fn ec_try_fail(&mut self, id: u8) -> bool {
        if self.ec_running(id) && !self.ec_within_restriction(id) {
            self.exit_eternity_challenge();
            // CHALLENGE_FAILED achievement (114).
            self.check_challenge_failed_achievements();
            return true;
        }
        false
    }

    /// The total completions EC `id` would sit at if the player eternitied
    /// right now (`gainedCompletionStatus.totalCompletions`): the banked count
    /// plus what this eternity's IP peak reaches — one completion, or several
    /// with the ECB perk (73). Mirrors [`Self::complete_running_ec`]'s loop
    /// without mutating; used by the Eternity autobuyer's in-EC condition.
    pub fn ec_pending_total_completions(&self, id: u8) -> u16 {
        if id == 0 || id > ETERNITY_CHALLENGE_COUNT as u8 {
            return 0;
        }
        let mut total = self.eternity_challenges[(id - 1) as usize];
        while total < self.ec_max_completions(id)
            && self.records.this_eternity.max_ip >= self.ec_goal_at(id, total)
        {
            total += 1;
            if !self.perk_applies(73) {
                break;
            }
        }
        total
    }

    /// Bank a completion of the running EC on an Eternity
    /// (`giveEternityRewards`' challenge branch): +1 completion (capped),
    /// requirement bit cleared, tree auto-respecced.
    pub(crate) fn complete_running_ec(&mut self) {
        let id = self.eternity_challenge_current;
        if id == 0 {
            return;
        }
        let i = (id - 1) as usize;
        let max = self.ec_max_completions(id);
        self.eternity_challenges[i] = (self.eternity_challenges[i] + 1).min(max);
        // The ECB perk (73): keep banking completions while the higher
        // tiers' scaled goals are already met.
        if self.perk_applies(73) {
            while self.eternity_challenges[i] < max
                && self.records.this_eternity.max_ip >= self.ec_current_goal(id)
            {
                self.eternity_challenges[i] += 1;
            }
        }
        self.ec_requirement_bits &= !(1 << id);
        self.respec_time_studies_now();
    }

    // --- Effect readers used at the engine sites --------------------------------

    /// The game-speed factor (`getGameSpeedupFactor`): the EC12 fixed 1/1000
    /// takes priority; otherwise the Black Hole and `timespeed` glyph
    /// multipliers stack, clamped like the original.
    pub fn game_speed_factor(&self) -> f64 {
        if self.ec_running(12) {
            return 0.001;
        }
        let mut factor = 1.0;
        // Enslaved's Reality disables the Black Hole.
        if !self.celestials.enslaved.run {
            factor *= self.black_hole_speed_factor();
        }
        factor *= self.glyph_effect_timespeed();
        // Lai'tela's `gamespeedFromSingularities` milestone.
        factor *= self.singularity_milestone_effect_or(
            crate::celestials::singularity::GAMESPEED_FROM_SINGULARITIES,
            1.0,
        );
        // The `effarigblackhole` glyph effect raises the whole factor `^x`
        // (`game.js`: `factor = Math.pow(factor, effarigblackhole)`).
        let bh_pow = self.glyph_effect_effarigblackhole();
        if bh_pow != 1.0 {
            factor = factor.powf(bh_pow);
        }
        // Pelle: the `timeSpeedMult` rebuyable (while doomed).
        factor *= self.pelle_time_speed_mult();
        // Effarig's Reality compresses game speed too (`getGameSpeedupFactor`
        // NERFS block).
        if self.celestials.effarig.run {
            factor = self
                .effarig_multiplier(Decimal::from_float(factor))
                .to_f64();
        }
        factor.clamp(1e-300, 1e300)
    }

    /// `EternityChallenges.autoComplete.tick` (the PEC perks 60–62): accrue
    /// real time toward the interval and complete the next
    /// not-fully-completed EC sequentially. With Ra's `instantEC` unlock every
    /// EC completes immediately. The RU12/IU15 requirement-lock guards are out
    /// of frontier (armed req-locks are not modelled).
    pub(crate) fn ec_auto_complete_tick(&mut self, real_dt_ms: f64) {
        // `game.js`: the accumulator only advances with the first PEC perk.
        if !self.perk_applies(60) {
            return;
        }
        self.reality.last_auto_ec += real_dt_ms;
        let interval = self.ec_auto_complete_interval_ms();
        // Doomed (`Pelle.isDisabled("autoec")`) or toggled off: hold the
        // accumulator at one interval, complete nothing.
        if !self.reality.auto_ec || self.pelle_is_disabled("autoec") {
            self.reality.last_auto_ec = self.reality.last_auto_ec.min(interval);
            return;
        }
        if self.ra_unlock_active(crate::celestials::ra::RA_UNLOCK_INSTANT_EC) {
            for slot in self.eternity_challenges.iter_mut() {
                *slot = 5;
            }
            return;
        }
        while self.reality.last_auto_ec - interval > 0.0 {
            let Some(next) = self.eternity_challenges.iter().position(|&c| c < 5) else {
                break;
            };
            self.reality.last_auto_ec -= interval;
            self.eternity_challenges[next] += 1;
        }
        self.reality.last_auto_ec %= interval;
    }

    /// The EC auto-completion interval: the best PEC perk (60/40/20 minutes),
    /// divided by V's `fastAutoEC` reward (the achievement multiplier).
    pub fn ec_auto_complete_interval_ms(&self) -> f64 {
        if !self.perk_applies(60) {
            return f64::INFINITY;
        }
        let mut minutes = 60.0;
        if self.perk_applies(61) {
            minutes = 40.0;
        }
        if self.perk_applies(62) {
            minutes = 20.0;
        }
        minutes /= self.v_fast_auto_ec_effect();
        minutes * 60_000.0
    }

    /// EC3's reward: `+0.72` to the buy-10 multiplier per completion.
    pub(crate) fn ec3_buy_ten_bonus(&self) -> f64 {
        0.72 * self.eternity_challenge_completions(3) as f64
    }

    /// EC7's reward: TD1 produces 8th Infinity Dimensions at
    /// `TD1-production^(0.2·completions) − 1` per second.
    pub(crate) fn ec7_reward_id8_per_second(&self) -> Decimal {
        let completions = self.eternity_challenge_completions(7) as f64;
        if completions == 0.0 {
            return Decimal::ZERO;
        }
        (self
            .td_production_per_second(0)
            .pow(&Decimal::from_float(0.2 * completions))
            - Decimal::ONE)
            .max(&Decimal::ZERO)
    }

    /// EC8's reward: Replicanti Galaxies strengthened by Infinity Power —
    /// `max(0, (log10(log10(power)+1))^(0.03·completions) − 1)`.
    pub(crate) fn ec8_reward_rg_strength(&self) -> f64 {
        let completions = self.eternity_challenge_completions(8) as f64;
        if completions == 0.0 {
            return 0.0;
        }
        let inner = (self.infinity_power.pos_log10() + 1.0).log10();
        (inner.powf(0.03 * completions) - 1.0).max(0.0)
    }

    /// EC10's restriction-side boost: AD multiplier `infinitiesTotal^950`
    /// (clamped ≥ 1), raised to TS31's 4th power.
    pub(crate) fn ec10_ad_multiplier(&self) -> Decimal {
        let mut mult = self
            .infinities_total()
            .max(&Decimal::ONE)
            .pow(&Decimal::from_float(950.0));
        if self.time_study_bought(31) {
            mult = mult.pow(&Decimal::from_float(4.0));
        }
        mult
    }

    /// EC10's reward: TD multiplier `(2.783e-6 × infinitiesTotal)^(0.4+0.1c)`
    /// (clamped ≥ 1), raised to TS31's power.
    pub(crate) fn ec10_reward_td_mult(&self) -> Decimal {
        let completions = self.eternity_challenge_completions(10) as f64;
        if completions == 0.0 {
            return Decimal::ONE;
        }
        let mut mult = (self.infinities_total() * Decimal::from_float(2.783e-6))
            .pow(&Decimal::from_float(0.4 + 0.1 * completions))
            .max(&Decimal::ONE);
        if self.time_study_bought(31) {
            mult = mult.pow(&Decimal::from_float(4.0));
        }
        mult
    }

    /// EC12's reward: ID cost multipliers raised to `1 − 0.008·completions`.
    pub(crate) fn ec12_id_cost_pow(&self) -> f64 {
        1.0 - 0.008 * self.eternity_challenge_completions(12) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ec_autocomplete_completes_sequentially() {
        let mut game = GameState::new();
        // No PEC perk: nothing accrues or completes.
        game.ec_auto_complete_tick(3_600_000.0);
        assert_eq!(game.eternity_challenges, [0; 12]);

        // PEC1 (60 min): 2.5 hours completes two ECs (the loop uses a strict
        // `remaining − interval > 0`, like the original).
        game.reality.perks.insert(60);
        game.ec_auto_complete_tick(2.5 * 3_600_000.0);
        assert_eq!(game.eternity_challenges[0], 2);

        // PEC3 drops the interval to 20 minutes: the next hour (plus the 30
        // min carried) completes EC1 (5) and rolls into EC2.
        game.reality.perks.insert(62);
        game.ec_auto_complete_tick(3_600_000.0);
        assert_eq!(game.eternity_challenges[0], 5);
        assert_eq!(game.eternity_challenges[1], 1);
        // Another hour flows into EC2.
        game.ec_auto_complete_tick(3_600_000.0);
        assert_eq!(game.eternity_challenges[1], 4);

        // Toggled off: the accumulator clamps at one interval, no completions.
        game.reality.auto_ec = false;
        game.ec_auto_complete_tick(3_600_000.0);
        assert_eq!(game.eternity_challenges[1], 4);
        assert!(game.reality.last_auto_ec <= game.ec_auto_complete_interval_ms());
    }

    #[test]
    fn autobuyer_faster_perks_shrink_the_interval() {
        let mut game = GameState::new();
        assert_eq!(game.perk_autobuyer_faster(101), 1.0);
        game.reality.perks.insert(101);
        assert!((game.perk_autobuyer_faster(101) - 1.0 / 3.0).abs() < 1e-12);
    }
    use crate::ETERNITY_GOAL;

    /// A state holding the studies feeding EC1's slot, with plenty of TT.
    fn game_ready_for_ec1() -> GameState {
        let mut game = GameState::new();
        game.time_theorems = Decimal::from_float(1000.0);
        game.studies = vec![171];
        game.eternities = Decimal::from_float(25_000.0);
        game
    }

    #[test]
    fn ec_study_needs_requirements() {
        let mut game = GameState::new();
        game.time_theorems = Decimal::from_float(1000.0);
        // No TS171 yet.
        assert!(!game.can_buy_ec_study(1));
        game.studies = vec![171];
        // Secondary requirement (20000 eternities) not met.
        assert!(!game.can_buy_ec_study(1));
        game.eternities = Decimal::from_float(25_000.0);
        assert!(game.buy_ec_study(1));
        assert_eq!(game.eternity_challenge_unlocked, 1);
        assert_eq!(game.time_theorems, Decimal::from_float(970.0));
        // Only one EC study at a time.
        assert!(!game.can_buy_ec_study(2));
    }

    #[test]
    fn ec_requirement_waived_after_first_unlock() {
        let mut game = game_ready_for_ec1();
        assert!(game.buy_ec_study(1));
        // Respec refunds the study and drops eternities below the requirement…
        game.respec_time_studies_now();
        game.studies = vec![171];
        game.eternities = Decimal::ZERO;
        // …but the requirement bit waives the secondary check.
        assert!(game.can_buy_ec_study(1));
    }

    #[test]
    fn ec11_forbids_conflicting_paths() {
        let mut game = GameState::new();
        game.time_theorems = Decimal::from_float(1000.0);
        game.studies = vec![231, 72];
        assert!(!game.can_buy_ec_study(11));
        game.studies = vec![231, 71];
        assert!(game.can_buy_ec_study(11));
    }

    /// EC1's completion cap rises to 1000 inside Enslaved's Reality
    /// (`maxCompletions`): the goal keeps scaling past 5 completions and the
    /// study's secondary requirement keeps growing.
    #[test]
    fn ec1_allows_1000_completions_inside_enslaved() {
        let mut game = GameState::new();
        assert_eq!(game.ec_max_completions(1), 5);
        game.celestials.enslaved.run = true;
        assert_eq!(game.ec_max_completions(1), 1000);
        // Other ECs keep the base cap.
        assert_eq!(game.ec_max_completions(2), 5);

        // The goal scales past the base cap: at 10 completions the goal is
        // 1e1800 × (1e200)^10.
        game.eternity_challenges[0] = 10;
        assert_eq!(game.ec_current_goal(1), Decimal::new(1.0, 1800 + 200 * 10));
        // Outside the run the goal clamps back to the base cap's scaling.
        game.celestials.enslaved.run = false;
        assert_eq!(game.ec_current_goal(1), Decimal::new(1.0, 1800 + 200 * 4));

        // The secondary requirement keeps scaling inside the run:
        // 20000 + completions × 20000.
        game.celestials.enslaved.run = true;
        let (_, required) = game.ec_secondary_requirement(1).unwrap();
        assert_eq!(required, Decimal::from_float(20_000.0 + 10.0 * 20_000.0));

        // Completions bank past 5 inside the run.
        game.eternity_challenge_current = 1;
        game.records.this_eternity.max_ip = Decimal::new(1.0, 4000);
        game.complete_running_ec();
        assert_eq!(game.eternity_challenge_completions(1), 11);
    }

    #[test]
    fn start_run_complete_ec() {
        let mut game = game_ready_for_ec1();
        assert!(game.buy_ec_study(1));
        assert!(game.start_eternity_challenge(1));
        assert!(game.ec_running(1));

        // The Eternity goal is now EC1's goal.
        assert_eq!(game.eternity_goal(), Decimal::new(1.0, 1800));
        assert!(!game.can_eternity());

        // Reach the goal and Eternity: a completion is banked and the tree
        // auto-respecs (study slot cleared).
        game.records.this_eternity.max_ip = Decimal::new(1.0, 1800);
        assert!(game.eternity());
        assert_eq!(game.eternity_challenge_completions(1), 1);
        assert_eq!(game.eternity_challenge_unlocked, 0);
        assert!(!game.any_ec_running());
        // The next goal scales by 1e200.
        assert_eq!(game.ec_current_goal(1), Decimal::new(1.0, 2000));
    }

    #[test]
    fn exit_ec_respecs_and_resets() {
        let mut game = game_ready_for_ec1();
        game.buy_ec_study(1);
        game.start_eternity_challenge(1);
        assert!(game.exit_eternity_challenge());
        assert!(!game.any_ec_running());
        // The exit-triggered respec runs inside the forced Eternity (flag
        // consumed, tree refunded — `exit()` sets respec then `eternity(true)`).
        assert!(!game.respec);
        assert!(game.studies.is_empty());
        // 1000 − 30 (EC1 study) + 30 (its refund) + 15 (the injected TS171's
        // refund — the fixture granted it without paying).
        assert_eq!(game.time_theorems, Decimal::from_float(1015.0));
        // Below the goal, no completion was banked.
        assert_eq!(game.eternity_challenge_completions(1), 0);
    }

    #[test]
    fn ec12_time_limit_fails_the_run() {
        let mut game = GameState::new();
        game.time_theorems = Decimal::from_float(1000.0);
        game.studies = vec![233];
        assert!(game.buy_ec_study(12));
        assert!(game.start_eternity_challenge(12));
        assert!(game.ec_running(12));
        // The game runs 1000× slower.
        assert_eq!(game.game_speed_factor(), 0.001);

        // One real second is one game millisecond — still within the 1 s limit.
        game.tick(1000.0);
        assert!(game.ec_running(12));
        // Push game time past the limit: 1 s of game time needs 1000 s real.
        game.simulate(1_100_000.0, 10_000.0);
        assert!(!game.ec_running(12));
        // Failure exits without a completion.
        assert_eq!(game.eternity_challenge_completions(12), 0);
    }

    #[test]
    fn ec4_infinity_limit_fails_on_crunch() {
        let mut game = GameState::new();
        game.time_theorems = Decimal::from_float(1000.0);
        game.studies = vec![143];
        game.infinities_banked = Decimal::from_float(2e8);
        assert!(game.buy_ec_study(4));
        assert!(game.start_eternity_challenge(4));

        // 16 crunches are fine; the 17th violates the restriction.
        for _ in 0..16 {
            game.antimatter = crate::data::constants::BIG_CRUNCH_THRESHOLD;
            game.records.this_infinity.max_am = game.antimatter;
            assert!(game.big_crunch());
            assert!(game.ec_running(4));
        }
        game.antimatter = crate::data::constants::BIG_CRUNCH_THRESHOLD;
        game.records.this_infinity.max_am = game.antimatter;
        assert!(game.big_crunch());
        assert!(!game.ec_running(4));
    }

    #[test]
    fn eternity_goal_scales_with_completions() {
        let mut game = GameState::new();
        game.eternity_challenges[0] = 3;
        assert_eq!(game.ec_current_goal(1), Decimal::new(1.0, 2400));
        // Completions cap the goal exponent at +4 increases.
        game.eternity_challenges[0] = 5;
        assert_eq!(game.ec_current_goal(1), Decimal::new(1.0, 2600));
    }

    #[test]
    fn eternity_button_uses_ec_goal() {
        let mut game = game_ready_for_ec1();
        game.buy_ec_study(1);
        game.start_eternity_challenge(1);
        game.records.this_eternity.max_ip = ETERNITY_GOAL; // 1.8e308 < 1e1800
        assert!(!game.can_eternity());
    }
}
