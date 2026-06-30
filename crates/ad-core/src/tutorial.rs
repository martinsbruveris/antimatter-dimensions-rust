//! Tutorial highlight state machine — the gold glow + yellow `!` icon that
//! draws the new player's eye to the one element they should interact with next.
//!
//! Ported from the original `core/tutorial.js`. Two persisted fields drive it
//! ([`GameState::tutorial_state`] and [`GameState::tutorial_active`]); the
//! frontend reads them and renders the highlight. The machine advances either
//! passively — when the *next* step's condition becomes true (checked once per
//! [`tick`](GameState::tick)) — or by the player performing the highlighted
//! action (boost/galaxy/tickspeed), which clears the current glow via
//! [`tutorial_turn_off`](GameState::tutorial_turn_off).
//!
//! The original's `Tutorial.isActive` also gates on `fullGameCompletions === 0`,
//! which is always true pre-Infinity, so that term is dropped here. The machine
//! stops at [`GALAXY`](self::state::GALAXY): the AUTOMATOR step is out of scope,
//! so its condition never fires.

use break_infinity::Decimal;

use crate::state::GameState;

/// Tutorial step identifiers, mirroring the original `TUTORIAL_STATE`.
pub mod state {
    /// Highlight the 1st Antimatter Dimension buy button.
    pub const DIM1: u8 = 0;
    /// Highlight the 2nd Antimatter Dimension buy button.
    pub const DIM2: u8 = 1;
    /// Highlight the Tickspeed buy button.
    pub const TICKSPEED: u8 = 2;
    /// Highlight the Dimension Boost button.
    pub const DIMBOOST: u8 = 3;
    /// Highlight the Antimatter Galaxy button.
    pub const GALAXY: u8 = 4;
    /// Automator — out of scope pre-Infinity; the machine never reaches it.
    pub const AUTOMATOR: u8 = 5;
}

impl GameState {
    /// Whether tutorial step `step` is the active highlight right now. Mirrors
    /// `Tutorial.isActive` minus the always-true `fullGameCompletions === 0`
    /// term. The frontend calls this (via the snapshot) to decide where to draw
    /// the glow + `!`.
    pub fn tutorial_active_at(&self, step: u8) -> bool {
        self.tutorial_state == step && self.tutorial_active
    }

    /// The condition under which the machine enters `step`. These are the
    /// original's `tutorialStates[step].condition` predicates; the machine
    /// advances when the *next* step's condition holds. AUTOMATOR and beyond are
    /// out of scope and never fire.
    fn tutorial_step_condition(&self, step: u8) -> bool {
        match step {
            state::DIM1 => true,
            state::DIM2 => self.antimatter >= Decimal::from_float(100.0),
            state::TICKSPEED => self.dimensions[1].bought > 0,
            state::DIMBOOST => self.dimensions[3].amount >= Decimal::from_float(20.0),
            state::GALAXY => self.dimensions[7].amount >= Decimal::from_float(80.0),
            _ => false,
        }
    }

    /// Advance to the next step if its condition is met. Mirrors
    /// `Tutorial.tutorialLoop`; called once at the end of [`tick`](Self::tick).
    pub(crate) fn tutorial_loop(&mut self) {
        let next = self.tutorial_state + 1;
        if self.tutorial_step_condition(next) {
            self.tutorial_move_on(self.tutorial_state);
        }
    }

    /// Move on from `from` to the next step (only if `from` is still current),
    /// re-arming the highlight. Mirrors `Tutorial.moveOn`.
    fn tutorial_move_on(&mut self, from: u8) {
        if from != self.tutorial_state {
            return;
        }
        self.tutorial_state += 1;
        self.tutorial_active = true;
    }

    /// Turn off the current step's highlight because the player performed the
    /// highlighted action. Mirrors `Tutorial.turnOffEffect`: clears the glow for
    /// `from` (if it is the current step), then immediately re-runs
    /// [`tutorial_loop`](Self::tutorial_loop) so chained progress in one action
    /// (e.g. buying the 2nd dimension and tickspeed together) advances at once.
    pub(crate) fn tutorial_turn_off(&mut self, from: u8) {
        if from != self.tutorial_state {
            return;
        }
        self.tutorial_active = false;
        self.tutorial_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::state::*;
    use super::*;

    #[test]
    fn starts_at_dim1_active() {
        let game = GameState::new();
        assert_eq!(game.tutorial_state, DIM1);
        assert!(game.tutorial_active);
        assert!(game.tutorial_active_at(DIM1));
        assert!(!game.tutorial_active_at(DIM2));
    }

    #[test]
    fn dim1_advances_to_dim2_at_100_antimatter() {
        let mut game = GameState::new();
        game.antimatter = Decimal::from_float(99.0);
        game.tutorial_loop();
        assert_eq!(game.tutorial_state, DIM1);

        game.antimatter = Decimal::from_float(100.0);
        game.tutorial_loop();
        assert_eq!(game.tutorial_state, DIM2);
        assert!(game.tutorial_active_at(DIM2));
    }

    #[test]
    fn advances_to_tickspeed_when_second_dimension_bought() {
        let mut game = GameState::new();
        game.tutorial_state = DIM2;
        game.dimensions[1].bought = 1;
        game.tutorial_loop();
        assert_eq!(game.tutorial_state, TICKSPEED);
    }

    #[test]
    fn buying_tickspeed_turns_off_then_chains() {
        // At TICKSPEED, turning off clears the glow; if the DIMBOOST condition
        // (20 of the 4th dimension) is already met, it advances immediately.
        let mut game = GameState::new();
        game.tutorial_state = TICKSPEED;
        game.dimensions[3].amount = Decimal::from_float(20.0);
        game.tutorial_turn_off(TICKSPEED);
        assert_eq!(game.tutorial_state, DIMBOOST);
        assert!(game.tutorial_active_at(DIMBOOST));
    }

    #[test]
    fn turn_off_without_chain_just_clears_glow() {
        let mut game = GameState::new();
        game.tutorial_state = TICKSPEED;
        // DIMBOOST condition not met → stays at TICKSPEED but inactive.
        game.tutorial_turn_off(TICKSPEED);
        assert_eq!(game.tutorial_state, TICKSPEED);
        assert!(!game.tutorial_active);
        assert!(!game.tutorial_active_at(TICKSPEED));
    }

    #[test]
    fn turn_off_for_wrong_state_is_noop() {
        let mut game = GameState::new();
        game.tutorial_state = GALAXY;
        game.tutorial_turn_off(DIMBOOST);
        assert_eq!(game.tutorial_state, GALAXY);
        assert!(game.tutorial_active);
    }

    #[test]
    fn machine_stops_at_galaxy() {
        let mut game = GameState::new();
        game.tutorial_state = GALAXY;
        // The AUTOMATOR condition never fires, so GALAXY is terminal.
        game.tutorial_loop();
        assert_eq!(game.tutorial_state, GALAXY);
        assert!(!game.tutorial_active_at(AUTOMATOR));
    }
}
