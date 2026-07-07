# Architecture

This is the **living** architecture reference: it describes how the workspace
fits together *right now*. Keep it current — when a change alters crate
responsibilities, dependencies, or a design principle, update this file in the
same change. For the intent and history behind a system, follow the design-doc
links (those are historical and are not kept in sync with the code).

## Workspace layout & dependencies

```
break_infinity              base; the Decimal type, no internal deps
   ├── ad-core              game rules; depends on break_infinity
   │      ├── ad-sim        simulation driver; one-way dependency on ad-core
   │      ├── ad-python     PyO3 bindings over ad-core
   │      └── ad-gui        Tauri backend embeds ad-core (Rust-authoritative)
   └── ad-format            formatting; depends on break_infinity only, never
                            reads GameState; compiled to WASM for ad-gui's webview

ad-fidelity                 harness; drives ad-core / ad-sim and compares
                            outputs against the original JS game
```

Key invariants of the dependency graph:
- `ad-core` **never** depends on `ad-sim`. The simulation driver depends on the
  engine, not the other way around.
- `ad-format` **never** reads `GameState`. It is pure presentation over `Decimal`.
- `ad-gui` uses both `ad-core` (Tauri backend) and `ad-format` (via WASM in the
  webview).

## Crate responsibilities

| Crate | Type | Purpose |
|-------|------|---------|
| `break_infinity` | lib | Decimal type: `mantissa × 10^exponent` arithmetic for numbers up to ~1e9e15 |
| `ad-core` | lib | Game engine (the rules). Pure logic, no IO. Owns `GameState`, the `Action` IR + `apply_action` seam, and the `data` module for static config. Never depends on `ad-sim`. File map: [`crates/ad-core/ARCHITECTURE.md`](../crates/ad-core/ARCHITECTURE.md). |
| `ad-sim` | lib | Simulation driver (decides *what a player does*). One-way dependency on `ad-core`; mutates the game only by emitting `Action`s. Hosts the `Controller` trait, `StrategyController`, and `run_simulation`/`simulate`. See [`design/2026-06-27-simulation-architecture.md`](design/2026-06-27-simulation-architecture.md) and the file map in [`crates/ad-sim/ARCHITECTURE.md`](../crates/ad-sim/ARCHITECTURE.md). |
| `ad-format` | lib (+cdylib) | Pure, presentation-only number formatting: `format(&Decimal, &FormatOptions) -> String` + notation strategies. Never reads `GameState`. Ships WASM bindings (`wasm` feature, built via `wasm-pack`) used by `ad-gui`'s webview. See [`design/2026-06-25-number-formatting.md`](design/2026-06-25-number-formatting.md). |
| `ad-fidelity` | lib/bin | Save-replay differential harness: captures real saves from the JS game, replays each through a JS oracle (Playwright) and through `ad-core`, and diffs the persisted `player` tree with per-field tolerance. The `ad-fidelity` binary runs the Rust comparison (table / verbose). See [`design/2026-07-06-fidelity-testing.md`](design/2026-07-06-fidelity-testing.md). |
| `ad-python` | lib (cdylib) | PyO3 bindings exposing the engine to Python (`antimatter_dimensions._native`). |
| `ad-gui` | bin | **Playable frontend.** Tauri backend + Vue 3/Vite/Pinia. Rust-authoritative; see [`crates/ad-gui/AGENTS.md`](../crates/ad-gui/AGENTS.md). |

## Architecture principles

1. **Immutable config, mutable state.** All game configuration is `const`/`static`
   in the `data` module. Only `GameState` mutates.
2. **No `dyn` on hot paths.** Effect computation uses enums (jump table) rather
   than trait objects to allow inlining.
3. **Deterministic simulation.** The engine is fully deterministic given the same
   inputs — no `SystemTime`, no unseeded RNG.
4. **Frontend as thin shell.** The GUI never computes game logic; it only reads
   `GameState` for display.
5. **No ECS.** The game has a fixed, known set of entities. Plain structs with
   named fields are simpler and faster.

## Per-crate internals

The file-by-file maps live next to the code:

- [`crates/ad-core/ARCHITECTURE.md`](../crates/ad-core/ARCHITECTURE.md) — the
  engine's modules.
- [`crates/ad-sim/ARCHITECTURE.md`](../crates/ad-sim/ARCHITECTURE.md) — the
  simulation driver.
- [`crates/ad-gui/AGENTS.md`](../crates/ad-gui/AGENTS.md) — the frontend.

For the number representation used everywhere, see the **Number System** section
of [`../AGENTS.md`](../AGENTS.md).
