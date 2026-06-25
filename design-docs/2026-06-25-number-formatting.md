# Number Formatting: Where It Lives

Status: decided. Milestone 1 ships **Option A**; we migrate to **Option C** in a
later milestone.

## Context

Antimatter Dimensions displays huge numbers (up to ~1e9e15 and beyond) using a
configurable *notation*. The original game splits this into:

- `src/core/format.js` — thin wrappers: `format`, `formatInt`, `formatX`,
  `formatPercents`, etc.
- the notations registry (~25 strategies: Scientific, Engineering, Standard /
  letters, Mixed, Logarithm, Roman, Emoji, …). The active one is
  `player.options.notation`, plus digit-count and comma sub-options.

Three consumers will eventually want formatting:

1. the **engine / GUI** — every cost, multiplier, amount, production, header
   value is formatted, hundreds of `format()` calls per frame;
2. **Python bindings** (project goal #3) — human-readable output in notebooks;
3. the **web frontend** (Tauri webview) — the Vue components render formatted
   strings.

Key facts that constrain the design:

- Formatting is **presentation, not game logic.** Wherever it lives it must be a
  pure function `format(value, &FormatOptions) -> String`; it must **not** read
  from `GameState`, which stays a pure, deterministic simulation.
- The notation preference is **UI state**, passed in as an argument. It never
  belongs in `GameState`.
- `format()` is called *hundreds of times per frame*, so any design that routes
  individual format calls across the async Tauri IPC boundary is a non-starter.

## Options

### A. Rust formats, pre-baked into the per-tick snapshot
The Rust `build_game_view` formats every value and the snapshot carries finished
strings (one IPC round-trip per tick). This is what `ad-gui`
milestone 1 does (`crates/ad-gui/src/main.rs::format_decimal`).

- Pros: source of truth in Rust; trivial Python reuse; no JS number library;
  directly fidelity-testable against the JS `format()`.
- Cons: every formatted value *and every variant* (`format` vs `formatInt` vs
  `formatX` of the same number) must be enumerated as a snapshot field. The
  original components freely re-format the same number per context; pre-baking
  loses that flexibility and the ability to reuse the original templates'
  `{{ format(x) }}` calls. **Ergonomics degrade as tabs multiply.**

### B. Format in JS (port / vendor `format.js` + notations to JS)
- Pros: mirrors the original components exactly (reuse templates verbatim);
  per-component flexibility; notation preference is pure client state; no IPC
  chatter.
- Cons: **no Python reuse**; a second implementation to keep in sync with the
  engine's number type; requires `break_infinity.js` on the frontend.

### C. Format in Rust, shared to both consumers (PyO3 + WASM)
Write the notation system once as a standalone pure crate (`ad-format`, taking
`value + FormatOptions`). Expose it to Python via PyO3 and compile it to
WebAssembly so the Vue components call `format(rawNumber, opts)` **synchronously,
in-process** in the webview (no IPC). The snapshot then sends *raw* numbers
(mantissa/exponent) and JS formats them.

- Pros: single source of truth; Python **and** web reuse; full JS template
  ergonomics; no IPC chatter; fidelity-testable. Satisfies all three consumers
  from one implementation.
- Cons: adds a `wasm-bindgen` / `wasm-pack` build step and a small `Decimal`
  round-trip glue across the wasm boundary.

## Decision

**Milestone 1: Option A.** It already works, is the fastest path to pre-infinity
parity, and the handful of formatted fields is not yet painful. No notation
options UI, no `break_infinity.js` on the frontend.

**Target: Option C.** A dedicated later milestone introduces the `ad-format`
crate and wires it to PyO3 + WASM. At that point the snapshot switches from
pre-formatted strings to raw numbers and components call the wasm `format()`.
This is the only option that serves engine, Python, and web from one
implementation while preserving template ergonomics — and it keeps the notation
preference a pure `FormatOptions` argument, never in `GameState`.

Why not B: it gives up Python reuse and creates a second formatter to maintain,
which contradicts the single-source-of-truth goal of the Rust rewrite.

Why not stay on A: the snapshot-field explosion and loss of per-component format
flexibility get worse with every new tab.

## Migration notes (A → C)

- Introduce `crates/ad-format` with `format(value: &Decimal, opts: &FormatOptions)`
  and the notation strategies (start with Scientific, Engineering, Standard,
  Mixed; add the rest incrementally).
- Add a `FormatOptions` struct (notation choice, digit counts, comma settings).
  It lives in GUI / caller state, **not** in `GameState`.
- PyO3: expose `format` + `FormatOptions` from `ad-python`.
- WASM: build `ad-format` with `wasm-bindgen`; the frontend imports the generated
  glue and passes `{mantissa, exponent}` for each value.
- Change the GUI snapshot (`build_game_view`) to emit raw numbers instead of
  formatted strings; update components to call the wasm `format()`.
- Fidelity: add `format()` comparisons (Rust vs JS) to the `ad-fidelity` harness.
