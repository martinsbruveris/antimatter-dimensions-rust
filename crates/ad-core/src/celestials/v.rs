//! V (Feature 7.4) — the Celestial of Achievements: hard goals in constrained
//! runs, Space Theorems, and ST-gated rewards. See
//! `docs/design/2026-07-06-celestials.md` §4.
//!
//! **Work in progress:** the state block + run flag land here so save/load and
//! the shared celestial machinery compile; the full main-unlock conditions, the
//! 9 V-achievements, Space Theorems, and the run modifiers are filled in by its
//! own task.

use crate::state::GameState;

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
    /// numbers in the save — log10s / counts). The id-0 record starts at `-10`
    /// (glyph count is stored negated).
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
}

impl GameState {
    /// Whether V (the celestial) is unlocked — bit 0 of `unlockBits`. Stub until
    /// the V task wires the six main-unlock conditions.
    pub fn v_celestial_unlocked(&self) -> bool {
        self.celestials.v.unlock_bits & 1 != 0
    }
}
