// Tutorial-highlight helpers. The engine owns the state machine
// (`player.tutorialState` / `tutorialActive`, ported in ad-core's tutorial.rs)
// and ships the two raw fields in the snapshot; the UI only decides where to
// draw the gold glow + yellow `!`. Mirrors `Tutorial.isActive` /
// `Tutorial.emphasizeH2P` from the original `core/tutorial.js`.

// Step ids, matching the engine's `tutorial::state` constants.
export const TUTORIAL_STATE = {
  DIM1: 0,
  DIM2: 1,
  TICKSPEED: 2,
  DIMBOOST: 3,
  GALAXY: 4,
};

// Whether `step` is the active tutorial highlight in the current snapshot
// (`Tutorial.isActive`, minus the always-true `fullGameCompletions === 0`).
export function hasTutorial(snapshot, step) {
  return Boolean(snapshot) && snapshot.tutorial_active && snapshot.tutorial_state === step;
}

// Whether the How-To-Play link should pulse: until the first Dimension Boost
// (`Tutorial.emphasizeH2P`), i.e. still at/below the DIMBOOST step and no boost
// bought yet.
export function emphasizeH2P(snapshot) {
  return (
    Boolean(snapshot) &&
    snapshot.dim_boosts === 0 &&
    snapshot.tutorial_state <= TUTORIAL_STATE.DIMBOOST
  );
}
