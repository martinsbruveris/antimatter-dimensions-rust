---
status: Implemented
---

# Experiment Architecture: Strategy-Based Simulation

## 1. Problem Statement

We want to measure the time to reach the Big Crunch (first Infinity: antimatter ≥
1.7977e308) under different buying strategies.

**Setup:** An infinitely-fast human player — every game tick, the player can make as many
purchases as affordable. No autobuyer cooldowns. The strategic choices to explore:

1. **Sacrifice threshold** — At what multiplier gain ratio is sacrifice worthwhile?
2. **Tickspeed vs dimension priority** — Discount factor when comparing costs: should I
   save up for tickspeed or buy cheaper dimensions?
3. **Dimension buy order** — High dimensions first, low first, or cheapest first?
4. **Prestige plan** — A prescribed sequence of dim boosts and galaxies before the Big
   Crunch (e.g., "4 boosts → galaxy → 3 boosts → galaxy → crunch").

## 2. Architecture Overview

Three layers, matching the existing workspace plan:

```
┌─────────────────────────────────────────────────────┐
│  Python experiment scripts                           │
│  (parameter sweeps, plotting, analysis)              │
├─────────────────────────────────────────────────────┤
│  ad-python  (PyO3 crate)                             │
│  Exposes: StrategyConfig, SimulationResult, simulate │
├─────────────────────────────────────────────────────┤
│  ad-core                                             │
│  ├── strategy.rs   — strategy configuration types    │
│  └── simulator.rs  — simulation loop with strategy   │
└─────────────────────────────────────────────────────┘
```

**Key principle:** All game logic stays in `ad-core`. The Python layer is configuration +
results only — no per-tick callbacks across the FFI boundary.

## 3. Strategy Configuration

The strategy is a **config struct**, not a trait. All experiment variations are
parameterized — no need for custom Rust code per experiment.

```rust
// ad-core/src/strategy.rs

/// Complete configuration for a buying strategy.
pub struct StrategyConfig {
    pub sacrifice: SacrificeConfig,
    pub purchase: PurchaseConfig,
    pub prestige: PrestigeMode,
}

/// When to sacrifice dimensions.
pub struct SacrificeConfig {
    pub enabled: bool,
    /// Sacrifice when new_multiplier / old_multiplier > threshold.
    /// E.g., threshold=2.0 means only sacrifice for a 2x gain.
    pub min_gain_ratio: f64,
}

/// How to spend antimatter between prestige events.
pub struct PurchaseConfig {
    /// How to decide between tickspeed and dimensions.
    pub priority: BuyPriority,
    /// Which dimension to buy when buying dimensions.
    pub dimension_order: DimensionOrder,
}

/// Tickspeed vs dimension priority.
pub enum BuyPriority {
    /// Compare effective costs:
    ///   effective_tickspeed_cost = tickspeed_cost / tickspeed_weight
    ///   effective_dim_cost = dimension_cost
    /// Buy whichever is cheaper. Loop until nothing affordable.
    ///
    /// weight > 1 → prefer tickspeed (it appears cheaper)
    /// weight < 1 → prefer dimensions
    /// weight = 1 → pure cost comparison
    Weighted { tickspeed_weight: f64 },
}

/// Order in which to evaluate dimension purchases.
pub enum DimensionOrder {
    HighestFirst, // Buy highest unlocked tier first
    LowestFirst,  // Buy lowest tier first
    CheapestFirst, // Buy whichever costs least
}

/// How to handle prestige events (dim boosts and galaxies).
pub enum PrestigeMode {
    /// Buy dim boosts and galaxies whenever affordable.
    /// Galaxy is prioritised over dim boost.
    Auto,
    /// Follow a prescribed sequence of prestige events.
    /// After all steps are exhausted, continue buying until Big Crunch.
    Plan(Vec<PrestigeStep>),
}

/// A step in the prestige plan (used with PrestigeMode::Plan).
pub enum PrestigeStep {
    /// Buy N dimension boosts (accumulating between each).
    DimBoost(u32),
    /// Buy one antimatter galaxy.
    Galaxy,
}
```

### Strategy Execution Order (per tick)

Within each tick, the strategy executes in this priority order, looping until nothing
more can be bought:

