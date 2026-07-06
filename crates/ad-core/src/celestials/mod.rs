//! Celestials (Phase 7) — the endgame encounters unlocked after the first
//! Reality. Each adds a permanently-persistent state block, a special "Reality"
//! run under modified rules, and progressive unlocks. See
//! `docs/design/2026-07-06-celestials.md`.
//!
//! This module owns [`CelestialsState`] (`player.celestials`) and the shared
//! run machinery: the mutually-exclusive per-celestial `run` flags, entering a
//! celestial reality (a reward-free Reality that sets one run flag), and the
//! `is_in_celestial_reality` guard. The per-celestial logic lives in the
//! submodules.

use crate::state::GameState;

pub mod effarig;
pub mod enslaved;
pub mod teresa;
pub mod v;

pub use effarig::EffarigState;
pub use enslaved::EnslavedState;
pub use teresa::TeresaState;
pub use v::VState;

/// Which celestial a reality run belongs to. Ordered as in the original
/// navigation (`Celestials` object) so ids line up with the enter-modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Celestial {
    Teresa,
    Effarig,
    Enslaved,
    V,
}

/// `player.celestials` — one sub-struct per implemented celestial. Ra, Lai'tela
/// and Pelle (celestials 5–7) are out of frontier; their save blocks are kept
/// opaque by the DTO layer so real saves round-trip.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CelestialsState {
    #[cfg_attr(feature = "serde", serde(default))]
    pub teresa: TeresaState,
    #[cfg_attr(feature = "serde", serde(default))]
    pub effarig: EffarigState,
    #[cfg_attr(feature = "serde", serde(default))]
    pub enslaved: EnslavedState,
    #[cfg_attr(feature = "serde", serde(default))]
    pub v: VState,
}

impl CelestialsState {
    pub fn new() -> Self {
        Self {
            teresa: TeresaState::new(),
            effarig: EffarigState::new(),
            enslaved: EnslavedState::new(),
            v: VState::new(),
        }
    }
}

impl GameState {
    /// `isInCelestialReality()`: any celestial run flag is set.
    pub fn is_in_celestial_reality(&self) -> bool {
        self.celestials.teresa.run
            || self.celestials.effarig.run
            || self.celestials.enslaved.run
            || self.celestials.v.run
    }

    /// `clearCelestialRuns()`: clear every celestial run flag. Called at the
    /// start of every Reality reset (a celestial reality is mutually exclusive
    /// with the others and with normal play).
    pub(crate) fn clear_celestial_runs(&mut self) {
        self.celestials.teresa.run = false;
        self.celestials.effarig.run = false;
        self.celestials.enslaved.run = false;
        self.celestials.v.run = false;
    }

    /// Whether the Celestials tab / features are available. The original gates
    /// Teresa on Achievement 147 (first Reality); we gate on `reality_unlocked`
    /// since our achievement grid is display-only past the early rows (see the
    /// design doc §5).
    pub fn celestials_unlocked(&self) -> bool {
        self.reality_unlocked()
    }

    /// Whether the given celestial's Reality can currently be entered: the tab
    /// is unlocked, its run is unlocked, and we are not already mid-entry.
    pub fn can_start_celestial_reality(&self, cel: Celestial) -> bool {
        if !self.celestials_unlocked() {
            return false;
        }
        match cel {
            Celestial::Teresa => self.teresa_run_unlocked(),
            Celestial::Effarig => self.effarig_run_unlocked(),
            Celestial::Enslaved => self.enslaved_run_unlocked(),
            Celestial::V => self.v_celestial_unlocked(),
        }
    }

    /// Enter a celestial's Reality (`EnterCelestialsModal` confirm →
    /// `beginProcessReality(getRealityProps(true))` + `initializeRun`): a
    /// reward-free Reality reset that then sets the run flag. Returns whether it
    /// happened.
    pub fn start_celestial_reality(&mut self, cel: Celestial) -> bool {
        if !self.can_start_celestial_reality(cel) {
            return false;
        }
        // The reward-free reset clears every run flag (via
        // `clear_celestial_runs` inside `reality_reset_internal`).
        self.reset_reality();
        match cel {
            Celestial::Teresa => self.celestials.teresa.run = true,
            Celestial::Effarig => {
                // Glyph effects are computed on demand in our engine, so the
                // original's `recalculateAllGlyphs()` (needed there for the
                // cached level cap) has no analogue.
                self.celestials.effarig.run = true;
            }
            Celestial::Enslaved => self.celestials.enslaved.run = true,
            Celestial::V => self.celestials.v.run = true,
        }
        true
    }

