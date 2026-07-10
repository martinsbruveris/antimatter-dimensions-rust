//! V (Feature 7.4) — the Celestial of Achievements. Six simultaneous unlock
//! conditions gate V; then V's Reality (square-rooted multipliers, squared
//! Replicanti interval) hosts 9 V-achievements whose completions become Space
//! Theorems, which unlock the ST-gated rewards. See
//! `docs/design/2026-07-06-celestials.md` §4. Original: `celestials/V.js` +
//! `secret-formula/celestials/v.js`.
//!
//! **Scope.** The six main-unlock conditions, the run modifiers, all 9
//! V-achievement conditions/goals + `tryComplete`, Space Theorems, and the
//! `adPow` reward are ported. The hard achievements (ids 6–8) require Ra's
//! "flip" and so never complete in frontier — faithful. **Deferred:** the
//! Perk-Point goal reduction, and the `fastAutoEC` / `autoAutoClean` /
//! `achievementBH` / `raUnlock` reward *effects* (their systems are unbuilt or
//! Ra-gated); the reward *unlock flags* still fire.

use crate::state::GameState;

/// VUnlock ids (`unlockBits` bit positions).
pub const V_UNLOCK_CELESTIAL: u8 = 0;
pub const V_UNLOCK_SHARD_REDUCTION: u8 = 1;
pub const V_UNLOCK_AD_POW: u8 = 2;
pub const V_UNLOCK_FAST_AUTO_EC: u8 = 3;
pub const V_UNLOCK_AUTO_AUTO_CLEAN: u8 = 4;
pub const V_UNLOCK_ACHIEVEMENT_BH: u8 = 5;
pub const V_UNLOCK_RA: u8 = 6;

/// The Space-Theorem thresholds of the ST-gated unlocks, `(bit, ST)`.
pub const V_UNLOCK_ST_THRESHOLDS: [(u8, u32); 6] = [
    (V_UNLOCK_SHARD_REDUCTION, 2),
    (V_UNLOCK_AD_POW, 5),
    (V_UNLOCK_FAST_AUTO_EC, 10),
    (V_UNLOCK_AUTO_AUTO_CLEAN, 16),
    (V_UNLOCK_ACHIEVEMENT_BH, 30),
    (V_UNLOCK_RA, 36),
];

/// Per-achievement tier goals (`runUnlocks[i].values`). Ids 0–5 have 6 tiers;
/// the hard ids 6–8 have 5.
pub const V_ACHIEVEMENT_VALUES: [&[f64]; 9] = [
    &[-5.0, -4.0, -3.0, -2.0, -1.0, 0.0],
    &[4000.0, 4300.0, 4600.0, 4900.0, 5200.0, 5500.0],
    &[6e5, 7.2e5, 8.4e5, 9.6e5, 1.08e6, 1.2e6],
    &[400e6, 450e6, 500e6, 600e6, 700e6, 800e6],
    &[7000.0, 7600.0, 8200.0, 8800.0, 9400.0, 10000.0],
    &[51.0, 52.0, 53.0, 54.0, 55.0, 56.0],
    &[1.0, 4.0, 7.0, 10.0, 13.0],
    &[100.0, 150.0, 200.0, 250.0, 300.0],
    &[6500.0, 7000.0, 8000.0, 9000.0, 10000.0],
];

/// Whether each achievement is "hard" (needs Ra's flip to complete).
pub const V_ACHIEVEMENT_HARD: [bool; 9] =
    [false, false, false, false, false, false, true, true, true];

/// The six main-unlock requirements (`mainUnlock`), by log10 (or raw count for
/// realities): realities 10000, eternities 1e70, total infinities 1e160, this-
/// reality DT 1e320, this-reality replicanti 1e320000, RM 1e60.
pub const V_MAIN_UNLOCK_REALITIES: f64 = 10000.0;
pub const V_MAIN_UNLOCK_ETERNITIES_LOG: f64 = 70.0;
pub const V_MAIN_UNLOCK_INFINITIES_LOG: f64 = 160.0;
pub const V_MAIN_UNLOCK_DT_LOG: f64 = 320.0;
pub const V_MAIN_UNLOCK_REPLICANTI_LOG: f64 = 320000.0;
pub const V_MAIN_UNLOCK_RM_LOG: f64 = 60.0;

