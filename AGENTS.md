# AGENTS.md

## Project Overview

This is a Rust reimplementation of [Antimatter Dimensions](https://ivark.github.io/AntimachDim/), an incremental/idle game. The long-term goal includes supporting the [endgame mod](https://buck4437.github.io/AntimatterDimensionsEndgame/) as well.

The project exists to:
1. Learn idiomatic Rust (not a line-by-line JS translation)
2. Build a fast simulation engine for numerical experiments
3. Provide Python bindings (PyO3) for data analysis
4. Create a playable frontend (Tauri + Vue 3; see `crates/ad-gui`)

The original JS source code is available at `../antimatter-dimensions` and `../antimatter-dimensions-endgame` for reference.

## Repository Structure

```
antimatter-dimensions-rust/
├── Cargo.toml              # Workspace manifest
├── design-docs/            # Architecture & analysis documents
├── python/                 # Python source for the PyO3 bindings
├── crates/
│   ├── break_infinity/     # Vendored big-number library (Decimal type)
│   ├── ad-core/            # Game engine (rules) + static config
│   ├── ad-sim/             # Simulation driver: Controller + run_simulation (depends on ad-core)
│   ├── ad-format/          # Number formatting (notations): format(value, &FormatOptions)
│   ├── ad-fidelity/        # Rust-vs-JS fidelity test harness
│   ├── ad-python/          # PyO3 bindings (antimatter_dimensions._native)
│   └── ad-gui/             # Tauri + Vue 3 frontend (playable; see its AGENTS.md)
```

### Crate Responsibilities

| Crate | Type | Purpose |
|-------|------|---------|
| `break_infinity` | lib | Decimal type: `mantissa × 10^exponent` arithmetic for numbers up to ~1e9e15 |
| `ad-core` | lib | Game engine (the rules). Pure logic, no IO. Owns `GameState`, the `Action` IR + `apply_action` seam, and the `data` module for static config. Never depends on `ad-sim`. |
| `ad-sim` | lib | Simulation driver (decides *what a player does*). One-way dependency on `ad-core`; mutates the game only by emitting `Action`s. Hosts the `Controller` trait, `StrategyController`, and `run_simulation`/`simulate`. See `design-docs/2026-06-27-simulation-architecture.md`. |
| `ad-format` | lib (+cdylib) | Pure, presentation-only number formatting: `format(&Decimal, &FormatOptions) -> String` + notation strategies. Never reads `GameState`. Ships WASM bindings (`wasm` feature, built via `wasm-pack`) used by `ad-gui`'s webview. See `design-docs/2026-06-25-number-formatting.md`. |
| `ad-fidelity` | lib/bin | Scenario-based harness comparing Rust engine outputs against the JS game. |
| `ad-python` | lib (cdylib) | PyO3 bindings exposing the engine to Python (`antimatter_dimensions._native`). |
| `ad-gui` | bin | **Playable frontend.** Tauri backend + Vue 3/Vite/Pinia. Rust-authoritative; see `crates/ad-gui/AGENTS.md`. |

### Key Source Files (ad-core)

- `src/state.rs` — `GameState` struct (all mutable game state)
- `src/action.rs` — `Action` IR + `GameState::apply_action`: the single mutation
  seam every action producer (GUI, autobuyers, simulation) routes through
- `src/tick.rs` — Main game loop (`tick()` and `simulate()`)
- `src/dimensions.rs` — Dimension purchasing, production, multipliers
- `src/tickspeed.rs` — Tickspeed upgrades and effects
- `src/galaxy.rs` — Antimatter galaxy purchases
- `src/sacrifice.rs` — Dimension sacrifice
- `src/crunch.rs` — Big Crunch (Infinity): `can_big_crunch`, `big_crunch`, and the
  shared `big_crunch_reset(forced, entering_challenge)` that both the manual crunch
  and the challenge enter/exit route through. Awards Infinity Points
  (`gained_infinity_points`, pre-break = 1), Infinities (`gained_infinities`), and
  challenge completion only when at the goal; updates the fastest-infinity record; IP
  / infinities / total-time-played persist across the reset. See
  `design-docs/2026-07-02-infinity-points-and-records.md`.
- `src/challenges.rs` — Normal Challenges (Feature 2.5): `NormalChallengeState` on
  `GameState` (`current` + `completed` bitmask), unlock/start/exit/complete logic,
  and the reward wiring (completing NC1–9 unlocks the AD/Tickspeed autobuyers). The
  per-challenge rule modifiers are added incrementally at their engine sites; NC1
  (the tutorial) is done. See `design-docs/2026-07-03-normal-challenges.md`.
- `src/records.rs` — `Records`: the modelled slice of `player.records` (total time
  played, this-infinity time/`maxAM`, best-infinity time). Advanced in `tick`; the
  current-infinity records reset on a Big Crunch.
- `src/infinity_upgrades.rs` — Infinity Upgrades (Feature 2.2): the `InfinityUpgrade`
  enum + data table (cost, save-id, column prerequisite), purchase logic
  (`buy_infinity_upgrade`, IP-gated bitmask on `GameState::infinity_upgrades`), and
  the effect readers other modules call (`buy_ten_multiplier`, `dim_boost_power`,
  `galaxy_strength_effect`, `reset_boost_reduction`, the AD-multiplier
  contributions, `skip_resets_if_possible`, passive `generate_passive_ip`). Effects
  are *applied* at the original's sites (dimension multiplier, tickspeed, boost/
  galaxy requirement, reset paths). See
  `design-docs/2026-07-03-infinity-upgrades.md`.