1. **Prestige check:** In `Auto` mode, buy a galaxy if affordable, else buy a dim boost.
   In `Plan` mode, execute the current plan step if achievable. Either way, a prestige
   event restarts the loop from step 1.
2. **Sacrifice check:** If enabled and gain ratio exceeds threshold, sacrifice.
3. **Purchase:** Compare tickspeed cost (adjusted by weight) against the cheapest eligible
   dimension purchase. Buy max of the cheaper option.
4. **Repeat** from step 1 (a dim boost resets dimensions, so new purchases become
   available).

Step 1 is checked first because prestige events reset progress — there's no point buying
dimensions if we're about to reset. However, during the "accumulation phase" (building
toward the next prestige requirement), steps 2-3 do the work.

### Prestige Modes

**`PrestigeMode::Auto`** — The baseline mode. Buys galaxies and dim boosts whenever
affordable, prioritising galaxies (since they provide a permanent tickspeed improvement).
This is what an optimal idle player would do.

**`PrestigeMode::Plan(steps)`** — A prescribed sequence of prestige events. Useful for
experiments comparing specific boost/galaxy orderings. The plan is a flat sequence
tracked by a cursor:

```
Plan: [DimBoost(2), Galaxy, DimBoost(3), Galaxy]

Execution:
  Phase 1: Buy dims/tickspeed → dim boost → buy dims/tickspeed → dim boost
  Phase 2: Buy dims/tickspeed → galaxy (resets everything)
  Phase 3: Buy dims/tickspeed → dim boost → ... (3 times)
  Phase 4: Buy dims/tickspeed → galaxy
  Phase 5: Buy dims/tickspeed → Big Crunch
```

During a DimBoost(N) phase, dim boosts are bought one at a time (each triggers a reset,
requiring rebuilding). During a Galaxy phase, no extra dim boosts are bought — only
dims/tickspeed to reach the galaxy requirement.

**Open question:** Should we allow extra dim boosts during a Galaxy phase? They provide
production multipliers that help reach the galaxy requirement faster, but each one
triggers a reset. This could be a boolean flag on the Galaxy variant:

```rust
Galaxy { allow_extra_boosts: bool }
```

For now, keep it simple: no extra boosts during galaxy phases. They can be added as
explicit DimBoost steps before the Galaxy step.

## 4. Simulator

### State Trace (Adaptive Downsampling)

The simulation returns a trace of `GameState` snapshots over time. Since storing every
tick is too large, we use an adaptive downsampling algorithm controlled by a single
parameter `snapshot_count`:

**Algorithm:** The buffer has capacity `2 * snapshot_count`. When it fills up, we discard
every second entry (keeping entries at even indices) and double the recording interval.
This ensures:

- The returned trace has between `snapshot_count` and `2 * snapshot_count` entries
- Snapshots are approximately uniformly spaced in time
- Resolution adapts automatically to the total simulation length
- Memory usage is bounded regardless of simulation duration

```rust
/// A single state snapshot in the trace.
pub struct Snapshot {
    pub tick: u64,
    pub time_ms: f64,
    pub state: GameState,
}

/// Streaming downsampling buffer for state traces.
pub struct StateTrace {
    buffer: Vec<Snapshot>,
    /// Buffer capacity = 2 * snapshot_count
    capacity: usize,
    /// Current recording interval (in ticks): 1, 2, 4, 8, ...
    interval: u64,
    /// Next tick number at which to record
    next_record_tick: u64,
}

impl StateTrace {
    pub fn new(snapshot_count: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(2 * snapshot_count),
            capacity: 2 * snapshot_count,
            interval: 1,
            next_record_tick: 0,
        }
    }

    /// Called every tick. Records the state if it's time.
    pub fn maybe_record(
        &mut self,
        tick: u64,
        time_ms: f64,
        state: &GameState,
    ) {
        if tick < self.next_record_tick {
            return;
        }

        self.buffer.push(Snapshot {
            tick,
            time_ms,
            state: state.clone(),
        });
        self.next_record_tick = tick + self.interval;

        // Compact when buffer is full
        if self.buffer.len() >= self.capacity {
            // Keep every other entry (even indices)
            let mut i = 0;
            self.buffer.retain(|_| {
                let keep = i % 2 == 0;
                i += 1;
                keep
            });
            self.interval *= 2;
        }
    }

    pub fn into_snapshots(self) -> Vec<Snapshot> {
        self.buffer
    }
}
```

