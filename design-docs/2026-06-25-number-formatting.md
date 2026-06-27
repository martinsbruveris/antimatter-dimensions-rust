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

## What `FormatOptions` must carry

> Source of truth: `src/core/format.js` (thin wrappers) plus the external
> `@antimatter-dimensions/notations` package. The notation *strategies* are **not**
> in `src/core/`; they live in
> `../antimatter-dimensions/node_modules/@antimatter-dimensions/notations/dist/ad-notations.esm.js`.

**The notation choice does not fully determine the output.** Two things vary
independently of the notation:

1. **Per-call digit counts** — passed at each call site, so the *same number* in the
   *same notation* formats differently depending on the caller.
2. **Global Settings** — exponent-comma thresholds and the "Infinite" cutoff, driven
   by user options / game state rather than by the notation.

### The pipeline

`format.js` is a thin wrapper over
`Notations.current.format(value, places, placesUnder1000, placesExponent)`. The base
`Notation.format` routes purely on the decimal's base-10 exponent:

| Range | Method | Precision used |
|---|---|---|
| `exp < -300` (very small) | `formatVerySmallDecimal` | `placesUnder1000` |
| `exp < 3` (under 1000) | `formatUnder1000` → `toFixed` | `placesUnder1000` |
| `≥ MAX_VALUE` & pre-break | returns `"Infinite"` | — |
| otherwise (big case) | `formatDecimal` | `places` (mantissa) + `placesExponent` |

So the three digit parameters map to three different magnitude regimes.

### Per-call parameters (vary by call site, not stored on the notation)

- **`places`** — mantissa decimal places for numbers ≥ 1000. Default `0`; call sites
  commonly pass `0`, `2`, `3`. JS types this as a signed `number` with `-1` as an
  "unspecified" sentinel, clamped via `Math.max(0, …)` in `formatMantissaBaseTen`.
  We don't need the sentinel (callers always supply a value), so we model it as
  `u32` — non-negativity becomes a type invariant and the clamp disappears.
- **`placesUnder1000`** — decimal places for numbers < 1000 (and very-small).
  Default `0`.
- **`placesExponent`** — digits for the exponent *when the exponent is itself large
  enough to be in notation* (e.g. `1e1.234e15`). The game **hardcodes this to `3`** in
  `format()`; it is never threaded from call sites.

Evidence these are per-call, not per-notation: `formatX(v)` → `×format(v, places)`;
`formatPercents(v)` → `format(v*100, 2, places)%`; `formatInt` of a "painful" notation
→ `format(v, 2)`; `formatRarity` picks `0` or `1` places dynamically. Same notation,
different digits.

### Global Settings (user options / game state, not the notation)

- **`exponentCommas { show, min, max }`** — controls how the *exponent itself* is
  rendered: plain → comma-grouped → recursive notation. The game wires `min`/`max` to
  the player's `notationDigits.comma` (default 5) and `notationDigits.notation`
  (default 9): `min = 10**comma`, `max = 10**notation`. Defaults: `show: true`,
  `min 1e5`, `max 1e9`.
- **`exponentDefaultPlaces`** = 3 (the default behind `placesExponent`).
- **`isInfinite`** — the game overrides this to
  `formatPreBreak && decimal >= NUMBER_MAX_VALUE`, i.e. show `"Infinite"` only
  pre-break. This depends on **game/UI state**. We model it as a per-frame
  `inf_threshold: Option<Decimal>` in `FormatOptions` (see below), **not** as a
  per-call argument: the JS `isInfinite` is a frame-global `Settings` predicate, not a
  per-call value, so it belongs with the other per-frame options. The **caller**
  derives the threshold from game state (`Some(NUMBER_MAX_VALUE)` pre-break, `None`
  post-break) and bakes it in; `ad-format` only compares the `value` it is handed
  against that `Decimal` and never reads `GameState`.

### Notation-specific notes (the four M1 targets)

- **Scientific** — mantissa base-10, exponent step 1.
- **Engineering** — same, but exponent forced to multiples of 3 (mantissa 1–999).
- **Standard** — `places` is **ignored** (`abbreviateStandard` keys only off the
  exponent → `K/M/B/T/Qa…` prefixes); uses `" "` separator and forces a non-negative
  exponent. Still consumes `placesExponent` once it overflows past the prefix table.