    /// The per-celestial completion hooks from `giveRealityRewards` — run on a
    /// *rewarded* Reality (not a forced reset or a celestial-swap), before the
    /// reset zeroes the run flags. Called from `finish_process_reality`.
    pub(crate) fn celestial_reality_completion_hooks(&mut self) {
        if self.celestials.teresa.run {
            self.teresa_complete_run();
        }
        if self.celestials.effarig.run {
            self.effarig_complete_stage();
        }
        if self.celestials.enslaved.run {
            self.enslaved_complete_run();
        }
        // V's completion is handled continuously by `v_check_run_unlocks` each
        // tick; a rewarded Reality just exits the run.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use break_infinity::Decimal;

    fn realitied_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game
    }

    #[test]
    fn entering_teresa_reality_sets_the_run_flag_and_clears_others() {
        let mut game = realitied_game();
        game.celestials.teresa.unlock_bits |= 1; // run unlock (id 0)
        game.celestials.v.run = true; // pretend another run was active
        assert!(game.start_celestial_reality(Celestial::Teresa));
        assert!(game.celestials.teresa.run);
        assert!(!game.celestials.v.run);
        assert!(game.is_in_celestial_reality());
    }

    #[test]
    fn teresa_run_softens_ip_gain() {
        let mut game = realitied_game();
        game.broke_infinity = true;
        game.records.this_infinity.max_am = Decimal::new(1.0, 4000);
        let normal = game.gained_infinity_points();
        game.celestials.teresa.run = true;
        let softened = game.gained_infinity_points();
        // `^0.55` shrinks a large IP value.
        assert!(softened < normal);
    }

    #[test]
    fn a_normal_reality_exits_a_celestial_run() {
        let mut game = realitied_game();
        game.celestials.teresa.run = true;
        game.reset_reality();
        assert!(!game.celestials.teresa.run);
        assert!(!game.is_in_celestial_reality());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn celestials_round_trip_through_save() {
        let mut game = realitied_game();
        game.celestials.teresa.poured_amount = 1e20;
        game.celestials.teresa.unlock_bits = 0b111;
        game.celestials.teresa.best_run_am = Decimal::new(1.0, 500);
        game.celestials.teresa.perk_shop = [3, 5, 1, 2, 0];
        game.celestials.effarig.relic_shards = 1234.5;
        game.celestials.enslaved.stored = 9.9e8;
        game.celestials.enslaved.completed = true;
        game.celestials.enslaved.unlock_bits = 0b11;
        game.celestials.v.unlock_bits = 0b101;
        game.celestials.v.run_unlocks = [1, 2, 0, 0, 0, 3, 0, 0, 0];
        game.celestials.v.goal_reduction_steps = [0, 100, 0, 0, 0, 0, 0, 0, 0];
        game.celestials.v.st_spent = 4;
        game.celestials.v.run_records[4] = 9500.0;

        let encoded = crate::save::encode_save(&game, 0);
        let decoded = crate::save::decode_save(&encoded).expect("decode");

        assert_eq!(decoded.celestials.teresa.poured_amount, 1e20);
        assert_eq!(decoded.celestials.teresa.unlock_bits, 0b111);
        assert_eq!(
            decoded.celestials.teresa.best_run_am,
            Decimal::new(1.0, 500)
        );
        assert_eq!(decoded.celestials.teresa.perk_shop, [3, 5, 1, 2, 0]);
        assert_eq!(decoded.celestials.effarig.relic_shards, 1234.5);
        assert_eq!(decoded.celestials.enslaved.stored, 9.9e8);
        assert!(decoded.celestials.enslaved.completed);
        assert_eq!(decoded.celestials.enslaved.unlock_bits, 0b11);
        assert_eq!(decoded.celestials.v.unlock_bits, 0b101);
        assert_eq!(
            decoded.celestials.v.run_unlocks,
            [1, 2, 0, 0, 0, 3, 0, 0, 0]
        );
        assert_eq!(
            decoded.celestials.v.goal_reduction_steps,
            [0, 100, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(decoded.celestials.v.st_spent, 4);
        assert_eq!(decoded.celestials.v.run_records[4], 9500.0);
    }
}