- `src/achievements.rs` — Normal achievements: `achievement_bits` bitmask helpers
  (`achievement_unlocked`/`unlock_achievement`), the global `achievement_power`
  multiplier, and `starting_antimatter`. Unlocks fire inline from the relevant
  action methods; see `design-docs/2026-06-30-achievements.md`.
- `src/tutorial.rs` — Tutorial-highlight state machine (`tutorial_state` /
  `tutorial_active`): the gold glow + `!` that points a new player at the next
  action. Advances passively in `tick()` and on the boost/galaxy/tickspeed
  actions; the frontend renders the highlight. See
  `design-docs/2026-06-30-ui-reveal-and-tutorial.md`.
- `src/autobuyers.rs` — Automation system
- `src/options.rs` — `Options` struct: player UI/UX preferences (mirrors JS
  `player.options`), held in `GameState`, preserved across a Big Crunch.
  Includes the per-action `Confirmations` toggles (boost/galaxy/sacrifice/crunch)
- `src/observed.rs` — `ObservedState`: read-only snapshot of `GameState` plus
  computed fields (costs, affordability, `next_sacrifice_boost`). The decision
  input for `ad-sim` controllers and the trace/GUI view.
- `src/data/` — Static game configuration (constants, costs, dimension configs)

### Key Source Files (ad-sim)

- `src/controller.rs` — `Controller` trait (`on_start` + per-tick `act`) and
  `StrategyController` (fixed-strategy player; emits `Action`s only)
- `src/simulator.rs` — `run_simulation` driver loop, `simulate` wrapper, and the
  `StateTrace`/`StopCondition`/`StopReason`/`Simulation{Config,Result}` types
- `src/strategy.rs` — `StrategyConfig` and its enums (sacrifice/purchase/prestige)

## Architecture Principles

1. **Immutable config, mutable state.** All game configuration is `const`/`static` in the `data` module. Only `GameState` mutates.
2. **No `dyn` on hot paths.** Effect computation uses enums (jump table) rather than trait objects to allow inlining.
3. **Deterministic simulation.** The engine is fully deterministic given the same inputs—no `SystemTime`, no unseeded RNG.
4. **Frontend as thin shell.** The GUI never computes game logic; it only reads `GameState` for display.
5. **No ECS.** The game has a fixed, known set of entities. Plain structs with named fields are simpler and faster.

## Development Guidelines

### Building and Testing

```bash
cargo build                    # Build all crates
cargo test                     # Run all tests
cargo test -p break_infinity   # Test only the number library
cargo test -p ad-core          # Test only the game engine
cargo test -p ad-sim           # Test only the simulation driver
cargo clippy                   # Lint
cargo fmt                      # Format
```

#### Frontend (ad-gui)

```bash
cargo install wasm-pack                        # once (frontend build needs it)
npm --prefix crates/ad-gui/frontend install    # once
npm --prefix crates/ad-gui/frontend run build  # build wasm + Vue app to dist/
cargo run -p ad-gui                            # run the Tauri app (serves dist/)
cd crates/ad-gui && cargo tauri build          # release build (.app/.dmg)
```

The frontend `build` step first compiles `ad-format` to WASM (`wasm-pack`,
`wasm` feature) into `frontend/src/wasm/`, then runs Vite; the webview formats
numbers in-process via that module.

