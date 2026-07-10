//! Black Holes (Feature 6.5): two purchasable holes that periodically speed
//! up game time. Each cycles inactive → active with upgradeable interval /
//! power / duration; the second hole's phase only advances while the first is
//! active.
//!
//! Mirrors `src/core/black-hole.js` (`BlackHoleState`,
//! `BlackHoles.updatePhases`, the game-speed factor) with the celestial
//! extras (inversion, auto-pause modes, Enslaved interactions) out of
//! frontier. Costs use the same hybrid scaling as the Reality Upgrades. See
//! `docs/design/2026-07-05-reality.md`.

use break_infinity::Decimal;

use crate::state::GameState;

/// RM cost to unlock the first Black Hole.
pub const BH1_UNLOCK_COST: Decimal = Decimal::new_unchecked(1.0, 2);

/// The unpause acceleration ramp (seconds of real time to full power).
const ACCELERATION_TIME_S: f64 = 5.0;

/// One black hole (`player.blackHole[i]`).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlackHole {
    pub unlocked: bool,
    /// Whether the hole is in its charged (active) phase segment.
    pub active: bool,
    /// Time into the current segment, seconds.
    pub phase: f64,
    /// Completed activation count (display).
    pub activations: u32,
    /// Upgrade purchase counts.
    pub interval_upgrades: u32,
    pub power_upgrades: u32,
    pub duration_upgrades: u32,
}

impl BlackHole {
    pub fn new() -> Self {
        Self {
            unlocked: false,
            active: false,
            phase: 0.0,
            activations: 0,
            interval_upgrades: 0,
            power_upgrades: 0,
            duration_upgrades: 0,
        }
    }
}

impl Default for BlackHole {
    fn default() -> Self {
        Self::new()
    }
}

/// Both holes plus the global pause state.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlackHolesState {
    pub holes: [BlackHole; 2],
    /// `player.blackHolePause`.
    pub paused: bool,
    /// `player.blackHolePauseTime` — realTimePlayed at the last (un)pause,
    /// for the 5 s acceleration ramp.
    pub pause_time_ms: f64,
    /// `player.blackHoleNegative` — the inversion strength (game speed while
    /// paused-and-inverted); 1 = no inversion.
    #[cfg_attr(feature = "serde", serde(default = "default_negative"))]
    pub negative: f64,
    /// `player.blackHoleAutoPauseMode` — 0 never, 1 pause before BH1
    /// activates, 2 before BH2.
    #[cfg_attr(feature = "serde", serde(default))]
    pub auto_pause_mode: u8,
}

#[cfg(feature = "serde")]
fn default_negative() -> f64 {
    1.0
}

impl BlackHolesState {
    pub fn new() -> Self {
        Self {
            holes: [BlackHole::new(), BlackHole::new()],
            paused: false,
            pause_time_ms: 0.0,
            negative: 1.0,
            auto_pause_mode: 0,
        }
    }
}