/// `player.celestials.v`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VState {
    /// Unlock bits (`unlockBits`): bit 0 = V unlocked, 1–6 = the ST rewards.
    #[cfg_attr(feature = "serde", serde(default))]
    pub unlock_bits: u32,
    /// Whether V's Reality is running (`run`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub run: bool,
    /// Per-achievement tier completions (`runUnlocks`, 9 entries).
    #[cfg_attr(feature = "serde", serde(default))]
    pub run_unlocks: [u32; 9],
    /// Per-achievement goal-reduction steps (`goalReductionSteps`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub goal_reduction_steps: [u32; 9],
    /// Space Theorems spent on goal reduction (`STSpent`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub st_spent: u32,
    /// Best value reached per achievement across all runs (`runRecords`, plain
    /// numbers in the save — log10s / counts). The id-0 record starts at `-10`.
    #[cfg_attr(feature = "serde", serde(default = "default_v_run_records"))]
    pub run_records: [f64; 9],
}

fn default_v_run_records() -> [f64; 9] {
    let mut r = [0.0; 9];
    r[0] = -10.0;
    r
}

impl Default for VState {
    fn default() -> Self {
        Self::new()
    }
}

impl VState {
    pub fn new() -> Self {
        Self {
            unlock_bits: 0,
            run: false,
            run_unlocks: [0; 9],
            goal_reduction_steps: [0; 9],
            st_spent: 0,
            run_records: default_v_run_records(),
        }
    }

    /// Whether VUnlock `id` is owned.
    pub fn unlock_bought(&self, id: u8) -> bool {
        self.unlock_bits & (1u32 << id) != 0
    }
}

impl GameState {
    // --- Availability -----------------------------------------------------------

    /// Whether V (the celestial) is unlocked — bit 0.
    pub fn v_celestial_unlocked(&self) -> bool {
        self.celestials.v.unlock_bought(V_UNLOCK_CELESTIAL)
    }

    /// Whether V's tab is available: V unlocked, or all six main conditions met
    /// (so the "unlock V" prompt shows). Gated on the Effarig-eternity Enslaved
    /// chain being reachable is implicit (realities requirement).
    pub fn v_tab_available(&self) -> bool {
        self.v_celestial_unlocked() || self.enslaved_unlocked()
    }

    /// The six main-unlock progress fractions (`mainUnlock[i].progress()`),
    /// each clamped to [0, 1].
    pub fn v_main_unlock_progress(&self) -> [f64; 6] {
        let emphasize = |frac: f64| frac.clamp(0.0, 1.0).powi(10);
        [
            (self.reality.realities as f64 / V_MAIN_UNLOCK_REALITIES).clamp(0.0, 1.0),
            emphasize(self.eternities.pos_log10() / V_MAIN_UNLOCK_ETERNITIES_LOG),
            emphasize(
                self.infinities_total().pos_log10() / V_MAIN_UNLOCK_INFINITIES_LOG,
            ),
            emphasize(
                self.records.this_reality.max_dt.pos_log10() / V_MAIN_UNLOCK_DT_LOG,
            ),
            emphasize(
                self.records.this_reality.max_replicanti.pos_log10()
                    / V_MAIN_UNLOCK_REPLICANTI_LOG,
            ),
            emphasize(self.reality.machines.pos_log10() / V_MAIN_UNLOCK_RM_LOG),
        ]
    }

    /// `VUnlocks.vAchievementUnlock.canBeUnlocked`: every main condition met and
    /// V not yet unlocked.
    pub fn v_can_unlock_celestial(&self) -> bool {
        !self.v_celestial_unlocked()
            && self.v_main_unlock_progress().iter().all(|&p| p >= 1.0)
    }

    /// `V.unlockCelestial`.
    pub fn v_unlock_celestial(&mut self) -> bool {
        if !self.v_can_unlock_celestial() {
            return false;
        }
        self.celestials.v.unlock_bits |= 1u32 << V_UNLOCK_CELESTIAL;
        true
    }

    // --- Space Theorems / rewards -----------------------------------------------

    /// `V.spaceTheorems` (`updateTotalRunUnlocks`): Σ completions, ids ≥ 6 ×2.
    pub fn v_space_theorems(&self) -> u32 {
        self.celestials
            .v
            .run_unlocks
            .iter()
            .enumerate()
            .map(|(i, &c)| if i < 6 { c } else { c * 2 })
            .sum()
    }

    /// `VUnlocks.adPow`: the Antimatter-Dimension power `1 + √ST/100` (1 if the
    /// reward is not unlocked).
    pub(crate) fn v_ad_pow(&self) -> f64 {
        if !self.is_doomed() && self.celestials.v.unlock_bought(V_UNLOCK_AD_POW) {
            1.0 + (self.v_space_theorems() as f64).sqrt() / 100.0
        } else {
            1.0
        }
    }