`cargo run` serves the pre-built `dist/` (rebuild the frontend after JS/Vue
changes — no Rust rebuild needed). `cargo tauri build` produces a bundled
macOS `.app` with the custom icon (requires `cargo install tauri-cli`).
See `crates/ad-gui/AGENTS.md` for the frontend architecture and how to add
a page.

#### Python

```bash
uv run task format             # Format Python code (ruff)
uv run task check-style        # Lint Python code (ruff)
uv run maturin develop         # Build Python bindings
```

After editing Python code, always run `uv run task format` first, then `uv run task check-style`.

### Code Style

#### Rust

- **Max line width: 89 characters** (configured in `rustfmt.toml`; also applies to prose in `design-docs/`)
- Use `cargo fmt` before committing
- Follow standard Rust naming conventions (`snake_case` for functions/variables, `CamelCase` for types)
- Prefer `i64` for the Decimal exponent (not `f64`—this is an intentional departure from the JS port)
- Comment only where clarification is needed; don't comment obvious code

#### Python

- **Max line width: 89 characters** (configured via `[tool.ruff]` in `pyproject.toml`)
- Use `uv run task format` (ruff) before committing
- Follow PEP 8 naming conventions
- Python source lives in `python/`; the native extension is `antimatter_dimensions._native`

### Number System (`break_infinity`)

The `Decimal` type represents numbers as `mantissa × 10^exponent`:
- `mantissa: f64` — normalized to [1, 10) or 0
- `exponent: i64` — integer exponent (departure from JS which uses `f64`)
- Implements `Add`, `Sub`, `Mul`, `Div`, `Neg`, `PartialOrd`, `Ord`, `Display`, `FromStr`
- Key constants: `Decimal::ZERO`, `Decimal::ONE`
- Construction: `Decimal::from_float(f64)`, `Decimal::new(mantissa, exponent)`
  (normalizing). `Decimal::new_unchecked(mantissa, exponent)` is a `const fn`
  for already-normalized values, so it can initialize `const`/`static` items
  (e.g. `BIG_CRUNCH_THRESHOLD`); `new` cannot, as normalization isn't const.

### Adding Game Systems

The project follows a phased approach (see `design-docs/2026-06-19-architecture.md` §9):
1. Foundation: `break_infinity` + basic `GameState`
2. Core: antimatter dimensions, tickspeed, dim boosts, galaxies, sacrifice
3. First prestige: infinity, infinity dimensions, normal challenges
4. Second prestige: eternity, time dimensions, time studies
5. Mid-game: replicanti, dilation, eternity challenges
6. Reality: glyphs, perks, celestials

When adding a new system:
- Add game state fields to `GameState` in `state.rs`
- Add static configuration to `src/data/`
- Implement logic as methods on `GameState` or in a dedicated module
- Integrate into the tick loop in `tick.rs`
- Add unit tests alongside the code and integration tests in `tests/`

### Updating Documentation

After completing a significant piece of work (new game system, architectural
change, new crate, major refactor, or new tooling), update all relevant
documentation before considering the task done:

- **This file (`AGENTS.md`)** — update repository structure, key source files,
  crate responsibilities, build commands, or any section affected by the change.
- **`crates/ad-gui/AGENTS.md`** — if the frontend was modified.
- **`design-docs/`** — add a new design doc for major architectural decisions;
  update existing docs if they reference changed behaviour.
- **Design Documents table** (in this file) — add entries for any new design
  docs created.
- **README or inline doc-comments** — if public APIs or usage instructions
  changed.

This applies to both human and AI contributors. Documentation that drifts from
the code is worse than no documentation.

### Referencing the Original Game

The original JS source is at `../antimatter-dimensions/src/core/`. Key directories:
- `src/core/dimensions/` — Dimension classes
- `src/core/secret-formula/` — Game data/constants/configurations
- `src/core/game-mechanics/` — Base classes (Effect, Purchasable, etc.)
- `src/core/celestials/` — Endgame celestial mechanics
- `src/game.js` — Main game loop + prestige formulas

When porting a system, aim for **behavioral fidelity** (same gameplay results) rather than structural fidelity (same code organization).

**UI fidelity:** The UI should match the original game **exactly** — same layout,
sizing, colors, fonts, and styling. The frontend vendors the original game's
stylesheets verbatim (see `crates/ad-gui/frontend/public/stylesheets/`), so for
any UI implementation, consult the original game code to see how those stylesheets
are applied: which classes a component uses, the exact CSS values (widths,
font-sizes, spacing), and which CSS variables (e.g. `--color-accent`,
`--color-good`) it references. The original Vue components live in
`../antimatter-dimensions/src/components/`. Prefer reusing the vendored classes
and variables over inventing new styles, and copy concrete values from the
original rather than guessing.

