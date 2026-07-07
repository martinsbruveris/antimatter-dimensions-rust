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
3. **Rust comparison** (not yet built) — replays the same saves through `ad-core`,
   ticks to the same horizons, and diffs against the oracle fixtures.

## Directory Structure

```
ad-fidelity/
├── src/
│   ├── lib.rs              # Crate root
│   └── tolerance.rs        # Log-space comparison utilities (for the Rust diff)
├── capture/               # Stage 1: capture rig (userscript + save server)
│   ├── userscript.js       # Speed buttons + time-based capture (in-browser)
│   ├── save-server.js      # Local server that stores POSTed saves
│   └── captures/           # Captured savefiles + index.jsonl
└── oracle/                # Stage 2: Playwright oracle (reference fixtures)
    └── generate-replay-fixtures.js
```

## Prerequisites

Running the oracle requires:

1. **Node.js** (v18+)
2. **The original game source** at `../../../antimatter-dimensions/` (sibling to the
   workspace root) with `npm install` already run.

## Capture

See [`capture/README.md`](capture/README.md). The userscript adds speed controls and
periodically POSTs the current savefile to `save-server.js`, which writes each into
`capture/captures/` and appends an entry to `index.jsonl`.

## Oracle

See [`oracle/README.md`](oracle/README.md). The oracle runs the **actual** JS game in
headless Chromium, ticks each captured save forward deterministically, and writes the
resulting savefiles as fixtures the Rust harness will diff against.

```bash
cd crates/ad-fidelity/oracle
npm install                 # pulls Playwright
npx playwright install chromium
npm run generate            # reads captured saves, writes ./fixtures
```

## Tolerance

The (upcoming) Rust comparison uses log-space relative tolerance for numeric fields:

```
|log10(rust_value) - log10(js_value)| < epsilon
```

- `EPSILON_EXACT` (1e-10): for single formula evaluations
- `EPSILON_SIMULATION` (1e-6): for multi-step simulations with accumulated error
