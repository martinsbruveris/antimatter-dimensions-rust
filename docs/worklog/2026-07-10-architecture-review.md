---
date: 2026-07-10
feature: architecture-review
design_docs:
  - ../design/2026-06-27-simulation-architecture.md
---

# Architecture review — ad-core, ad-gui, and the Actions question

## Summary
An in-depth review of `ad-core`, `ad-gui`, and their boundary, centred on
whether to complete the half-done "route all UI actions through the `Action`
framework" unification. Verdict: the design is fundamentally clean; the
unification should **not** be completed — the docs were updated to state the
decided design instead. Two of the review's low-risk cleanups were applied in
the same session (doc truth-up + a shared frontend Num-math util).

## Review findings (condensed)
- **Core principles hold.** Dependency graph intact, engine pure (no IO / wall
  clocks), config immutable, frontend a thin shell with only minor leaks. The
  "one `GameState`, 51 `impl` blocks across mechanic modules" shape mirrors
  the original JS game — right for a fidelity port.
- **Actions framework: keep as-is, don't unify.** The decisive argument came
  from the fidelity work: the original game gives the "same" action different
  per-caller semantics (manual `manualRequestGalaxyReset` vs the autobuyer's
  `requestGalaxyReset(bulk, limit)` — the cap applies to the purchase, not
  the phase reset; ditto Dim Boost buy-max and `buyMaxTickSpeed`). A unified
  vocabulary would need per-caller variants (not "unified") or would flatten
  those distinctions (breaking fidelity). The shared mutation interface *is*
  the `GameState` method surface; `Action` is the serializable adapter for
  action-as-data consumers (`ad-sim`, Python), grown demand-driven with the
  simulation frontier. Also: the GUI's 179 commands are already thin
  one-liners, ~half aren't game actions at all (options/saves/editor ops),
  and `ActionOutcome` is too narrow for command return values.
- **Drift found:** `crates/ad-core/ARCHITECTURE.md` claimed `apply_action` is
  "the single mutation seam every action producer (GUI, autobuyers,
  simulation) routes through" — only `ad-sim` uses it (2 call sites);
  `observed.rs`'s entry claimed the GUI consumes it (it doesn't).
- **ad-sim is a planned future feature, not active:** integration tests
  disabled (`autotests = false`), `ObservedState`/`Action` frozen at the
  pre-Infinity frontier. Decision: keep the code, mark the status in the
  docs; revive later by extending `ObservedState`/`Action` one prestige
  layer at a time.
- **`ad-gui/src/main.rs` is a 5,364-line monolith** (~3,000 view structs +
  builders, ~2,200 commands). A pure-move split into `views`/`commands`
  modules is the highest-payoff refactor; deferred for now (not requested).
- **Minor:** duplicated `{ m, e }` helpers across 5+ frontend components
  (fixed this session); a few thin-shell leaks (e.g. `EternityButton`'s
  visibility rule computed frontend-side) and GUI commands poking engine
  flags directly — cosmetic, fix opportunistically.

## What shipped
- **Doc truth-up:** `action.rs` module docs now state the decided design
  (sim/Python vocabulary over per-caller engine methods, grown per prestige
  layer, explicitly not a universal seam — with the fidelity rationale);
  `crates/ad-core/ARCHITECTURE.md`, `docs/ARCHITECTURE.md`, and `AGENTS.md`
  match. `ad-sim` marked "planned future feature, not under active
  development" in its `ARCHITECTURE.md`, the workspace crate table, and
  `AGENTS.md` (whose `cargo test -p ad-sim` line ran 0 tests).
- **`frontend/src/util/num.js`:** shared `numLog10` / `gtZero` /
  `normalizeNum` / `scaleNum` / `floatToNum` / `numFromLog10` /
  `averageNums`; replaced the hand-copied variants in `EternityButton`,
  `HeaderBigCrunchButton`, `TeresaTab`, `PastPrestigeRunsContainer`,
  `StatisticsTab`, and `matterScale.js`.

## Decisions & why
- One semantic unification in `num.js`: the buttons' local `log10` returned
  `0` for non-positive values, the other copies `-Infinity`. All button call
  sites render identically under `-Infinity` (threshold comparisons and the
  `< 0.9` red branch of the gain coloring), so the shared helper keeps the
  mathematically-correct `-Infinity` form.

## Surprises & gotchas
- The original simulation design doc (§B) envisioned autobuyers and the
  Automator emitting `Action`s; both were implemented as direct in-engine
  callers instead — and the fidelity work vindicated that deviation (see
  above). The doc stays historical; the living docs now carry the decision.

## Follow-ups
- Split `ad-gui/src/main.rs` (mechanical move, ~zero fidelity risk).
- Opportunistic: move frontend-computed rules (EternityButton visibility)
  into the snapshot; engine methods for the milestone-autobuyer toggles.
- ad-sim revival as its own project (extend `ObservedState`/`Action` per
  layer, re-enable tests) when simulation work resumes.

## Tests
- `cargo build -p ad-core -p ad-sim` clean; `cargo test -p ad-core
  --features serde` + `-p ad-gui` all pass (docs-only Rust changes).
- Frontend `npm run build` clean; app boots without errors (Num-util change
  is render-identical by inspection of every call site).