**Example:** With `snapshot_count = 500`, the buffer holds up to 1000 entries. A
simulation running 7.2M ticks compacts ~13 times, ending with ~500–1000 snapshots spaced
~7000–14000 ticks apart (~5.8–11.7 minutes of game time at 50ms/tick).

### Simulation Loop

```rust
// ad-core/src/simulator.rs

/// Configuration for a simulation run.
pub struct SimulationConfig {
    /// Buying strategy.
    pub strategy: StrategyConfig,
    /// Time step in milliseconds.
    pub tick_ms: f64,
    /// Approximate number of state snapshots to return.
    /// Actual count will be between this and 2x this.
    /// Set to 0 to disable tracing.
    pub snapshot_count: usize,
}

pub struct SimulationResult {
    /// Total game time elapsed (milliseconds).
    pub total_time_ms: f64,
    /// Number of simulation ticks executed.
    pub total_ticks: u64,
    /// Final game state summary.
    pub final_galaxies: u32,
    pub final_dim_boosts: u32,
    pub final_antimatter: Decimal,
    /// State trace (adaptive resolution).
    pub trace: Vec<Snapshot>,
}

/// Run a complete simulation from a fresh game until Big Crunch.
pub fn simulate(config: &SimulationConfig) -> SimulationResult {
    let mut game = GameState::new();
    let mut time_ms = 0.0;
    let mut ticks = 0u64;
    let mut plan = PlanCursor::new(&config.strategy.prestige_plan);
    let mut trace = StateTrace::new(config.snapshot_count);

    loop {
        // 1. Execute strategy (buy everything possible)
        execute_strategy(&mut game, &config.strategy, &mut plan);

        // 2. Check Big Crunch
        if game.antimatter >= BIG_CRUNCH_THRESHOLD {
            return SimulationResult {
                total_time_ms: time_ms,
                total_ticks: ticks,
                final_galaxies: game.galaxies,
                final_dim_boosts: game.dim_boosts,
                final_antimatter: game.antimatter,
                trace: trace.into_snapshots(),
            };
        }

        // 3. Record state (if it's time)
        trace.maybe_record(ticks, time_ms, &game);

        // 4. Advance one tick (autobuyers are disabled, so this is production only)
        game.tick(config.tick_ms);
        time_ms += config.tick_ms;
        ticks += 1;
    }
}
```

### Required Changes to ad-core

