# ad-fidelity

Fidelity test harness that verifies the Rust `ad-core` implementation matches the
original JavaScript [Antimatter Dimensions](https://ivark.github.io/AntimachDim/) game.

## Overview

Fidelity is checked with a **save-replay** harness (design:
[`docs/design/2026-07-06-fidelity-testing.md`](../../docs/design/2026-07-06-fidelity-testing.md)):
capture real savefiles from a manual playthrough, then replay each in both the
original JS game and Rust, ticking forward and diffing the persisted state.

The harness has three stages:

1. **Capture** ([`capture/`](capture/)) — an in-browser userscript (speed controls
   + time-based save capture) plus a local server that stores the POSTed saves.
2. **Oracle** ([`oracle/`](oracle/)) — a Playwright script that boots the real game
   in headless Chromium, deterministically ticks each save to fixed horizons, and
   writes the expected post-tick saves as reference fixtures.
3. **Rust comparison** (`src/` + the `ad-fidelity` binary) — replays the same
   saves through `ad-core`, ticks to the same horizons, and diffs the persisted
   `player` tree against the oracle fixtures with per-field tolerance.

## Directory Structure

```
ad-fidelity/
├── src/
│   ├── lib.rs              # Crate root + re-exports
│   ├── main.rs             # `ad-fidelity` CLI (table / verbose comparison)
│   ├── compare.rs          # Tolerant per-field diff walker + comparison modes
│   ├── allowlist.rs        # The player-tree fields that are compared (design §5)
│   ├── fixture.rs          # Loading oracle fixtures + replaying saves via ad-core
│   ├── run.rs              # (fixtures × horizons) comparison grid orchestration
│   ├── report.rs           # Table + verbose renderers
│   └── tolerance.rs        # Log-space comparison primitives
├── tests/
│   └── replay_smoke.rs     # End-to-end plumbing tests (no Node needed)
├── capture/               # Stage 1: capture rig (userscript + save server)
│   ├── userscript.js       # Speed buttons + time-based capture (in-browser)
│   └── save-server.js      # Local server that stores POSTed saves
├── oracle/                # Stage 2: Playwright oracle (reference fixtures)
│   └── generate-replay-fixtures.js
└── saves/                 # Data (git-ignored)
    ├── captures/           # Captured savefiles + index.jsonl
    └── fixtures/           # Oracle reference fixtures (<save>.json)
```

## Prerequisites

Running the oracle requires:

1. **Node.js** (v18+)
2. **The original game source** at `../../../antimatter-dimensions/` (sibling to the
   workspace root) with `npm install` already run.

## Capture

See [`capture/README.md`](capture/README.md). The userscript adds speed controls and
periodically POSTs the current savefile to `save-server.js`, which writes each into
`saves/captures/` and appends an entry to `index.jsonl`.

## Oracle

See [`oracle/README.md`](oracle/README.md). The oracle runs the **actual** JS game in
headless Chromium, ticks each captured save forward deterministically, and writes the
resulting savefiles as fixtures the Rust harness will diff against.

```bash
cd crates/ad-fidelity/oracle
npm install                 # pulls Playwright
npx playwright install chromium
npm run generate            # reads ../saves/captures, writes ../saves/fixtures
```

## Rust comparison (the `ad-fidelity` binary)

Once the oracle has written `saves/fixtures/`, replay them through `ad-core` and diff:

```bash
# From the workspace root; defaults to saves/fixtures at every horizon present.
cargo run -p ad-fidelity                       # pass/fail table
cargo run -p ad-fidelity -- --verbose          # + per-field divergences
cargo run -p ad-fidelity -- path/to/fixtures   # a different fixtures dir
```

The default output is a grid — one row per fixture, one column per horizon
(tick count), each cell `ok` / `FAIL`:

```
#  fixture              1     10    100   1000
1  01_pre_big_crunch    ok    ok    FAIL  FAIL
2  …

7/8 cells passed (1 diverged)
```

`--verbose` lists, for each failing cell, the fields that diverged with their JS
(expected) and Rust (actual) values and the delta. The process exits non-zero if
any cell fails, so it drops into CI once fixtures exist.

Options:

| Flag | Meaning |
|------|---------|
| `[DIR]` | Fixtures directory (default `saves/fixtures`). |
| `--tests 1,3,12` | Only these fixtures, by 0-based row index. |
| `--ticks 1,10` | Only these horizons (columns). |
| `--tick-ms 50` | Override the fixture's `meta.tickMs` (must match the oracle). |
| `--epsilon 1e-6` | Log-space / relative comparison epsilon. |
| `--roundtrip` | Add an `rt` column: Rust decode→encode of the input vs the input itself — the identity guard (design §6) that isolates encode/decode bugs from tick bugs. |
| `-v`, `--verbose` | Per-field failure detail. |

### What is compared

Only the persisted `player`-tree fields on the **allowlist** (`src/allowlist.rs`,
design §5) — the AM economy, Infinity/Eternity/Replicanti/Dilation, the
unlock-gating records, autobuyer settings, and (partially) Reality/black holes.
Everything else (options/UI, unported systems, `Date.now`/real-time and game-time
bookkeeping, values derived from a primary) is intentionally ignored. Each field
has a comparison mode (`src/compare.rs`, design §8): `Exact` (ints/bools/bitmasks,
compared by numeric *value* so `0` and `0.0` agree), `Decimal` (log-space
relative tolerance), `Number` (relative tolerance), `IdSet` (order-insensitive),
and `Glyphs` (object-array matched by slot).

## Tolerance

The comparison uses log-space relative tolerance for Decimal fields:

```
|log10(rust_value) - log10(js_value)| < epsilon
```

The `--epsilon` flag sets it (default `1e-6`). The [`tolerance`](src/tolerance.rs)
module also exposes the historical constants `EPSILON_EXACT` (1e-10, single
formula evaluations) and `EPSILON_SIMULATION` (1e-6, accumulated error). The
[`Tolerance`](src/compare.rs) type keeps the shape general — epsilon can grow
with the horizon (constant or linear in tick count, design §10) — so the
constants can be fixed empirically once the oracle produces data.