impl Default for BlackHolesState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// A hole's inactive interval in seconds (`3600/10^id × 0.8^upgrades`;
    /// 0 once permanent).
    pub fn black_hole_interval(&self, index: usize) -> f64 {
        if self.black_hole_is_permanent(index) {
            return 0.0;
        }
        self.black_hole_raw_interval(index)
    }

    fn black_hole_raw_interval(&self, index: usize) -> f64 {
        let mut interval = (3600.0 / 10f64.powi(index as i32))
            * 0.8f64.powi(self.black_holes.holes[index].interval_upgrades as i32);
        // Achievement 145: Black Hole intervals are 10% shorter.
        if self.achievement_unlocked(145) {
            interval *= 0.9;
        }
        interval
    }

    /// A hole's game-speed power while active (`180/2^id × 1.35^upgrades`).
    pub fn black_hole_power(&self, index: usize) -> f64 {
        let mut power = (180.0 / 2f64.powi(index as i32))
            * 1.35f64.powi(self.black_holes.holes[index].power_upgrades as i32);
        // Achievement 158 (both holes permanent): Black Hole power +10%.
        if self.achievement_unlocked(158) {
            power *= 1.1;
        }
        power
    }

    /// A hole's active duration in seconds (`(10 − 3·id) × 1.3^upgrades`).
    pub fn black_hole_duration(&self, index: usize) -> f64 {
        let mut duration = (10.0 - index as f64 * 3.0)
            * 1.3f64.powi(self.black_holes.holes[index].duration_upgrades as i32);
        // Achievement 155 (13.7 billion years played): durations +10%.
        if self.achievement_unlocked(155) {
            duration *= 1.1;
        }
        duration
    }

    /// `isPermanent`: duty cycle ≥ 0.9999.
    pub fn black_hole_is_permanent(&self, index: usize) -> bool {
        let duration = self.black_hole_duration(index);
        duration / (self.black_hole_raw_interval(index) + duration) >= 0.9999
    }

    /// Whether the hole is *effectively* active (its own charge and every
    /// lower hole active; pause overrides).
    pub fn black_hole_is_active(&self, index: usize) -> bool {
        // Doomed (`Pelle.isDisabled("blackhole")`): the holes are disabled.
        if self.pelle_is_disabled("blackhole") {
            return false;
        }
        if self.black_holes.paused {
            return false;
        }
        (0..=index).all(|i| {
            let hole = &self.black_holes.holes[i];
            hole.unlocked && (hole.active || self.black_hole_is_permanent(i))
        })
    }

    /// Whether the first Black Hole can be unlocked (100 RM).
    pub fn can_unlock_black_hole(&self) -> bool {
        !self.black_holes.holes[0].unlocked && self.reality.machines >= BH1_UNLOCK_COST
    }

    /// Unlock the first Black Hole for 100 RM (`BlackHoles.unlock`).
    pub fn unlock_black_hole(&mut self) -> bool {
        if !self.can_unlock_black_hole() {
            return false;
        }
        self.reality.machines -= BH1_UNLOCK_COST;
        self.black_holes.holes[0].unlocked = true;
        self.records.time_played_at_bh_unlock_ms = self.records.total_time_played_ms;
        // BLACK_HOLE_UNLOCKED achievements (144).
        self.check_black_hole_unlocked_achievements();
        true
    }

    /// RU20's purchase effect: the second Black Hole.
    pub(crate) fn unlock_second_black_hole(&mut self) {
        if self.black_holes.holes[0].unlocked {
            self.black_holes.holes[1].unlocked = true;
        }
    }

    // --- Upgrades -------------------------------------------------------------------

    /// Upgrade costs (`getHybridCostScaling(amount, 1e30, base×mult_by_hole,
    /// costMult, 0.2, …)`), with base 15/20/10 and per-purchase ×3.5/×2/×4;
    /// the second hole's costs are ×1000.
    pub fn black_hole_upgrade_cost(&self, index: usize, kind: u8) -> Decimal {
        let hole = &self.black_holes.holes[index];
        let (amount, initial, mult) = match kind {
            0 => (hole.interval_upgrades, 15.0, 3.5),
            1 => (hole.power_upgrades, 20.0, 2.0),
            _ => (hole.duration_upgrades, 10.0, 4.0),
        };
        let scale = if index == 1 { 1000.0 } else { 1.0 };
        Decimal::from_float(crate::reality_upgrades::linear_cost_scaling(
            amount as f64,
            1e30,
            initial * scale,
            mult,
            0.2,
        ))
    }

    /// Buy a black hole upgrade (`kind`: 0 interval, 1 power, 2 duration),
    /// preserving the segment's progress fraction like the original.
    pub fn buy_black_hole_upgrade(&mut self, index: usize, kind: u8) -> bool {
        if index >= 2 || !self.black_holes.holes[index].unlocked {
            return false;
        }
        // An interval already at 0 (permanent) can't meaningfully upgrade,
        // but the original still allows purchases; keep it simple and allow.
        let cost = self.black_hole_upgrade_cost(index, kind);
        if self.reality.machines < cost {
            return false;
        }
        // Progress fraction before the values change.
        let hole = &self.black_holes.holes[index];
        let state_time = if hole.active {
            self.black_hole_duration(index)
        } else {
            self.black_hole_interval(index)
        };
        let progress = if state_time > 0.0 {
            hole.phase / state_time
        } else {
            0.0
        };

        self.reality.machines -= cost;
        let hole = &mut self.black_holes.holes[index];
        match kind {
            0 => hole.interval_upgrades += 1,
            1 => hole.power_upgrades += 1,
            _ => hole.duration_upgrades += 1,
        }

        // Restore the progress fraction under the new segment length.
        let new_state_time = if self.black_holes.holes[index].active {
            self.black_hole_duration(index)
        } else {
            self.black_hole_interval(index)
        };
        self.black_holes.holes[index].phase = progress * new_state_time;

        // A hole that becomes permanent while inactive would lock inactive.
        if self.black_hole_is_permanent(index) {
            self.black_holes.holes[index].active = true;
        }
        // BLACK_HOLE_UPGRADE_BOUGHT achievements (145, and 155/158's effects rely
        // on the unlock).
        self.check_black_hole_upgrade_achievements();
        true
    }

    /// Toggle the global pause (`BlackHoles.togglePause`). The IU24
    /// requirement-lock guard is out of frontier (armed req-locks unmodelled).
    pub fn toggle_black_hole_pause(&mut self) -> bool {
        if !self.black_holes.holes[0].unlocked {
            return false;
        }
        // Unpausing resets the slowest-inversion tracker (IU24's gate).
        if self.black_holes.paused {
            self.requirement_checks.reality_slowest_bh = 1.0;
        }
        self.black_holes.paused = !self.black_holes.paused;
        self.black_holes.pause_time_ms = self.records.real_time_played_ms;
        true
    }

    /// `BlackHoles.areNegative`: paused with an inversion strength below 1
    /// (disabled inside Enslaved's and Lai'tela's Realities).
    pub fn black_holes_are_negative(&self) -> bool {
        self.black_holes.paused
            && !self.celestials.enslaved.run
            && !self.celestials.laitela.run
            && self.black_holes.negative < 1.0
    }

    /// Set the inversion strength (`player.blackHoleNegative`, the charging
    /// slider): 10^−x for x in 0..=300. Weakening the inversion raises the
    /// slowest-inversion tracker along (`Math.max`).
    pub fn set_black_hole_negative(&mut self, negative: f64) {
        self.black_holes.negative = negative.clamp(1e-300, 1.0);
        self.requirement_checks.reality_slowest_bh = self
            .requirement_checks
            .reality_slowest_bh
            .max(self.black_holes.negative);
    }

    /// Whether the inversion slider is available (`isNegativeBHUnlocked`):
    /// V flipped (Ra's hard-V unlock) and both holes permanent.
    pub fn black_hole_negative_unlocked(&self) -> bool {
        self.ra_hard_v_unlocked()
            && self.black_holes.holes[0].unlocked
            && (0..2).all(|i| self.black_hole_is_permanent(i))
    }

    /// The unpause power ramp (`unpauseAccelerationFactor`): 0 → 1 over 5 s
    /// of real time after unpausing.
    fn black_hole_acceleration_factor(&self) -> f64 {
        if self
            .black_holes
            .holes
            .iter()
            .enumerate()
            .all(|(i, h)| !h.unlocked || self.black_hole_is_permanent(i))
        {
            return 1.0;
        }
        ((self.records.real_time_played_ms - self.black_holes.pause_time_ms)
            / (1000.0 * ACCELERATION_TIME_S))
            .clamp(0.0, 1.0)
    }

    /// The combined black-hole game-speed multiplier (the BLACK_HOLE branch
    /// of `getGameSpeedupFactor`).
    pub(crate) fn black_hole_speed_factor_impl(&self) -> f64 {
        // Inverted: the negative strength *slows* the game below 1.
        if self.black_holes_are_negative() {
            return self.black_holes.negative;
        }
        if self.black_holes.paused {
            return 1.0;
        }
        let mut factor = 1.0;
        for index in 0..2 {
            let hole = &self.black_holes.holes[index];
            if !hole.unlocked {
                break;
            }
            if !self.black_hole_is_active(index) {
                break;
            }
            factor *= self
                .black_hole_power(index)
                .powf(self.black_hole_acceleration_factor());
            // V's `achievementBH` reward: the achievement multiplier boosts
            // each active hole.
            factor *= self.v_achievement_bh_effect();
        }
        factor
    }

    /// Advance the black-hole phases by `real_dt_ms` (`BlackHoles
    /// .updatePhases`): hole 1 runs on real time; hole 2's phase advances only
    /// while hole 1 is active.
    pub(crate) fn tick_black_holes(&mut self, real_dt_ms: f64) {
        if !self.black_holes.holes[0].unlocked || self.black_holes.paused {
            return;
        }
        // Auto-pause (`autoPauseData`): if the configured pause point falls
        // within this tick, advance only up to it and pause.
        let raw_dt_s = real_dt_ms / 1000.0;
        let (auto_pause, dt_s) = self.black_hole_auto_pause_data(raw_dt_s);
        // Active periods cascade: [real time, BH1-active time].
        let bh1_active_time = self.black_hole_active_time(0, dt_s);
        self.advance_black_hole_phase(0, dt_s);
        if self.black_holes.holes[1].unlocked {
            self.advance_black_hole_phase(1, bh1_active_time);
        }
        if auto_pause {
            self.toggle_black_hole_pause();
        }
    }

    /// `BlackHoles.autoPauseData(realTime)`: whether the auto-pause fires
    /// within `real_time_s`, and the (possibly shortened) time to advance.
    fn black_hole_auto_pause_data(&self, real_time_s: f64) -> (bool, f64) {
        if self.black_holes.auto_pause_mode == 0 {
            return (false, real_time_s);
        }
        let Some(time_left) =
            self.black_hole_time_to_next_pause(self.black_holes.auto_pause_mode)
        else {
            return (false, real_time_s);
        };
        if time_left < 1e-9 || time_left > real_time_s {
            return (false, real_time_s);
        }
        (true, time_left)
    }

    /// `BlackHoles.timeToNextPause(bhNum)` in seconds: how long until the
    /// pause point `ACCELERATION_TIME` before the watched hole's next
    /// activation (None = never pauses under the current cycles).
    fn black_hole_time_to_next_pause(&self, bh_num: u8) -> Option<f64> {
        if bh_num == 1 {
            let hole = &self.black_holes.holes[0];
            let duration = self.black_hole_duration(0);
            let interval = self.black_hole_interval(0);
            // If no gap is as long as the warmup time, never pause.
            if interval <= ACCELERATION_TIME_S {
                return None;
            }
            let t = if hole.active {
                duration - hole.phase + interval
            } else {
                interval - hole.phase
            };
            return Some(if t < ACCELERATION_TIME_S {
                t + duration + interval - ACCELERATION_TIME_S
            } else {
                t - ACCELERATION_TIME_S
            });
        }
        // BH2: scan the next 100 hole transitions (the original's bounded
        // simulation), looking for a BH2 activation preceded by enough
        // inactive time to fit the warmup.
        let mut charged = [
            self.black_holes.holes[0].active,
            self.black_holes.holes[1].active,
        ];
        let mut phases = [
            self.black_holes.holes[0].phase,
            self.black_holes.holes[1].phase,
        ];
        let durations = [self.black_hole_duration(0), self.black_hole_duration(1)];
        let intervals = [self.black_hole_interval(0), self.black_hole_interval(1)];
        if intervals[0] <= ACCELERATION_TIME_S && intervals[1] <= ACCELERATION_TIME_S {
            return None;
        }
        let phase_bound_list = [
            [intervals[0], f64::INFINITY],
            [durations[0], intervals[1]],
            [durations[0], durations[1]],
        ];
        let mut inactive_time = 0.0;
        let mut total_time = 0.0;
        for _ in 0..100 {
            let current = if charged[0] {
                if charged[1] {
                    2
                } else {
                    1
                }
            } else {
                0
            };
            let bounds = &phase_bound_list[current];
            let min_time = if current > 0 {
                (bounds[0] - phases[0]).min(bounds[1] - phases[1])
            } else {
                bounds[0] - phases[0]
            };
            if current == 2 {
                if inactive_time >= ACCELERATION_TIME_S {
                    return Some(total_time - ACCELERATION_TIME_S);
                }
                inactive_time = 0.0;
            } else {
                inactive_time += min_time;
            }
            total_time += min_time;
            if current > 0 {
                phases[1] += min_time;
                if phases[1] >= bounds[1] {
                    charged[1] = !charged[1];
                    phases[1] -= bounds[1];
                }
            }
            phases[0] += min_time;
            if phases[0] >= bounds[0] {
                charged[0] = !charged[0];
                phases[0] -= bounds[0];
            }
        }
        None
    }

    /// `realTimeWhileActive`: of `time` seconds passing for this hole, how
    /// many are spent with it active.
    fn black_hole_active_time(&self, index: usize, time: f64) -> f64 {
        if self.black_hole_is_permanent(index) {
            return time;
        }
        let hole = &self.black_holes.holes[index];
        let duration = self.black_hole_duration(index);
        let interval = self.black_hole_interval(index);
        let cycle = duration + interval;
        // Time until the next active→inactive transition.
        let next_deactivation = if hole.active {
            duration - hole.phase
        } else {
            cycle - hole.phase
        };
        let current_active = next_deactivation.min(time).min(duration).max(0.0);
        if time <= next_deactivation {
            // Only (part of) the current activation applies.
            return if hole.active { current_active } else { 0.0 };
        }
        let current = if hole.active { current_active } else { 0.0 };
        let after = time - next_deactivation;
        let full_cycles = (after / cycle).floor();
        let partial = after - full_cycles * cycle;
        current + full_cycles * duration + (partial - interval).max(0.0)
    }

    /// `updatePhase` for one hole: advance its phase by `active_period`
    /// seconds, flipping segments as thresholds pass.
    fn advance_black_hole_phase(&mut self, index: usize, active_period: f64) {
        if self.black_hole_is_permanent(index) {
            self.black_holes.holes[index].active = true;
            return;
        }
        let duration = self.black_hole_duration(index);
        let interval = self.black_hole_interval(index);
        let cycle = duration + interval;
        let hole = &mut self.black_holes.holes[index];
        hole.phase += active_period;
        if hole.phase >= cycle {
            hole.activations += (hole.phase / cycle).floor() as u32;
            hole.phase %= cycle;
        }
        if hole.active {
            if hole.phase >= duration {
                hole.phase -= duration;
                hole.active = false;
            }
        } else if hole.phase >= interval {
            hole.phase -= interval;
            hole.activations += 1;
            hole.active = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bh_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game.reality.machines = Decimal::from_float(1e6);
        game.unlock_black_hole();
        game
    }

    #[test]
    fn inversion_slows_the_game_while_paused() {
        let mut game = bh_game();
        game.unlock_black_hole();
        game.set_black_hole_negative(1e-10);
        // Not paused: no inversion.
        assert!(!game.black_holes_are_negative());
        game.toggle_black_hole_pause();
        assert!(game.black_holes_are_negative());
        assert_eq!(game.black_hole_speed_factor_impl(), 1e-10);
        // The slowest-inversion tracker follows the strength; unpausing
        // resets it.
        assert_eq!(game.requirement_checks.reality_slowest_bh, 1.0);
        game.set_black_hole_negative(1e-5);
        assert_eq!(game.requirement_checks.reality_slowest_bh, 1.0);
        game.toggle_black_hole_pause();
        assert_eq!(game.requirement_checks.reality_slowest_bh, 1.0);
        assert!(!game.black_holes_are_negative());
    }

    #[test]
    fn auto_pause_stops_before_activation() {
        let mut game = bh_game();
        game.unlock_black_hole();
        game.black_holes.auto_pause_mode = 1;
        // BH1: interval 3600 s, duration 10 s. The pause point sits 5 s before
        // the first activation (3595 s in). One big tick crosses it.
        game.tick(3_600_000.0);
        assert!(game.black_holes.paused);
        // The phase stopped at the pause point, not the activation.
        assert!(!game.black_holes.holes[0].active);
        assert!((game.black_holes.holes[0].phase - 3595.0).abs() < 1e-6);
    }

    #[test]
    fn unlock_costs_100_rm() {
        let mut game = GameState::new();
        game.reality.machines = Decimal::from_float(99.0);
        assert!(!game.unlock_black_hole());
        game.reality.machines = Decimal::from_float(150.0);
        assert!(game.unlock_black_hole());
        assert_eq!(game.reality.machines, Decimal::from_float(50.0));
        assert!(game.black_holes.holes[0].unlocked);
        assert_eq!(game.records.time_played_at_bh_unlock_ms, 0.0);
    }

    #[test]
    fn base_values_match_original() {
        let game = bh_game();
        assert_eq!(game.black_hole_interval(0), 3600.0);
        assert_eq!(game.black_hole_power(0), 180.0);
        assert_eq!(game.black_hole_duration(0), 10.0);
        assert_eq!(game.black_hole_interval(1), 360.0);
        assert_eq!(game.black_hole_power(1), 90.0);
        assert_eq!(game.black_hole_duration(1), 7.0);
    }

    #[test]
    fn upgrades_cost_rm_and_scale() {
        let mut game = bh_game();
        assert_eq!(
            game.black_hole_upgrade_cost(0, 0),
            Decimal::from_float(15.0)
        );
        assert!(game.buy_black_hole_upgrade(0, 0));
        assert!((game.black_hole_interval(0) - 2880.0).abs() < 1e-9);
        // The original's cost helper ceils: ceil(15 × 3.5) = 53.
        assert_eq!(
            game.black_hole_upgrade_cost(0, 0),
            Decimal::from_float(53.0)
        );
        assert!(game.buy_black_hole_upgrade(0, 1));
        assert!((game.black_hole_power(0) - 243.0).abs() < 1e-9);
        assert!(game.buy_black_hole_upgrade(0, 2));
        assert!((game.black_hole_duration(0) - 13.0).abs() < 1e-9);
    }

    #[test]
    fn hole_cycles_between_inactive_and_active() {
        let mut game = bh_game();
        // Past the 5 s unpause ramp (the factor reads realTimePlayed).
        game.records.real_time_played_ms = 1e7;
        // 3600 s inactive, then 10 s active.
        game.tick_black_holes(3599_000.0);
        assert!(!game.black_holes.holes[0].active);
        game.tick_black_holes(2_000.0);
        assert!(game.black_holes.holes[0].active);
        assert!(game.black_hole_is_active(0));
        assert!(game.game_speed_factor() > 1.0);
        game.tick_black_holes(10_000.0);
        assert!(!game.black_holes.holes[0].active);
        assert_eq!(game.black_holes.holes[0].activations, 1);
    }

    #[test]
    fn speed_factor_uses_power_when_active() {
        let mut game = bh_game();
        game.black_holes.holes[0].active = true;
        // Past the 5 s ramp.
        game.records.real_time_played_ms = 100_000.0;
        assert_eq!(game.game_speed_factor(), 180.0);
        // Pause disables it.
        assert!(game.toggle_black_hole_pause());
        assert_eq!(game.game_speed_factor(), 1.0);
        assert!(game.toggle_black_hole_pause());
        // Freshly unpaused: the ramp starts at ^0 = 1.
        assert_eq!(game.game_speed_factor(), 1.0);
        game.records.real_time_played_ms += 2_500.0;
        let mid = game.black_hole_speed_factor_impl();
        assert!((mid - 180f64.powf(0.5)).abs() < 1e-9);
    }

    #[test]
    fn second_hole_advances_only_while_first_active() {
        let mut game = bh_game();
        game.black_holes.holes[1].unlocked = true;
        // 3600 s: BH1 fully inactive → BH2 phase untouched.
        game.tick_black_holes(3600_000.0);
        assert_eq!(game.black_holes.holes[1].phase, 0.0);
        // BH1 now active for 10 s → BH2 phase advances by exactly that.
        game.tick_black_holes(10_000.0);
        assert!((game.black_holes.holes[1].phase - 10.0).abs() < 1e-9);
        // BH2 needs 360 s of BH1-active time to fire; not yet.
        assert!(!game.black_holes.holes[1].active);
    }

    #[test]
    fn second_hole_speed_multiplies_first() {
        let mut game = bh_game();
        game.black_holes.holes[1].unlocked = true;
        game.black_holes.holes[0].active = true;
        game.black_holes.holes[1].active = true;
        game.records.real_time_played_ms = 100_000.0;
        assert_eq!(game.game_speed_factor(), 180.0 * 90.0);
        // BH2 alone does nothing if BH1 is inactive.
        game.black_holes.holes[0].active = false;
        assert_eq!(game.game_speed_factor(), 1.0);
    }
}
