# AGENTS.md

## Project Overview

This is a Rust reimplementation of [Antimatter Dimensions](https://ivark.github.io/AntimachDim/), an incremental/idle game. The long-term goal includes supporting the [endgame mod](https://buck4437.github.io/AntimatterDimensionsEndgame/) as well.

The project exists to:
1. Learn idiomatic Rust (not a line-by-line JS translation)
2. Build a fast simulation engine for numerical experiments
3. Provide Python bindings (PyO3) for data analysis
4. Create a playable egui frontend

The original JS source code is available at `../antimatter-dimensions` and `../antimatter-dimensions-endgame` for reference.

## Repository Structure

```
antimatter-dimensions-rust/
├── Cargo.toml              # Workspace manifest
├── design-docs/            # Architecture & analysis documents
├── crates/
│   ├── break_infinity/     # Vendored big-number library (Decimal type)
│   ├── ad-core/            # Game simulation engine + static config
│   └── ad-gui/             # egui-based playable frontend
```

### Crate Responsibilities

| Crate | Type | Purpose |
|-------|------|---------|
| `break_infinity` | lib | Decimal type: `mantissa × 10^exponent` arithmetic for numbers up to ~1e9e15 |
| `ad-core` | lib | Game simulation engine. Pure logic, no IO. Contains a `data` module for static config. |
| `ad-gui` | bin | egui/eframe frontend. Thin shell that owns a `GameState` and calls `tick()` each frame. |

Future crates (not yet created): `ad-python` (PyO3 bindings), `ad-fidelity` (test harness comparing Rust vs JS outputs).

### Key Source Files (ad-core)

- `src/state.rs` — `GameState` struct (all mutable game state)
- `src/tick.rs` — Main game loop (`tick()` and `simulate()`)
- `src/dimensions.rs` — Dimension purchasing, production, multipliers
- `src/tickspeed.rs` — Tickspeed upgrades and effects
- `src/galaxy.rs` — Antimatter galaxy purchases
- `src/sacrifice.rs` — Dimension sacrifice
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
cargo run -p ad-gui            # Run the GUI
cargo clippy                   # Lint
cargo fmt                      # Format
```

### Code Style

- **Max line width: 89 characters** (configured in `rustfmt.toml`; also applies to prose in `design-docs/`)
- Use `cargo fmt` before committing
- Follow standard Rust naming conventions (`snake_case` for functions/variables, `CamelCase` for types)
- Prefer `i64` for the Decimal exponent (not `f64`—this is an intentional departure from the JS port)
- Comment only where clarification is needed; don't comment obvious code

### Number System (`break_infinity`)

The `Decimal` type represents numbers as `mantissa × 10^exponent`:
- `mantissa: f64` — normalized to [1, 10) or 0
- `exponent: i64` — integer exponent (departure from JS which uses `f64`)
- Implements `Add`, `Sub`, `Mul`, `Div`, `Neg`, `PartialOrd`, `Ord`, `Display`, `FromStr`
- Key constants: `Decimal::ZERO`, `Decimal::ONE`
- Construction: `Decimal::from_float(f64)`, `Decimal::new(mantissa, exponent)`

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

### Referencing the Original Game

The original JS source is at `../antimatter-dimensions/src/core/`. Key directories:
- `src/core/dimensions/` — Dimension classes
- `src/core/secret-formula/` — Game data/constants/configurations
- `src/core/game-mechanics/` — Base classes (Effect, Purchasable, etc.)
- `src/core/celestials/` — Endgame celestial mechanics
- `src/game.js` — Main game loop + prestige formulas

When porting a system, aim for **behavioral fidelity** (same gameplay results) rather than structural fidelity (same code organization).

## Design Documents

Located in `design-docs/`:

| Document | Summary |
|----------|---------|
| `2026-06-11-codebase-analysis.md` | Full analysis of the original JS game's architecture |
| `2026-06-11-endgame-analysis.md` | Analysis of the endgame mod's additions |
| `2026-06-19-architecture.md` | Rust project architecture, workspace layout, design decisions |
| `2026-06-19-break-infinity-review.md` | Code review of the vendored break_infinity crate |
| `2026-06-21-break-eternity-representation.md` | Design for extending Decimal to support break_eternity (tower numbers) |

Read these before making architectural decisions. The architecture doc is the primary reference.

## Testing

- **Unit tests:** In each crate, testing individual functions and systems in isolation.
- **Integration tests:** In `crates/ad-core/tests/`, testing full game tick sequences.
- **Property tests:** For Decimal arithmetic invariants (planned).
- **Fidelity tests:** Scenario-based comparison against JS outputs (planned, using `ad-fidelity`).

For fidelity testing, comparison uses log-space relative tolerance (default 1e-10) since floating-point arithmetic differs slightly between JS and Rust.