1. **Autobuyer enable/disable** — Add a mechanism to disable all autobuyers on
   `GameState`. The simulator creates a fresh `GameState` with autobuyers disabled, then
   uses the existing `tick()` method (which skips autobuyers when they're disabled). This
   is also needed for the regular game (players toggle autobuyers on/off).

2. **`BIG_CRUNCH_THRESHOLD`** — New constant: `Decimal::new(1.7976931348623157, 308)` (JS
   `Number.MAX_VALUE`).

3. **`sacrifice_gain_ratio()`** — New query: `sacrifice_multiplier_if_sacrificed() /
   sacrifice_multiplier()`. Already computable from existing methods.

### Performance Estimate

| Game time | tick_ms=50 | tick_ms=10 |
|-----------|-----------|-----------|
| 1 hour    | 72K ticks | 360K ticks |
| 10 hours  | 720K      | 3.6M       |
| 100 hours | 7.2M      | 36M        |

Each tick is ~50 arithmetic operations. At ~1ns per op, 7.2M ticks ≈ 0.36s. Even a
100-hour game should simulate in under a second on modern hardware.

For parameter sweeps with 1000 configurations, total wall time with sequential execution:
~6 minutes at tick_ms=50. With Rayon parallelism (8 cores): ~45 seconds.

### Future Optimization: Adaptive Time-Stepping

If performance becomes a bottleneck, we can skip ahead when nothing is affordable:

```rust
// When nothing can be bought, estimate time to next purchase:
let time_to_afford = (cheapest_cost - game.antimatter)
    / game.antimatter_per_second();
// Advance by that time (capped for safety)
let skip = time_to_afford.min(max_skip_ms);
game.tick_production(skip);
```

This would reduce ticks by 10-100x during "waiting" phases. Not needed initially — the
tick-based approach is fast enough.

## 5. Python API (ad-python)

### Crate Setup

```toml
# crates/ad-python/Cargo.toml
[package]
name = "ad-python"

[lib]
crate-type = ["cdylib"]
name = "ad_python"

[dependencies]
ad-core = { path = "../ad-core" }
pyo3 = { version = "0.23", features = ["extension-module"] }
```

### Python Interface

```python
import ad_python as ad
import numpy as np
import matplotlib.pyplot as plt

# --- Baseline: auto-prestige with sacrifice ---
config = ad.SimulationConfig(
    strategy=ad.StrategyConfig(
        sacrifice_enabled=True,
        sacrifice_threshold=10.0,
        tickspeed_weight=1.0,
        dimension_order="highest_first",
        prestige_mode="auto",
    ),
    tick_ms=50.0,
    snapshot_count=500,  # returns 500–1000 snapshots
)
result = ad.simulate(config)
print(f"Time: {result.total_time_s:.1f}s")
print(f"Galaxies: {result.galaxies}, Dim boosts: {result.dim_boosts}")
print(f"Trace points: {len(result.trace)}")

# --- Fixed prestige plan ---
config2 = ad.SimulationConfig(
    strategy=ad.StrategyConfig(
        sacrifice_enabled=True,
        sacrifice_threshold=10.0,
        tickspeed_weight=1.0,
        dimension_order="highest_first",
        prestige_mode=["boost:4", "galaxy", "boost:3", "galaxy"],
    ),
    tick_ms=50.0,
    snapshot_count=500,
)

# --- Plotting state over time ---
times = [s.time_ms / 1000 for s in result.trace]
antimatter = [s.antimatter_log10 for s in result.trace]
plt.plot(times, antimatter)
plt.xlabel("Time (s)")
plt.ylabel("log₁₀(antimatter)")

# --- Batch simulation (future work, parallel via Rayon) ---
# configs = [
#     ad.SimulationConfig(
#         strategy=ad.StrategyConfig(sacrifice_threshold=t, ...),
#         tick_ms=50.0,
#         snapshot_count=0,  # disable trace for sweep (only need final time)
#     )
#     for t in np.logspace(0, 3, 100)
# ]
# results = ad.simulate_batch(configs)
```

### Prestige Mode Format

For ergonomic Python usage, the prestige mode is specified as either the string `"auto"`
or a list of prestige steps:

| Value       | Meaning                             |
|-------------|-------------------------------------|
| `"auto"`    | Auto-buy boosts and galaxies        |
| `["boost:N", "galaxy", ...]` | Follow a fixed plan |

Within a plan, `"boost:N"` means buy N dimension boosts, `"galaxy"` means buy one
galaxy.

Example: `["boost:4", "galaxy", "boost:3", "galaxy", "boost:2"]`

### Batch API (Future Work)

`simulate_batch(configs)` will run all simulations in parallel using Rayon's thread pool,
returning a list of `SimulationResult` in the same order as inputs. This is the planned
primary API for parameter sweeps. The `simulate()` function is designed to be `Send` +
pure (no shared mutable state), so parallelizing with Rayon will be straightforward when
needed.

## 6. Implementation Plan

### Phase 1: Strategy + Simulator in ad-core

1. Add `strategy.rs` — strategy config types (all the enums/structs above)
2. Add autobuyer enable/disable to `GameState` (useful for both simulator and regular game)
3. Add `BIG_CRUNCH_THRESHOLD` constant
4. Add `simulator.rs` — `simulate()` function + `execute_strategy()` + `PlanCursor`
5. Integration tests: verify a known strategy reaches Big Crunch

### Phase 2: ad-python crate (PyO3)

1. Create `crates/ad-python/` with PyO3 boilerplate
2. Expose `StrategyConfig` with Python-friendly constructors
3. Expose `simulate()`
4. Test from Python: single run

### Phase 3 (Future): Batch simulation + experiment notebooks

1. Add `rayon` dependency and `simulate_batch()` to ad-python
2. Sacrifice threshold sweep
3. Tickspeed weight sweep
4. Dimension order comparison
5. Prestige plan comparison (varying boost/galaxy sequences)
6. 2D sweeps (e.g., sacrifice threshold × tickspeed weight)

## 7. Design Decisions & Rationale

**Config struct vs trait for strategy:** Config struct. All experiments are parameterized
variations of the same decision tree. A trait would require writing Rust code for each
experiment. The config is serializable and Python-friendly.

**Tick-based vs event-driven simulation:** Tick-based (with configurable dt). Simpler to
implement, easy to verify against the existing game loop, and fast enough in Rust.
Event-driven can be added later as an optimization.

**No per-tick Python callbacks:** The FFI boundary is crossed only twice per simulation
(config in, result out). This keeps the simulation fast (~1M ticks/sec) rather than
bottlenecking on Python↔Rust calls.

**Prestige plan as flat sequence:** Simple and declarative. The user specifies exactly
what prestige events to execute and in what order. No conditional logic — that's what
parameter sweeps are for.

**`BuyPriority::Weighted` with single parameter:** The `tickspeed_weight` parameter is
intuitive: "how much do I value tickspeed relative to dimensions?" A weight of 2.0 means
"I'd pay up to 2x more for tickspeed." This avoids needing to compute actual
cost-effectiveness ratios (which depend on game state in complex ways).

## 8. Implementation Notes

Phases 1 and 2 are complete. Key implementation details and lessons learned:

### Files Added/Modified

| File | Change |
|------|--------|
| `ad-core/src/strategy.rs` | Strategy config types (new) |
| `ad-core/src/simulator.rs` | Simulation engine (new) |
| `ad-core/src/autobuyers.rs` | Global `enabled` flag on `AutobuyerState` |
| `ad-core/src/data/constants.rs` | `big_crunch_threshold()` function |
| `ad-core/src/lib.rs` | Module exports |
| `ad-core/tests/simulator.rs` | 7 tests (StateTrace + simulation) |
| `crates/ad-python/` | PyO3 crate exposing `simulate()` (new) |
| `pyproject.toml` | Maturin build config (new) |

### Performance: Buy-Max is Essential

The initial implementation bought one tickspeed or one buy-until-10 per iteration of the
purchase loop. This caused the simulation to hang: with large antimatter, hundreds of
tickspeed purchases at 10× cost growth meant thousands of loop iterations per tick.

**Fix:** Use `buy_max_tickspeed()` and `buy_max_dimension(tier)` in the buy loop. After
buying max of the preferred option, re-evaluate priorities with remaining antimatter.
This reduced per-tick strategy execution from seconds to microseconds.

### Affordability Check: Unit Cost, Not Cost-Until-10

The `best_dimension()` function must check single-unit cost, not `cost_until_10`. At
game start with 10 antimatter, AD1 unit cost is 10 (affordable) but cost_until_10 is 100
(not affordable). Using cost_until_10 caused the simulation to never buy any dimensions,
leaving antimatter permanently at 10.

### PrestigeMode::Auto as Baseline

A fixed prestige plan requires knowing the correct number of boosts and galaxies in
advance. Missing even one early boost leads to exponentially longer wait times. The
`PrestigeMode::Auto` mode (buy galaxy if affordable, else buy dim boost) provides the
correct baseline — it dynamically determines the right prestige sequence.

### Baseline Result

Configuration: `PrestigeMode::Auto`, sacrifice threshold = 10, tickspeed weight = 1.0,
dimension order = highest first, tick_ms = 50.

| Metric | Value |
|--------|-------|
| Game time to Big Crunch | 8098s (2.2 hours) |
| Simulation ticks | 161,969 |
| Wall time (release) | ~0.16s |
| Final galaxies | 2 |
| Final dim boosts | 16 |
| Final antimatter | ~10^308.5 |

This represents an "infinitely fast player" who makes optimal purchases every tick with
no autobuyer cooldowns. The 2.2-hour game time is the theoretical minimum for reaching
first infinity with this strategy.