    /// `VUnlocks.fastAutoEC`: the achievement multiplier divides the EC
    /// auto-completion interval (1 until the reward is unlocked).
    pub(crate) fn v_fast_auto_ec_effect(&self) -> f64 {
        if !self.is_doomed() && self.celestials.v.unlock_bought(V_UNLOCK_FAST_AUTO_EC) {
            self.achievement_power().to_f64()
        } else {
            1.0
        }
    }

    /// `VUnlocks.achievementBH`: the achievement multiplier boosts Black Hole
    /// power (1 until the reward is unlocked).
    pub(crate) fn v_achievement_bh_effect(&self) -> f64 {
        if !self.is_doomed() && self.celestials.v.unlock_bought(V_UNLOCK_ACHIEVEMENT_BH)
        {
            self.achievement_power().to_f64()
        } else {
            1.0
        }
    }

    /// `VUnlocks.autoAutoClean.isUnlocked` — the 16-ST reward (UI gate).
    pub fn v_auto_auto_clean_unlocked(&self) -> bool {
        self.celestials.v.unlock_bought(V_UNLOCK_AUTO_AUTO_CLEAN)
    }

    /// `VUnlocks.autoAutoClean.canBeApplied`: the 16-ST reward, dead while
    /// Doomed.
    pub(crate) fn v_auto_auto_clean_applies(&self) -> bool {
        !self.is_doomed() && self.v_auto_auto_clean_unlocked()
    }

    // --- Goal reduction (`VUnlocks.shardReduction`, Perk-Point spending) --------

    /// `shardReduction(tiers)` — the per-achievement reduction curve.
    fn v_shard_reduction(id: usize, tiers: f64) -> f64 {
        match id {
            1 => (300.0 * tiers).floor(),
            2 => 1.2e5 * tiers,
            3 => 50e6 * tiers,
            4 => 600.0 * tiers,
            5 => tiers.floor(),
            7 => 50.0 * tiers,
            8 => (500.0 * tiers).floor(),
            _ => 0.0,
        }
    }

    /// `maxShardReduction(goal)` — the reduction cap per achievement.
    fn v_max_shard_reduction(id: usize, goal: f64) -> f64 {
        match id {
            1 => goal - 4000.0,
            2 => goal - 6e5,
            3 => goal - 400e6,
            4 => goal - 7000.0,
            5 => 5.0,
            7 => goal - 50.0,
            8 => 500.0,
            _ => 0.0,
        }
    }

    /// `reductionStepSize` (1 unless specified).
    fn v_reduction_step_size(id: usize) -> u32 {
        match id {
            5 => 100,
            7 => 2,
            _ => 1,
        }
    }

    /// `conditionBaseValue` — the current tier's unreduced goal.
    fn v_condition_base_value(&self, id: usize) -> f64 {
        let goals = V_ACHIEVEMENT_VALUES[id];
        let completions = self.celestials.v.run_unlocks[id] as usize;
        goals
            .get(completions)
            .copied()
            .unwrap_or(goals[goals.len() - 1])
    }

    /// `reduction` — the clamped goal reduction for achievement `id`.
    fn v_goal_reduction(&self, id: usize) -> f64 {
        let tiers = self.celestials.v.goal_reduction_steps[id] as f64 / 100.0;
        let max =
            Self::v_max_shard_reduction(id, self.v_condition_base_value(id)).max(0.0);
        Self::v_shard_reduction(id, tiers).clamp(0.0, max)
    }

    /// `conditionValue` — the effective (possibly reduced) goal.
    fn v_condition_value(&self, id: usize) -> f64 {
        let base = self.v_condition_base_value(id);
        // `isReduced`: steps bought and the shardReduction unlock alive
        // (its effect dies while Doomed).
        if self.celestials.v.goal_reduction_steps[id] == 0
            || self.is_doomed()
            || !self.celestials.v.unlock_bought(V_UNLOCK_SHARD_REDUCTION)
        {
            return base;
        }
        let reduction = self.v_goal_reduction(id);
        if reduction <= 0.0 {
            return base;
        }
        base - reduction
    }

    /// `reductionCost` — Perk Points for the next reduction step(s): 1000 per
    /// step normally; hard achievements scale ×1.15 per step already bought
    /// (with the bulk-buy factor for multi-step purchases).
    pub fn v_reduction_cost(&self, id: usize) -> f64 {
        let step = Self::v_reduction_step_size(id) as f64;
        if V_ACHIEVEMENT_HARD[id] {
            let modified_step_count = (1.15f64.powf(step) - 1.0) / 0.15;
            modified_step_count
                * 1000.0
                * 1.15f64.powi(self.celestials.v.goal_reduction_steps[id] as i32)
        } else {
            step * 1000.0
        }
    }

