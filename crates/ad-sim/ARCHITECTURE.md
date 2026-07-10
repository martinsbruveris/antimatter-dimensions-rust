# ad-sim architecture

`ad-sim` is the simulation driver — it decides *what a player does*. It has a
one-way dependency on `ad-core` and mutates the game only by emitting `Action`s
through the engine's `apply_action` adapter. It hosts the `Controller` trait,
`StrategyController`, and the `run_simulation`/`simulate` entry points.

> **Status: planned future feature, not under active development.** The crate
> is frozen at the pre-Infinity frontier: `ad-core`'s `Action` vocabulary and
> `ObservedState` cover early-game strategy runs only, and the integration
> tests are disabled (`autotests = false` in `Cargo.toml` — some playthroughs
> run long after game-logic changes and the suite needs revisiting). While the
> port is the focus, the only requirement is that the crate keeps compiling.
> When reviving it, extend `ObservedState` and `Action` one prestige layer at
> a time (Infinity → Eternity → …) and re-enable the tests.

Design rationale: [`../../docs/design/2026-06-27-simulation-architecture.md`](../../docs/design/2026-06-27-simulation-architecture.md).

This is a **living** file map: keep it in sync with the code.

## Key source files

- `src/controller.rs` — `Controller` trait (`on_start` + per-tick `act`) and
  `StrategyController` (fixed-strategy player; emits `Action`s only)
- `src/simulator.rs` — `run_simulation` driver loop, `simulate` wrapper, and the
  `StateTrace`/`StopCondition`/`StopReason`/`Simulation{Config,Result}` types
- `src/strategy.rs` — `StrategyConfig` and its enums (sacrifice/purchase/prestige)
