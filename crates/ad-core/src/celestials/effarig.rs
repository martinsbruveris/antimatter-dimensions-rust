//! Effarig (Feature 7.2) — a multi-stage Reality with a Relic-Shard economy and
//! a glyph forge. See `docs/design/2026-07-06-celestials.md` §2.
//!
//! **Work in progress:** the state block + run flag land here so save/load and
//! the shared celestial machinery compile; the full logic (Relic Shards, the
//! 3-stage run nerfs, glyph level cap, the Effarig glyph type) is filled in by
//! its own task.

use crate::state::GameState;

/// `player.celestials.effarig`.
#[derive(Debug, Clone, Default)]
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
    /// Whether Effarig's Reality is unlocked (the `run` unlock, id 3). Stub
    /// until the Effarig task wires the Relic-Shard purchase.
    pub fn effarig_run_unlocked(&self) -> bool {
        self.celestials.effarig.unlock_bought(3)
    }

    /// The Effarig completion hook from `giveRealityRewards`. Stub until the
    /// stage machinery lands.
    pub(crate) fn effarig_complete_stage(&mut self) {}

    /// `giveRealityRewards`: add the run's Relic Shards. Stub until the shard
    /// formula + Teresa-effarig gate lands.
    pub(crate) fn effarig_gain_relic_shards(&mut self) {}
}