**Number formatting / notations:** `src/core/format.js` holds only the thin wrappers
(`format`, `formatInt`, `formatX`, …). The actual notation strategies (Scientific,
Engineering, Standard, Letters, …) live in the external `@antimatter-dimensions/notations`
package, not in `src/core/`. Its source is the bundled dist (no `.ts` sources shipped):
`../antimatter-dimensions/node_modules/@antimatter-dimensions/notations/dist/ad-notations.esm.js`
(run `npm install` in `../antimatter-dimensions` if `node_modules` is absent).

## Design Documents

Located in `design-docs/`:

| Document | Summary |
|----------|---------|
| `2026-06-11-codebase-analysis.md` | Full analysis of the original JS game's architecture |
| `2026-06-11-endgame-analysis.md` | Analysis of the endgame mod's additions |
| `2026-06-19-architecture.md` | Rust project architecture, workspace layout, design decisions |
| `2026-06-19-break-infinity-review.md` | Code review of the vendored break_infinity crate |
| `2026-06-21-break-eternity-representation.md` | Design for extending Decimal to support break_eternity (tower numbers) |
| `2026-06-24-ui-framework-analysis.md` | Comparison of GUI framework options for the playable frontend |
| `2026-06-25-frontend-architecture.md` | `ad-gui` design: Tauri + Vue 3/Vite/Pinia, vendored CSS, Rust-authoritative snapshot |
| `2026-06-25-number-formatting.md` | Where number formatting lives (Rust now; PyO3 + WASM later) and why |
| `2026-06-27-options-tabs.md` | Analysis of the Visual & Gameplay options tabs + iterative port plan |
| `2026-06-27-simulation-architecture.md` | Options for a full end-to-end simulation driver (Action IR + Controller trait) kept cleanly separate from game logic |
| `2026-06-28-js-frontend-rust-wasm-engine.md` | Feasibility analysis of keeping the original JS/Vue app and swapping its engine for Rust/WASM (rejected; recommends a WASM target for `ad-core` instead) |
| `2026-06-30-offline-progress.md` | How the original simulates offline progress, how it maps onto our `simulate`/`ticks` primitives, the game-speed/timestamp implications, and a design for a manual Offline-mode button |
| `2026-06-30-ui-reveal-and-tutorial.md` | Progressive UI reveal (hiding/showing AD rows, tickspeed, sacrifice), first-time/disable-able confirmation modals (boost/galaxy/sacrifice/crunch), and the tutorial glow + exclamation highlight; how the original implements each and a phased plan |
| `2026-06-30-achievements.md` | Normal achievements: bitmask state on `GameState`, unlock hooks inline in the buy/galaxy/boost/crunch/tick methods (rows 1–2 minus News), per-achievement effects + the global achievement-power multiplier, `achievementBits` save round-trip, the sprite-driven tab, and the unlock toast; phased plan |
| `2026-07-02-infinity-points-and-records.md` | Completing Feature 2.1: Infinity Points / Infinities currency, the `Records` struct (time played, this/best infinity), the IP gain formula (pre-break = 1), Big Crunch reward+reset semantics, save/load round-trip, and the Infinity tab + IP header |
| `2026-07-03-infinity-upgrades.md` | Feature 2.2: the 16-upgrade Infinity grid — data table, bitmask state, purchase/column prereqs, every effect and its engine application site, passive `ipGen`, save/load, and the grid UI; bottom row (`ipMult`/`ipOffline`) deferred |
| `2026-07-03-normal-challenges.md` | Feature 2.5: the 12 Normal Challenges — run state machine (start/complete/exit, forced Big-Crunch reset, unlock chain), all 12 modifiers mapped to their engine sites, reward→autobuyer wiring, save/load, the Challenges tab UI, and an incremental plan (NC1 slice first) |

The table lists key documents; see the `design-docs/` folder for the full,
date-prefixed set. Read these before making architectural decisions. The
architecture doc is the primary reference.

## Testing

- **Unit tests:** In each crate, testing individual functions and systems in isolation.
- **Integration tests:** In `crates/ad-core/tests/`, testing full game tick sequences.
- **Property tests:** For Decimal arithmetic invariants (planned).
- **Fidelity tests:** Scenario-based comparison against JS outputs (planned, using `ad-fidelity`).

For fidelity testing, comparison uses log-space relative tolerance (default 1e-10) since floating-point arithmetic differs slightly between JS and Rust.
