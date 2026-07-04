//! Eternity Challenges (Feature 4.5) — state and study-slot plumbing.
//!
//! This module currently carries the *state* the Time Studies tree needs (the
//! held EC study slot, per-EC completion counts, and the EC studies' TT
//! costs). The challenge run logic (start/goal/restrictions/rewards) lands
//! with Feature 4.5.

use crate::state::GameState;

/// Number of Eternity Challenges.
pub const ETERNITY_CHALLENGE_COUNT: usize = 12;

/// Max completions per challenge.
pub const EC_MAX_COMPLETIONS: u8 = 5;

/// TT cost of each EC's unlock study (`ec-time-studies.js`), 1-indexed via
/// [`ec_study_cost`].
const EC_STUDY_COSTS: [f64; ETERNITY_CHALLENGE_COUNT] = [
    30.0, 35.0, 40.0, 70.0, 130.0, 85.0, 115.0, 115.0, 415.0, 550.0, 1.0, 1.0,
];

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
    pub fn eternity_challenge_completions(&self, id: u8) -> u8 {
        if (1..=ETERNITY_CHALLENGE_COUNT as u8).contains(&id) {
            self.eternity_challenges[(id - 1) as usize]
        } else {
            0
        }
    }
}