    /// `canBeReduced`: mid-progress (some completions, not maxed) and the
    /// reduction not yet at its cap.
    pub fn v_can_be_reduced(&self, id: usize) -> bool {
        let completions = self.celestials.v.run_unlocks[id] as usize;
        completions < V_ACHIEVEMENT_VALUES[id].len()
            && completions != 0
            && self.v_goal_reduction(id)
                != Self::v_max_shard_reduction(id, self.v_condition_base_value(id))
    }

    /// `reduceGoals` — spend Perk Points for one reduction purchase (the
    /// original's only hard gate is affordability), then re-check completions.
    pub fn v_reduce_goal(&mut self, id: usize) -> bool {
        if id >= 9 {
            return false;
        }
        let cost = self.v_reduction_cost(id);
        if self.reality.perk_points < cost {
            return false;
        }
        self.reality.perk_points -= cost;
        self.celestials.v.goal_reduction_steps[id] += Self::v_reduction_step_size(id);
        for i in 0..9 {
            self.v_try_complete(i);
        }
        self.v_check_for_unlocks();
        true
    }

    /// Whether V's achievements are "flipped" (Ra's `unlockHardV`) — enables the
    /// hard achievements once Ra's V pet reaches level 6.
    fn v_is_flipped(&self) -> bool {
        self.ra_hard_v_unlocked()
    }

    // --- Per-tick unlock checks (`V.checkForUnlocks`) ---------------------------

    /// Auto-unlock the ST-gated rewards and, while running, try to complete each
    /// V-achievement. Called each tick.
    pub(crate) fn v_check_for_unlocks(&mut self) {
        if !self.v_celestial_unlocked() {
            return;
        }
        let st = self.v_space_theorems();
        for (bit, threshold) in V_UNLOCK_ST_THRESHOLDS {
            if st >= threshold {
                self.celestials.v.unlock_bits |= 1u32 << bit;
            }
        }
        if self.celestials.v.run {
            for id in 0..9usize {
                self.v_try_complete(id);
            }
        }
    }

    /// `VRunUnlockState.tryComplete`: record the best value reached and bump the
    /// completion count while the record clears the (base) goal. Hard
    /// achievements need the Ra flip, so they only record — never complete —
    /// in frontier.
    fn v_try_complete(&mut self, id: usize) {
        let value = self.v_achievement_current_value(id);
        if self.v_achievement_condition(id) && value >= self.celestials.v.run_records[id]
        {
            self.celestials.v.run_records[id] = value;
        }
        // Hard achievements never complete without the flip; skip the loop
        // (the original `continue`s, which would spin — we break cleanly).
        if V_ACHIEVEMENT_HARD[id] && !self.v_is_flipped() {
            return;
        }
        // The goal may be reduced by Perk-Point spending (`conditionValue`);
        // recomputed per tier as completions advance.
        let goals = V_ACHIEVEMENT_VALUES[id];
        while (self.celestials.v.run_unlocks[id] as usize) < goals.len()
            && self.celestials.v.run_records[id] >= self.v_condition_value(id)
        {
            self.celestials.v.run_unlocks[id] += 1;
        }
    }

    /// Space Theorems not yet spent on goal reduction (`V.availableST`).
    pub fn v_available_space_theorems(&self) -> u32 {
        self.v_space_theorems()
            .saturating_sub(self.celestials.v.st_spent)
    }

    /// View helper: `(completions, tiers, current_value, next_goal,
    /// condition_met, is_hard)` for V-achievement `id`.
    pub fn v_achievement_status(&self, id: usize) -> (u32, u32, f64, f64, bool, bool) {
        let goals = V_ACHIEVEMENT_VALUES[id];
        let completions = self.celestials.v.run_unlocks[id];
        (
            completions,
            goals.len() as u32,
            self.v_achievement_current_value(id),
            // The displayed goal honours any Perk-Point reduction.
            self.v_condition_value(id),
            self.v_achievement_condition(id),
            V_ACHIEVEMENT_HARD[id],
        )
    }

    /// `runUnlocks[id].currentValue()` — the achievement's live measured value.
    fn v_achievement_current_value(&self, id: usize) -> f64 {
        match id {
            0 => -(self.active_glyphs_without_companion().len() as f64),
            1 => {
                (self.replicanti.galaxies + self.extra_replicanti_galaxies()) as f64
                    + self.galaxies as f64
                    + self.dilation.total_tachyon_galaxies
            }
            2 => self.infinity_points.pos_log10(),
            3 => self.antimatter.pos_log10(),
            4 => self.eternity_points.pos_log10(),
            5 => self.dim_boosts as f64,
            6 => -(self.requirement_checks.reality_max_glyphs as f64),
            // id 7 needs the slowest-Black-Hole record we don't track; it is a
            // hard achievement (never completes), so 0 is safe.
            7 => 0.0,
            8 => self.gained_glyph_level().actual_level as f64,
            _ => 0.0,
        }
    }

