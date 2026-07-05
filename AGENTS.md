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
├── AGENTS.md               # This file — stable agent instructions
├── CLAUDE.md               # Imports AGENTS.md so Claude Code picks it up
├── docs/
│   ├── README.md           # Documentation index + conventions
│   ├── ARCHITECTURE.md     # Living architecture reference (crates, principles)
│   ├── PORTING.md          # Porting method + referencing the original game
│   ├── design/             # Design docs / RFCs — historical, written before coding
│   └── worklog/            # Append-only session logs — historical, after coding
├── python/                 # Python source for the PyO3 bindings
├── crates/
│   ├── break_infinity/     # Vendored big-number library (Decimal type)
│   ├── ad-core/            # Game engine (rules) + static config; see its ARCHITECTURE.md
│   ├── ad-sim/             # Simulation driver (depends on ad-core); see its ARCHITECTURE.md
│   ├── ad-format/          # Number formatting (notations): format(value, &FormatOptions)
│   ├── ad-fidelity/        # Rust-vs-JS fidelity test harness
│   ├── ad-python/          # PyO3 bindings (antimatter_dimensions._native)
│   └── ad-gui/             # Tauri + Vue 3 frontend (playable; see its AGENTS.md)
```

For crate responsibilities, the dependency graph, and design principles, see
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Documentation Map

Read the relevant deep docs before working; keep the living ones current.

- **[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)** — how the workspace fits
  together (crate responsibilities, dependency graph, design principles). Read
  before cross-crate work. **Living — keep current.**
- **`crates/<crate>/ARCHITECTURE.md`** — file-by-file map of a crate's internals
  (currently [`ad-core`](crates/ad-core/ARCHITECTURE.md) and
  [`ad-sim`](crates/ad-sim/ARCHITECTURE.md)). Read and update these when working
  inside those crates. **Living — keep current.**
- **[`crates/ad-gui/AGENTS.md`](crates/ad-gui/AGENTS.md)** — frontend-specific
  agent instructions.
- **[`docs/PORTING.md`](docs/PORTING.md)** — fidelity standard and how to
  reference the original JS game.
- **[`docs/design/`](docs/design/)** — design docs / RFCs, written before coding.
  Historical; see [`docs/README.md`](docs/README.md) for the index and the status
  of each.
- **[`docs/worklog/`](docs/worklog/)** — append-only log written after each work
  session. See [`docs/worklog/README.md`](docs/worklog/README.md).
- **[`docs/README.md`](docs/README.md)** — the documentation index and conventions
  (naming, status front-matter, cross-linking).

## Building and Testing

```bash
cargo build                    # Build all crates
cargo test                     # Run all tests
cargo test -p break_infinity   # Test only the number library
cargo test -p ad-core          # Test only the game engine
cargo test -p ad-sim           # Test only the simulation driver
cargo clippy                   # Lint
cargo fmt                      # Format
```

### Frontend (ad-gui)

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
See [`crates/ad-gui/AGENTS.md`](crates/ad-gui/AGENTS.md) for the frontend
architecture and how to add a page.

### Python

```bash
uv run task format             # Format Python code (ruff)
uv run task check-style        # Lint Python code (ruff)
uv run maturin develop         # Build Python bindings
```

After editing Python code, always run `uv run task format` first, then `uv run task check-style`.

## Code Style

### Rust

- **Max line width: 89 characters** (configured in `rustfmt.toml`; also applies to prose in `docs/`)
- Use `cargo fmt` before committing
- Follow standard Rust naming conventions (`snake_case` for functions/variables, `CamelCase` for types)
- Prefer `i64` for the Decimal exponent (not `f64`—this is an intentional departure from the JS port)
- Comment only where clarification is needed; don't comment obvious code

### Python

- **Max line width: 89 characters** (configured via `[tool.ruff]` in `pyproject.toml`)
- Use `uv run task format` (ruff) before committing
- Follow PEP 8 naming conventions
- Python source lives in `python/`; the native extension is `antimatter_dimensions._native`

## Number System (`break_infinity`)

The `Decimal` type represents numbers as `mantissa × 10^exponent`:
- `mantissa: f64` — normalized to [1, 10) or 0
- `exponent: i64` — integer exponent (departure from JS which uses `f64`)
- Implements `Add`, `Sub`, `Mul`, `Div`, `Neg`, `PartialOrd`, `Ord`, `Display`, `FromStr`
- Key constants: `Decimal::ZERO`, `Decimal::ONE`
- Construction: `Decimal::from_float(f64)`, `Decimal::new(mantissa, exponent)`
  (normalizing). `Decimal::new_unchecked(mantissa, exponent)` is a `const fn`
  for already-normalized values, so it can initialize `const`/`static` items
  (e.g. `BIG_CRUNCH_THRESHOLD`); `new` cannot, as normalization isn't const.

## Adding a Game System

The project follows a phased porting roadmap; see the fidelity standard and the
phase list in [`docs/PORTING.md`](docs/PORTING.md), and the original architecture
decision in [`docs/design/2026-06-19-architecture.md`](docs/design/2026-06-19-architecture.md) §9.

When adding a new system:
- Add game state fields to `GameState` in `state.rs`
- Add static configuration to `src/data/`
- Implement logic as methods on `GameState` or in a dedicated module
- Integrate into the tick loop in `tick.rs`
- Add unit tests alongside the code and integration tests in `tests/`
- Update [`crates/ad-core/ARCHITECTURE.md`](crates/ad-core/ARCHITECTURE.md) (and
  `docs/ARCHITECTURE.md` for cross-crate changes) — see below.

## Updating Documentation

Documentation that drifts from the code is worse than no documentation. This
applies to both human and AI contributors. The docs fall into three buckets by
*when* they are written and *how* they are maintained:

- **Living (keep current):** `AGENTS.md`, `docs/ARCHITECTURE.md`,
  `crates/*/ARCHITECTURE.md`, `crates/ad-gui/AGENTS.md`. When you change code,
  update these in the *same* change so they always describe how things work
  **now**.
- **Design docs (`docs/design/`) — historical.** Written before coding. Do **not**
  rewrite them to match new code. The only edits allowed after the fact are:
  (1) the `status:` front-matter field, and (2) ticking checkboxes in embedded
  plans/checklists. When a doc no longer reflects reality, set `status: Superseded`
  (with `superseded_by:` if applicable) rather than editing the body.
- **Worklog (`docs/worklog/`) — historical, append-only.** After a work session,
  add a **new** dated file. Never edit an existing worklog entry.

After a significant piece of work (new system, architectural change, new crate,
major refactor, new tooling):

1. Update the **living** docs the change touches (`crates/*/ARCHITECTURE.md`,
   `docs/ARCHITECTURE.md`, `crates/ad-gui/AGENTS.md`, this file).
2. Add a **worklog** entry `docs/worklog/YYYY-MM-DD-<slug>.md` — see
   [`docs/worklog/README.md`](docs/worklog/README.md) for the template.
3. **Cross-link:** in the worklog entry, link the design doc(s) you implemented
   and note any **deviations** from them. If a design doc no longer matches the
   code, update its `status` (do not rewrite its body).
4. If you added a new design doc, add a row to the index table in
   [`docs/README.md`](docs/README.md) and set its `status`.

## Testing

- **Unit tests:** In each crate, testing individual functions and systems in isolation.
- **Integration tests:** In `crates/ad-core/tests/`, testing full game tick sequences.
- **Property tests:** For Decimal arithmetic invariants (planned).
- **Fidelity tests:** Scenario-based comparison against JS outputs (planned, using `ad-fidelity`).

For fidelity testing, comparison uses log-space relative tolerance (default 1e-10) since floating-point arithmetic differs slightly between JS and Rust.