- **Letters** — subclass of Engineering: uses `places` for the 3-digit mantissa, then
  transcribes the `exponent/3` into a base-26 `a…z` string; **ignores
  `placesExponent`**.

### Resulting struct (M1)

```rust
struct FormatOptions {
    notation: Notation,              // Scientific | Engineering | Standard | Letters
    places: i32,                     // mantissa digits ≥ 1000   (per call; default 0)
    places_under_1000: i32,          // digits < 1000            (per call; default 0)
    places_exponent: i32,            // exponent's own digits    (game uses 3)
    exponent_commas: ExponentCommas, // { show, min, max }       (user option)
    inf_threshold: Option<Decimal>,  // value >= this -> "Infinite"; None = never
                                     //   (per-frame; caller derives from game state)
}
```

`inf_threshold` is the only field derived from `GameState`-adjacent info
(`hasBroken`, challenge state). The "formatting must not read `GameState`" rule still
holds: the **caller** computes the threshold and bakes it in; `ad-format` only compares
`value` against the handed-in `Decimal`. Default `None` (never show "Infinite"), which
is the right default for Python/notebook use. The router checks it in the big-number
branch, mirroring where JS calls `Settings.isInfinite`.

## Implementation plan — `ad-format` M1

Scope: a standalone, pure crate implementing the routing pipeline above plus the four
notations (Scientific, Engineering, Standard, Letters). No PyO3 / WASM / GUI rewiring
yet — those are later milestones in the A → C migration. The existing
`ad-gui::format_decimal` stays in place until C.

### Crate setup

- `crates/ad-format/Cargo.toml` — depends on `break_infinity` (path dep) only. No
  `serde`/`wasm`/`pyo3` in M1 (add behind features later).
- Add to the workspace `Cargo.toml` members.
- `lib.rs` re-exports `format`, `FormatOptions`, `Notation`, `ExponentCommas`.

### Module layout

```
crates/ad-format/src/
  lib.rs            // public API: format(value, &opts) -> String; re-exports
  options.rs        // FormatOptions, Notation enum, ExponentCommas + defaults
  router.rs         // exponent-based routing (the base Notation.format logic)
  mantissa.rs       // formatMantissaBaseTen, formatMantissaWithExponent,
                    //   fixMantissaOverflow, toEngineering helpers
  exponent.rs       // formatExponent + comma logic (noSpecialFormatting/showCommas)
  notations/
    scientific.rs
    engineering.rs
    standard.rs     // abbreviateStandard prefix tables
    letters.rs      // CustomNotation base-26 transcription
```

Model each notation as a `NotationStrategy` trait whose required method is
`format_decimal(&self, value, places, places_exponent, opts) -> String`, with the
shared `formatUnder1000` / very-small fallbacks as default methods. The `Notation`
enum dispatches to a `&'static` strategy (allocation-free, and `Notation` stays
`Copy`-able for `FormatOptions`).

### Port order (each step fidelity-tested before the next)

Status: steps 1–7 done. All four M1 notations are implemented and fidelity-checked
against the real JS package; the standalone `ad-fidelity` harness is the remaining
follow-up.

1. ✅ **Router + under-1000 / very-small / negative paths.** Notation-independent
   `toFixed`-style outputs; the exponent thresholds (`-300`, `3`) and sign handling
   (`router.rs`).
2. ✅ **Mantissa+exponent core** (`formatMantissaWithExponent`): the
   `realBase = base**steps` loop, the mantissa-overflow roll-over (`9.999… → 1e+1`),
   and the `exponent == 0` short-circuit (`mantissa.rs`).
3. ✅ **Scientific** (base 10, steps 1).
4. ✅ **Engineering** (base 10, steps 3) — `steps == 3` forces the exponent to
   multiples of 3, so no separate `toEngineering` is needed here.
5. ✅ **`formatExponent` + `exponentCommas`** — recursive exponent formatting and the
   comma thresholds (`exponent.rs` + the `format_exponent` default method).
6. ✅ **Standard** — `abbreviateStandard` prefix tables (`STANDARD_ABBREVIATIONS`,
   `STANDARD_PREFIXES`, `STANDARD_PREFIXES_2`) and the regex cleanup (ported as three
   manual non-overlapping passes — no `regex` dependency).
7. ✅ **Letters** — `CustomNotation` base-26 transcription plus `toEngineering`
   (computed directly on mantissa/exponent rather than via a `noNormalize` Decimal).

