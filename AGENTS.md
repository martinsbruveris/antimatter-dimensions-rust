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
│   ├── ad-core/            # Game simulation engine + static config
│   ├── ad-format/          # Number formatting (notations): format(value, &FormatOptions)
│   ├── ad-fidelity/        # Rust-vs-JS fidelity test harness
│   ├── ad-python/          # PyO3 bindings (antimatter_dimensions._native)
│   └── ad-gui/             # Tauri + Vue 3 frontend (playable; see its AGENTS.md)
```

### Crate Responsibilities

| Crate | Type | Purpose |
|-------|------|---------|
| `break_infinity` | lib | Decimal type: `mantissa × 10^exponent` arithmetic for numbers up to ~1e9e15 |
| `ad-core` | lib | Game simulation engine. Pure logic, no IO. Contains a `data` module for static config. |
| `ad-format` | lib | Pure, presentation-only number formatting: `format(&Decimal, &FormatOptions) -> String` + notation strategies. Never reads `GameState`. See `design-docs/2026-06-25-number-formatting.md`. |
| `ad-fidelity` | lib/bin | Scenario-based harness comparing Rust engine outputs against the JS game. |
| `ad-python` | lib (cdylib) | PyO3 bindings exposing the engine to Python (`antimatter_dimensions._native`). |
| `ad-gui` | bin | **Playable frontend.** Tauri backend + Vue 3/Vite/Pinia. Rust-authoritative; see `crates/ad-gui/AGENTS.md`. |

### Key Source Files (ad-core)

- `src/state.rs` — `GameState` struct (all mutable game state)
- `src/tick.rs` — Main game loop (`tick()` and `simulate()`)
- `src/dimensions.rs` — Dimension purchasing, production, multipliers
- `src/tickspeed.rs` — Tickspeed upgrades and effects
- `src/galaxy.rs` — Antimatter galaxy purchases
- `src/sacrifice.rs` — Dimension sacrifice
- `src/crunch.rs` — First Big Crunch (Infinity): `can_big_crunch` + `big_crunch` reset
- `src/autobuyers.rs` — Automation system
- `src/data/` — Static game configuration (constants, costs, dimension configs)

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
cargo clippy                   # Lint
cargo fmt                      # Format
```

#### Frontend (ad-gui)

```bash
npm --prefix crates/ad-gui/frontend install   # once
npm --prefix crates/ad-gui/frontend run build # build the Vue app to dist/
cargo run -p ad-gui                           # run the Tauri app (serves dist/)
cd crates/ad-gui && cargo tauri build         # release build (.app/.dmg)
```

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

The table lists key documents; see the `design-docs/` folder for the full,
date-prefixed set. Read these before making architectural decisions. The
architecture doc is the primary reference.

## Testing

- **Unit tests:** In each crate, testing individual functions and systems in isolation.
- **Integration tests:** In `crates/ad-core/tests/`, testing full game tick sequences.
- **Property tests:** For Decimal arithmetic invariants (planned).
- **Fidelity tests:** Scenario-based comparison against JS outputs (planned, using `ad-fidelity`).

For fidelity testing, comparison uses log-space relative tolerance (default 1e-10) since floating-point arithmetic differs slightly between JS and Rust.
