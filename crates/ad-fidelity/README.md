# ad-fidelity

Fidelity test harness that verifies the Rust `ad-core` implementation matches the
original JavaScript [Antimatter Dimensions](https://ivark.github.io/AntimachDim/) game.

## Overview

The crate contains two kinds of tests:

- **Fixture-driven tests** (`tests/fixture_tests.rs`) — compare `ad-core` against
  reference values produced by running the actual JS game code.
- **Analytical tests** (`tests/section*.rs`) — verify individual formulas using
  hand-computed expected values derived from the JS source.

The fixture-driven tests are the primary fidelity mechanism. The analytical tests
provide finer-grained coverage and are useful during development.

## Directory Structure

```
ad-fidelity/
├── src/
│   ├── lib.rs              # Crate root
│   └── tolerance.rs        # Log-space comparison utilities
├── tests/
│   ├── fixture_tests.rs    # Fixture-driven tests (reads pre-infinity.json)
│   ├── section1_dimension_costs.rs
│   ├── section2_buy10_multiplier.rs
│   ├── section3_dimboost.rs
│   ├── section4_tickspeed.rs
│   ├── section5_galaxies.rs
│   ├── section6_sacrifice.rs
│   └── section7_production.rs
├── fixtures/
│   └── pre-infinity.json   # Reference values from the JS game
└── js-harness/
    ├── package.json
    ├── shims.js            # Global shims for pre-infinity context
    ├── loader.js           # Loads actual JS source files
    └── generate-fixtures.js
```

## Prerequisites

Generating fixtures requires:

1. **Node.js** (v18+)
2. **The original game source** at `../../../antimatter-dimensions/` (sibling to the
   workspace root) with `npm install` already run.

Running the Rust tests only requires the pre-generated `fixtures/pre-infinity.json` file,
which is checked into the repository.

## Generating Fixtures

The fixture generator loads the actual Antimatter Dimensions JS source files
(`constants.js`, `antimatter-dimension.js`, `dimboost.js`, `sacrifice.js`,
`tickspeed.js`, `galaxy.js`) via Node's `vm` module with minimal shims for
globals that are inactive in pre-infinity (challenges, upgrades, achievements, etc.).

```bash
# From the workspace root
cd crates/ad-fidelity/js-harness

# Ensure game dependencies are installed
(cd ../../../../antimatter-dimensions && npm install)

# Generate fixtures
node generate-fixtures.js
```

This writes `fixtures/pre-infinity.json` containing 218 reference values across
7 sections:

| Section | Entries | What it covers |
|---------|---------|----------------|
| Dimension costs | 88 | Per-10-purchase cost scaling for all 8 tiers |
| Buy-10 multiplier | 6 | Base multiplier and scaling with purchases |
| Dim boost multiplier | 72 | Tier-dependent boost formula at various boost counts |
| Dim boost requirements | 11 | Required tier and amount for each boost level |
| Tickspeed multiplier | 11 | Galaxy effect on tickspeed (linear and exponential) |
| Galaxy requirements | 8 | AD8 requirement for each galaxy |
| Sacrifice | 15 | `totalBoost` and `nextBoost` formulas |
| Dimension multipliers | 7 | Full multiplier including buy-10, dim boost, sacrifice |

## Running Tests

```bash
# Run all fidelity tests
cargo test -p ad-fidelity

# Run only fixture-driven tests
cargo test -p ad-fidelity --test fixture_tests

# Run only a specific section
cargo test -p ad-fidelity --test section4_tickspeed
```

## How the JS Harness Works

The game's source files use ES module syntax (`import`/`export`) and reference many
globals. The harness handles this by:

1. **`shims.js`** sets up the global scope with inactive stubs for all game systems
   (challenges, upgrades, achievements, celestials, etc.). In pre-infinity, all these
   return identity values (multiplier = 1, effect = 0, `isRunning` = false).

2. **`loader.js`** reads each game source file, strips `import`/`export` statements,
   replaces `window.` with `global.`, and evaluates the transformed source via
   `vm.Script.runInThisContext()`. This makes classes like `DimBoost`, `Sacrifice`,
   `Galaxy`, and `AntimatterDimension` available in the global scope.

3. **`generate-fixtures.js`** sets up specific game states (e.g., 10 galaxies,
   50 bought AD1) and calls the real game functions to capture reference values.

## Tolerance

Comparisons use log-space relative tolerance:

```
|log10(rust_value) - log10(js_value)| < epsilon
```

- `EPSILON_EXACT` (1e-10): for single formula evaluations
- `EPSILON_SIMULATION` (1e-6): for multi-step simulations with accumulated error

## Adding New Test Scenarios

1. Add the scenario to `generate-fixtures.js` with appropriate player state setup.
   Remember that `AntimatterDimension(tier)` is 1-indexed and reads from
   `player.dimensions.antimatter[tier - 1]`.
2. Regenerate: `node generate-fixtures.js`
3. Add a corresponding test in `fixture_tests.rs` that reads the new fixture data.

If the game code references a new global that isn't shimmed, you'll get a
`ReferenceError`. Add the missing shim to `shims.js`.