All four notations are validated against the real `@antimatter-dimensions/notations`
package (see `crates/ad-format/tests/notations.rs`); the standalone `ad-fidelity`
harness below is still outstanding.

### Testing / fidelity

- Unit tests per notation with hand-picked values across every routing branch:
  `0`, `< 1000`, sub-`1e-300`, negatives, the `9.9995e3` roll-over case, exponents at
  the comma `min`/`max` boundaries (`1e5`, `1e9`), and `1e1e15`-style double-exponent.
- Add a `format()` comparison harness to `ad-fidelity`: feed the same value + options
  to the Rust `format` and to the JS `Notations.<name>.format` (via the installed
  `ad-notations.esm.js`), assert string equality. Generate a spread of exponents
  (e.g. `10**k` and random mantissas for `k` in `-320..=350` and a few very large `k`).
- Keep the default `FormatOptions` (notation = Scientific, `places = 2`,
  `places_under_1000 = 0`, `places_exponent = 3`) matching how `ad-gui` currently
  renders, so a later swap is low-risk.

### Out of scope for M1 (tracked for later)

- PyO3 / WASM bindings and the GUI snapshot switch to raw numbers (migration step C).
- The remaining ~20 notations (Mixed, Logarithm, Roman, Emoji, …).
- The `isEND()` / `"END"` celestial special-case — game/UI state, belongs to the
  caller, not `ad-format`. (The `formatPreBreak` "Infinite" gating is handled via
  `inf_threshold`: `ad-format` does the comparison and emits `"Infinite"`, but the
  caller decides the threshold from game state — see the struct above.)

## Next steps (where to pick up)

The four M1 notations are implemented and curated-fidelity-tested. Remaining work,
roughly in priority order:

1. **`ad-fidelity` `format()` harness.** Distinct from the curated reference strings
   already in `crates/ad-format/tests/notations.rs`: drive a *random/broad* spread of
   `value × FormatOptions` through both the Rust `format` and the JS
   `Notations.<name>.format` (via the installed `ad-notations.esm.js`) and assert
   string equality. Exponents `10**k` with random mantissas for `k` in `-320..=350`
   plus a few very large `k`; vary `places` / `places_under_1000`. This is what will
   surface the `toFixed` rounding edge below at scale.
2. **`toFixed` rounding parity.** `format_mantissa_base_ten` uses Rust's `{:.*}`
   (round-half-to-even); JS `toFixed` rounds half-away-from-zero. The curated tests
   avoid exact `.5` boundaries, so they pass, but the two disagree on e.g. `1.005`.
   Pick a rounding helper that matches JS and replace the `format!` call (see the
   `// TODO(fidelity)` in `mantissa.rs`). Best done alongside step 1 so the harness
   proves it.
3. **Wire `ad-format` into `ad-gui` (migration A → C).** Replace
   `ad-gui::format_decimal` with `ad_format::format`; thread a `FormatOptions` from
   GUI state (notation picker, digit options, and the `inf_threshold` derived from
   game state). Per the design above this can stay Rust-side first (pre-baked
   snapshot) before taking on the WASM build step.
4. **PyO3 + WASM exposure (rest of step C).** Expose `format` + `FormatOptions` from
   `ad-python`; compile `ad-format` with `wasm-bindgen` and switch the snapshot to raw
   `{mantissa, exponent}`. Add the `serde` / `wasm` / `pyo3` integration behind crate
   features (M1 deliberately has none).
5. **More notations.** Add the remaining strategies incrementally (Mixed first, since
   it reuses Scientific/Engineering + Standard); each is a new `NotationStrategy` impl
   plus a `Notation` enum variant and reference tests.

Implementation notes for whoever resumes:

- Regenerate JS reference strings by requiring `@antimatter-dimensions/notations` in
  `../antimatter-dimensions` (run `npm install` there first if `node_modules` is
  absent); mirror the game call `notation.format(value, places, placesUnder1000, 3)`.
- The place counts (`places`, `places_under_1000`, `places_exponent`) are `u32`. JS
  uses a signed `-1` "unspecified" sentinel clamped via `Math.max(0, …)`; we don't
  need it, so non-negativity is enforced by the type rather than a runtime clamp.
- `Notation::name()` carries `#[allow(dead_code)]` until the fidelity harness consumes
  it for JS lookup.