    /// `runUnlocks[id].condition()` — whether the achievement is measurable now.
    fn v_achievement_condition(&self, id: usize) -> bool {
        if !self.celestials.v.run {
            return false;
        }
        match id {
            0 => self.dilation_study_bought(6),
            1 => true,
            2 => self.ec_running(7),
            3 => self.ec_running(12) && !self.dilation_unlocked(),
            4 => true,
            5 => self.dilation.active && self.ec_running(5),
            6 => self.dilation_study_bought(6),
            7 => true,
            8 => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use break_infinity::Decimal;

    fn v_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game
    }

    #[test]
    fn perk_points_reduce_achievement_goals() {
        let mut game = v_game();
        game.celestials.v.unlock_bits |= 1 << V_UNLOCK_SHARD_REDUCTION;
        // Achievement 4 (Eternal Sunshine): tier 1 done, goal 7600.
        game.celestials.v.run_unlocks[4] = 1;
        game.celestials.v.run_records[4] = 7300.0;
        assert_eq!(game.v_reduction_cost(4), 1000.0);
        assert!(game.v_can_be_reduced(4));
        game.reality.perk_points = 50_000.0;
        // 50 steps = 0.5 tiers → −300; goal 7600 → 7300.
        for _ in 0..50 {
            assert!(game.v_reduce_goal(4));
        }
        assert_eq!(game.reality.perk_points, 0.0);
        // The record (7300) now clears the reduced goal.
        assert_eq!(game.celestials.v.run_unlocks[4], 2);
    }

    #[test]
    fn hard_reduction_costs_scale() {
        let mut game = v_game();
        // Achievement 7 (Post-destination, hard): step size 2, ×1.15/step.
        let first = game.v_reduction_cost(7);
        assert!((first - (1.15f64.powi(2) - 1.0) / 0.15 * 1000.0).abs() < 1e-9);
        game.celestials.v.goal_reduction_steps[7] = 2;
        assert!(game.v_reduction_cost(7) > first);
    }

    #[test]
    fn main_unlock_needs_all_six_conditions() {
        let mut game = v_game();
        assert!(!game.v_can_unlock_celestial());
        game.reality.realities = 10_000;
        game.eternities = Decimal::new(1.0, 70);
        game.infinities = Decimal::new(1.0, 160);
        game.records.this_reality.max_dt = Decimal::new(1.0, 320);
        game.records.this_reality.max_replicanti = Decimal::new(1.0, 320_000);
        game.reality.machines = Decimal::new(1.0, 60);
        assert!(game.v_can_unlock_celestial());
        assert!(game.v_unlock_celestial());
        assert!(game.v_celestial_unlocked());
    }

    #[test]
    fn completing_an_achievement_grants_space_theorems() {
        let mut game = v_game();
        game.celestials.v.unlock_bits |= 1 << V_UNLOCK_CELESTIAL;
        game.celestials.v.run = true;
        // Eternal Sunshine (id 4): reach 1e7000 EP → tier 1.
        game.eternity_points = Decimal::new(1.0, 7000);
        game.v_check_for_unlocks();
        assert_eq!(game.celestials.v.run_unlocks[4], 1);
        assert_eq!(game.v_space_theorems(), 1);
    }

    #[test]
    fn hard_achievements_never_complete_in_frontier() {
        let mut game = v_game();
        game.celestials.v.unlock_bits |= 1 << V_UNLOCK_CELESTIAL;
        game.celestials.v.run = true;
        // Shutter Glyph (id 8, hard): even a huge glyph level records but never
        // completes.
        game.celestials.v.run_records[8] = 20_000.0;
        game.v_try_complete(8);
        assert_eq!(game.celestials.v.run_unlocks[8], 0);
    }

    #[test]
    fn ad_pow_scales_with_space_theorems() {
        let mut game = v_game();
        // 5 ST unlocks adPow; the effect is 1 + √ST/100.
        for i in 0..5 {
            game.celestials.v.run_unlocks[i] = 1;
        }
        game.celestials.v.unlock_bits |= 1 << V_UNLOCK_CELESTIAL;
        game.v_check_for_unlocks();
        assert!(game.celestials.v.unlock_bought(V_UNLOCK_AD_POW));
        let expected = 1.0 + 5f64.sqrt() / 100.0;
        assert!((game.v_ad_pow() - expected).abs() < 1e-9);
    }
}
