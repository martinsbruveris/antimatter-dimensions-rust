# Antimatter Dimensions Rust — Architecture Design

## 1. Goals & Constraints

### Primary Goals

1. **Learn Rust** — the project structure should expose idiomatic Rust patterns (traits, ownership, generics, error handling) rather than being a line-by-line JS translation.
2. **Fast simulation engine** — enable running thousands of game-hours in seconds for numerical experiments.
3. **Python bindings** — expose the engine to Python (via PyO3) for data analysis.
4. **Functional UI** — a playable frontend (not pixel-perfect, but useable).
5. **Fidelity testing** — automated comparison against the original JS game.

### Constraints

- Vendor an existing `break_infinity` Rust crate (or port one) for big number arithmetic.
- Defer the endgame mod — design for extensibility but only implement the base game.
- The original game is ~62k lines of core logic. A phased approach is essential.

---

## 2. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Cargo Workspace                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────┐   ┌──────────────────────────────────────────┐    │
│  │break_infinity│   │              ad-core                      │    │
│  │  (vendored)  │   │                                          │    │
│  │              │◄──┤  Game engine + static config (data mod)   │    │
│  │  Decimal type│   │  Pure logic, no IO                       │    │
│  └──────────────┘   └──────────────────┬───────────────────────┘    │
│                                        │                            │
│                         ┌──────────────┼──────────────┐             │
│                         │              │              │             │
│                         ▼              ▼              ▼             │
│             ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│             │   ad-gui     │  │  ad-python   │  │  ad-fidelity │   │
│             │  (egui app)  │  │  (PyO3)      │  │  (test       │   │
│             │              │  │              │  │   harness)   │   │
│             └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Crate Breakdown

| Crate | Type | Purpose |
|-------|------|---------|
| `break_infinity` | lib (vendored) | Decimal type: mantissa × 10^exponent arithmetic |
| `ad-core` | lib | Game simulation + static config. Includes a `data` module for costs, effects, thresholds. |
| `ad-gui` | bin | egui-based playable frontend |
| `ad-python` | cdylib | PyO3 bindings exposing the engine to Python |
| `ad-fidelity` | bin/lib | Fidelity test harness comparing Rust vs JS outputs |

> **Why not a separate `ad-data` crate?** Static configuration (dimension costs, upgrade
> definitions, effect formulas) is tightly coupled to game state types — effect evaluation
> needs access to `GameState`. A crate boundary would force either circular dependencies or
> an awkward shared-types crate. A `data` module inside `ad-core` keeps things organized
> while allowing direct access to internal types.

---

## 3. `break_infinity` — Number System

### Representation

```rust
/// A number represented as mantissa × 10^exponent.
/// Supports values up to approximately 1e9e15.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decimal {
    mantissa: f64,  // Normalized to [1, 10) or 0
    exponent: i64,
}
```

### Key Traits to Implement

- `Add`, `Sub`, `Mul`, `Div`, `Neg` (and `*Assign` variants)
- `PartialOrd`, `Ord` (total ordering with special NaN handling)
- `From<f64>`, `From<i64>`, `FromStr`
- `Display` (scientific notation formatting)
- `Serialize`, `Deserialize` (serde)
- Custom: `Decimal::pow10(exp)`, `Decimal::log10()`, `Decimal::pow(exp)`

### Vendoring Strategy

Search for an existing Rust port of `break_infinity.js`. If none exists or is incomplete, write one from scratch (~500–800 lines) based on the JS source. The interface is well-defined (see `break_infinity.d.ts`). Key operations needed by the game:

- Arithmetic: add, sub, mul, div
- Comparison: eq, lt, gt, cmp
- Powers: pow, pow10, exp, log10, log, sqrt
- Rounding: floor, ceil, round, trunc
- Utilities: abs, sign, clamp, max, min
- Formatting: toFixed, toPrecision, toString

---

## 4. Static Game Configuration (`ad-core::data`)

The JS game stores all configuration in `secret-formula/`. We mirror this as a `data` module inside `ad-core` with typed Rust data.

### Approach: Const Data + Enum-Dispatch Formulas

```rust
// ad-core/src/data/dimensions.rs

/// Cost configuration for an antimatter dimension tier.
pub struct DimensionCostConfig {
    pub base_cost: Decimal,
    pub cost_multiplier: Decimal,
    pub cost_scaling: CostScalingConfig,
}

/// All 8 antimatter dimension tiers.
pub const ANTIMATTER_DIMENSIONS: [DimensionCostConfig; 8] = [ /* ... */ ];
```

For configurations that include closures/formulas (like `effect: () => ...`), we use an enum-dispatch pattern:

```rust
/// An effect that can be evaluated given the current game state.
pub enum EffectFormula {
    /// Returns a constant value.
    Constant(Decimal),
    /// Multiplier based on time spent in current eternity.
    TimeInEternity { max_minutes: f64, multiplier: f64 },
    /// Computed from game state via a known formula variant.
    Computed(FormulaId),
}
```

This avoids `dyn Fn` closures while remaining data-driven. Each `FormulaId` maps to a pure function in `ad-core` that computes the effect value from game state — trivial when both live in the same crate.

---

## 5. `ad-core` — Game Engine

This is the heart of the project. It contains:

### 5.1 Game State

A single struct holding all mutable game state (analogous to `window.player`):

```rust
pub struct GameState {
    pub antimatter: Decimal,
    pub dimensions: DimensionsState,
    pub tickspeed: TickspeedState,
    pub dim_boosts: u32,
    pub galaxies: u32,
    pub sacrificed: Decimal,
    pub infinity: InfinityState,
    pub eternity: EternityState,
    pub reality: RealityState,
    pub challenges: ChallengeState,
    pub records: Records,
    pub autobuyers: AutobuyerState,
    pub options: GameOptions,
    // ...
}

pub struct DimensionsState {
    pub antimatter: [DimensionTier; 8],
    pub infinity: [DimensionTier; 8],
    pub time: [DimensionTier; 8],
}

pub struct DimensionTier {
    pub amount: Decimal,
    pub bought: u64,
    pub cost: Decimal,
}
```

### 5.2 Game Tick

The core simulation step. Designed to be called in a tight loop for batch simulation:

```rust
impl GameState {
    /// Advance the game by `dt` milliseconds of real time.
    /// Returns events that occurred (prestiges, unlocks, etc.)
    pub fn tick(&mut self, dt: f64) -> Vec<GameEvent> {
        let game_speed = self.compute_game_speed();
        let diff = dt * game_speed;

        let mut events = Vec::new();

        // Run autobuyers
        self.tick_autobuyers(diff, &mut events);

        // Production (order matters)
        self.tick_time_dimensions(diff);
        self.tick_infinity_dimensions(diff);
        self.tick_antimatter_dimensions(diff);

        // Replicanti, dilation, etc.
        self.tick_replicanti(diff);
        self.tick_dilation(diff);

        // Check auto-prestige conditions
        self.check_auto_prestige(&mut events);

        events
    }

    /// Run the simulation for `total_ms` of real time, using `tick_size` per step.
    /// This is the fast-path for numerical experiments.
    pub fn simulate(&mut self, total_ms: f64, tick_size: f64) -> SimulationLog {
        let steps = (total_ms / tick_size) as u64;
        let mut log = SimulationLog::new();
        for _ in 0..steps {
            let events = self.tick(tick_size);
            log.record_tick(self, &events);
        }
        log
    }
}
```

### 5.3 Multiplier Pipeline

The most complex part. Each dimension's multiplier is computed from dozens of sources.

**Design: Multiplier as a builder that queries game state**

```rust
pub struct MultiplierComputation {
    value: Decimal,
}

impl MultiplierComputation {
    pub fn new() -> Self {
        Self { value: Decimal::one() }
    }

    /// Multiply by an effect source if it's active.
    pub fn times_effect_of(&mut self, state: &GameState, source: EffectSource) -> &mut Self {
        if source.is_active(state) {
            self.value = self.value * source.effect_value(state);
        }
        self
    }

    /// Raise to a power from an effect source if active.
    pub fn pow_effect_of(&mut self, state: &GameState, source: EffectSource) -> &mut Self {
        if source.is_active(state) {
            self.value = self.value.pow(source.effect_value(state));
        }
        self
    }

    pub fn finish(self) -> Decimal {
        self.value
    }
}
```

**`EffectSource`** is an enum covering all ~200 effect sources:

```rust
pub enum EffectSource {
    Achievement(u32),
    TimeStudy(u32),
    InfinityChallenge(u32),
    InfinityChallengeReward(u32),
    EternityChallenge(u32),
    DilationUpgrade(u32),
    GlyphEffect(GlyphEffectType),
    AlchemyResource(AlchemyType),
    PelleUpgrade(u32),
    // ... all other sources
}

impl EffectSource {
    pub fn is_active(&self, state: &GameState) -> bool { /* ... */ }
    pub fn effect_value(&self, state: &GameState) -> Decimal { /* ... */ }
}
```

This is verbose but type-safe and exhaustive — the compiler ensures every effect source is handled.

### 5.4 Prestige System

```rust
pub trait PrestigeLayer {
    /// Check if prestige conditions are met.
    fn can_prestige(state: &GameState) -> bool;
    /// Calculate the reward for prestiging now.
    fn prestige_gain(state: &GameState) -> Decimal;
    /// Execute the prestige reset.
    fn prestige(state: &mut GameState) -> PrestigeResult;
}

pub struct InfinityPrestige;
pub struct EternityPrestige;
pub struct RealityPrestige;

impl PrestigeLayer for InfinityPrestige { /* ... */ }
```

### 5.5 Challenge System

Challenges modify game rules. Rather than scattering `if challenge.is_running()` checks, we use a modifier struct:

```rust
/// Active rule modifications from challenges, celestials, etc.
pub struct ActiveModifiers {
    pub max_dimension_tier: Option<u8>,       // e.g., NC7 limits to 6
    pub dimension_cost_override: Option<fn(tier: u8, state: &GameState) -> Decimal>,
    pub tickspeed_disabled: bool,              // NC9
    pub sacrifice_disabled: bool,              // NC10
    pub galaxy_cost_modifier: Option<GalaxyCostMod>,
    // ... one field per rule that challenges can modify
}

impl GameState {
    pub fn active_modifiers(&self) -> ActiveModifiers {
        let mut mods = ActiveModifiers::default();
        if let Some(nc) = self.challenges.active_normal {
            nc.apply_modifiers(&mut mods);
        }
        if let Some(ic) = self.challenges.active_infinity {
            ic.apply_modifiers(&mut mods);
        }
        // ... eternity challenges, celestials
        mods
    }
}
```

### 5.6 Caching Strategy

Use a tick-generation counter to invalidate cached computations:

```rust
pub struct Cache<T> {
    value: Option<T>,
    generation: u64,
}

impl<T> Cache<T> {
    pub fn get_or_compute(&mut self, current_gen: u64, f: impl FnOnce() -> T) -> &T {
        if self.generation != current_gen {
            self.value = Some(f());
            self.generation = current_gen;
        }
        self.value.as_ref().unwrap()
    }
}
```

The `GameState` holds a `tick_generation: u64` counter incremented each tick. Cached multipliers recompute only when the generation changes. For headless simulation at max speed, caching can be bypassed entirely since we tick sequentially.

---

## 6. `ad-gui` — Frontend

### Technology: egui + eframe

[egui](https://github.com/emilk/egui) is an immediate-mode Rust GUI library. Reasons for choosing it:

- **Pure Rust** — good for the learning goal, no JS/HTML/CSS
- **Cross-platform** — native (Windows/Mac/Linux) + compiles to WASM
- **Immediate mode** — simple mental model, no complex state management
- **Well-suited to data-heavy UIs** — tables, numbers, buttons

### Architecture

```rust
struct App {
    game: GameState,
    last_tick: Instant,
    ui_state: UiState,  // Tab selection, scroll positions, etc.
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Game tick
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_millis() as f64;
        self.game.tick(dt);
        self.last_tick = now;

        // UI rendering
        self.render_top_bar(ctx);
        self.render_tabs(ctx);

        // Request continuous repaint (~30fps)
        ctx.request_repaint_after(Duration::from_millis(33));
    }
}
```

### UI Tabs (matching the game's structure)

| Tab | Content |
|-----|---------|
| Dimensions | AD purchase buttons, amounts, multipliers, tickspeed |
| Infinity | Infinity dimensions, infinity upgrades, break infinity |
| Eternity | Time dimensions, time studies tree, dilation |
| Reality | Glyphs, perks, reality upgrades, celestials |
| Options | Save/load, settings |
| Statistics | Production rates, records, graphs |

The UI will be functional but minimal — no animations, no flavour text, just the mechanics.

---

## 7. `ad-python` — Python Bindings

### Interface Design (PyO3)

```rust
#[pyclass]
struct Game {
    state: GameState,
}

#[pymethods]
impl Game {
    #[new]
    fn new() -> Self { Game { state: GameState::new() } }

    /// Advance the game by dt milliseconds.
    fn tick(&mut self, dt: f64) -> Vec<String> { /* ... */ }

    /// Run simulation: total_ms at tick_size granularity.
    /// Returns a dict of logged data (antimatter, IP, EP over time).
    fn simulate(&mut self, total_ms: f64, tick_size: f64) -> PyResult<PyObject> { /* ... */ }

    /// Get current antimatter as a string (for display).
    #[getter]
    fn antimatter(&self) -> String { self.state.antimatter.to_string() }

    /// Get current antimatter as log10 (for numerical work).
    #[getter]
    fn antimatter_log10(&self) -> f64 { self.state.antimatter.log10() }

    /// Snapshot the full state as a dict.
    fn snapshot(&self) -> PyResult<PyObject> { /* ... */ }

    /// Load state from a dict (for reproducibility).
    fn load_snapshot(&mut self, data: PyObject) -> PyResult<()> { /* ... */ }

    /// Perform a prestige (infinity/eternity/reality).
    fn prestige(&mut self, layer: &str) -> PyResult<()> { /* ... */ }

    /// Buy a dimension, upgrade, etc.
    fn buy(&mut self, target: &str) -> PyResult<bool> { /* ... */ }
}
```

### 7.2 Constructing Game State from Python

The key challenge: `GameState` has ~50+ fields across nested layers, but any given experiment
only needs to set a few. We support three complementary approaches.

#### Approach 1: Named Presets

Pre-built starting points representing common game phases:

```python
import ad_python as ad

# Fresh game
game = ad.Game()

# Named presets for key progression milestones
game = ad.Game.preset("first_infinity")      # Just reached 1e308 AM
game = ad.Game.preset("break_infinity")      # Break Infinity unlocked
game = ad.Game.preset("first_eternity")      # Just reached 1e308 IP
game = ad.Game.preset("mid_eternity")        # Has time studies, replicanti
game = ad.Game.preset("first_reality")       # Just reached 1e4000 EP
game = ad.Game.preset("mid_reality")         # Has glyphs, perks, some celestials
```

Presets are defined in Rust as named `GameState` constructors. They're curated to represent
"interesting starting points" for analysis — not every possible state, but enough to skip
boring early-game grinding.

#### Approach 2: Partial Dict Override (merge with defaults)

Start from a preset (or fresh game) and override specific fields:

```python
# Start from a preset, override specific fields
game = ad.Game.from_config({
    "preset": "break_infinity",
    "antimatter": "1e1000",
    "galaxies": 50,
    "dim_boosts": 20,
    "infinity": {
        "infinity_points": "1e50",
        "break_infinity": True,
        "upgrades": [1, 2, 3, 4, 5, 6, 7, 8],  # upgrade IDs
    },
    "dimensions": {
        "antimatter": {
            "1": {"bought": 100},
            "8": {"bought": 80},
        }
    }
})
```

**Rules:**
- Unspecified fields inherit from the preset (or defaults if no preset given).
- Decimal values accept strings (`"1e308"`), floats, or ints.
- Nested dicts merge recursively — you only specify what you want to change.
- Lists (upgrades, time studies) are set wholesale, not merged.

#### Approach 3: Mutation After Construction

For iterative experimentation in notebooks:

```python
game = ad.Game.preset("first_eternity")

# Mutate specific fields
game.set_antimatter("1e1000")
game.set_infinity_points("1e100")
game.set_galaxies(200)
game.set_dim_boosts(50)

# Bulk-set upgrades, time studies, etc.
game.set_infinity_upgrades([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
game.set_time_studies([11, 21, 22, 31, 32, 33, 41, 42, 51, 61, 62, 71, 72, 73])
game.enable_break_infinity()

# Clone for A/B comparison
game_a = game.clone()
game_b = game.clone()
game_b.set_time_studies([11, 21, 22, 31, 32, 33, 41, 42, 51, 61, 62, 71, 72, 73, 91])
```

#### Approach 4: Import from JS Save

For maximum fidelity — load an actual save from the browser game:

```python
# Paste the base64 save string from the JS game
game = ad.Game.from_js_save("AntimatterDimensionsSavefileFormat...")

# Or from a decoded JSON save
game = ad.Game.from_js_save_json({"antimatter": "1e1234", ...})
```

This requires implementing the JS save format parser (base64 → JSON → GameState mapping),
but provides the highest-fidelity starting points for analysis. It also enables the workflow:
"play the game in browser → export save → analyse in Python."

#### State Validation

After construction, the engine validates the state is internally consistent:

```python
game = ad.Game.from_config({...})
# Raises ValueError if:
# - Upgrade prerequisites not met
# - Dimension tiers unlocked without sufficient boosts
# - Challenge completions reference non-existent challenges
# - etc.

# Or with lenient mode for "I know what I'm doing" experiments:
game = ad.Game.from_config({...}, validate=False)
```

#### Snapshot & Restore (Reproducibility)

```python
# Save full state for reproducibility
snapshot = game.snapshot()  # Returns a dict with ALL fields

# Later: exact restore
game2 = ad.Game.from_snapshot(snapshot)

# Snapshots are JSON-serializable for saving to disk
import json
with open("experiment_start.json", "w") as f:
    json.dump(snapshot, f)
```

### 7.3 Usage Example: Comparative Experiment

```python
import ad_python as ad
import pandas as pd

# Question: How much does Time Study 91 affect antimatter growth post-eternity?
base = ad.Game.preset("mid_eternity")

# Variant A: without TS91
game_a = base.clone()
game_a.set_time_studies([11, 21, 22, 33, 41, 42, 51, 61, 62])

# Variant B: with TS91
game_b = base.clone()
game_b.set_time_studies([11, 21, 22, 33, 41, 42, 51, 61, 62, 91])

# Simulate 10 minutes each
data_a = game_a.simulate(total_ms=600_000, tick_size=100)
data_b = game_b.simulate(total_ms=600_000, tick_size=100)

# Compare
df = pd.DataFrame({
    "time_s": [t / 1000 for t in data_a["time_ms"]],
    "without_ts91": data_a["antimatter_log10"],
    "with_ts91": data_b["antimatter_log10"],
})
df.plot(x="time_s", y=["without_ts91", "with_ts91"])
```

---

## 8. `ad-fidelity` — Automated Fidelity Testing

### Approach: Record & Compare

The fidelity testing system works by running both the JS original and the Rust engine through identical scenarios and comparing their outputs.

```
┌────────────┐         ┌────────────────┐        ┌──────────────┐
│  Scenario  │────────►│  JS Runner     │───────►│  JS Trace    │
│  (JSON)    │         │  (Node.js)     │        │  (JSON)      │
│            │────────►│  Rust Runner   │───────►│  Rust Trace  │
└────────────┘         └────────────────┘        └──────┬───────┘
                                                         │
                                                         ▼
                                                 ┌──────────────┐
                                                 │  Comparator  │
                                                 │  (tolerance) │
                                                 └──────────────┘
```

### Scenario Definition

```json
{
  "name": "basic_dimension_growth",
  "initial_state": "new_game",
  "actions": [
    { "tick": 1000 },
    { "buy": "antimatter_dimension_1" },
    { "tick": 5000 },
    { "buy": "antimatter_dimension_1", "repeat": 10 },
    { "tick": 60000 }
  ],
  "checkpoints": [
    { "after_action": 0, "capture": ["antimatter", "dimensions.antimatter"] },
    { "after_action": 2, "capture": ["antimatter"] },
    { "after_action": 4, "capture": ["antimatter", "dimensions.antimatter"] }
  ]
}
```

### JS Runner

A Node.js script that:
1. Loads the game's core modules (stripped of Vue/DOM dependencies)
2. Executes the scenario actions
3. Captures state at checkpoints
4. Outputs a JSON trace

```javascript
// ad-fidelity/js-runner/run-scenario.js
const { createHeadlessGame } = require('./headless-game');

async function runScenario(scenarioPath) {
    const scenario = JSON.parse(fs.readFileSync(scenarioPath));
    const game = createHeadlessGame(scenario.initial_state);

    const trace = [];
    for (const [i, action] of scenario.actions.entries()) {
        executeAction(game, action);
        const checkpoint = scenario.checkpoints.find(c => c.after_action === i);
        if (checkpoint) {
            trace.push(captureState(game, checkpoint.capture));
        }
    }
    return trace;
}
```

### Rust Runner

```rust
fn run_scenario(scenario: &Scenario) -> Vec<StateSnapshot> {
    let mut game = GameState::from_preset(&scenario.initial_state);
    let mut trace = Vec::new();

    for (i, action) in scenario.actions.iter().enumerate() {
        execute_action(&mut game, action);
        if let Some(checkpoint) = scenario.checkpoint_at(i) {
            trace.push(game.snapshot(&checkpoint.capture));
        }
    }
    trace
}
```

### Comparison with Tolerance

Since floating-point arithmetic between JS and Rust will differ slightly (especially with `break_infinity` reimplementation), comparisons use relative tolerance:

```rust
fn values_match(js: &Decimal, rust: &Decimal, tolerance: f64) -> bool {
    if js.is_zero() && rust.is_zero() { return true; }
    let js_log = js.log10();
    let rust_log = rust.log10();
    // Compare in log-space: relative error on the exponent
    (js_log - rust_log).abs() / js_log.abs().max(1.0) < tolerance
}
```

Default tolerance: 1e-10 (essentially exact for integer operations, loose enough for transcendental functions).

### Randomness Analysis

The game engine contains very little gameplay-affecting randomness. This is good news for
fidelity testing — most of the game is a pure function of state + time.

#### Seeded RNG (Deterministic)

| System | RNG Type | Notes |
|--------|----------|-------|
| **Glyph generation** | Seeded xorshift32 | Seed stored in `player.reality.seed`. Fully deterministic given the same seed. |

The glyph RNG is the only seeded system. It produces identical results across JS and Rust
if the same seed and xorshift32 implementation are used.

#### Unseeded RNG (`Math.random()` / `fastRandom()`)

| System | Location | Impact |
|--------|----------|--------|
| **Replicanti (slow path)** | `replicanti.js:232-244` | Binomial/Poisson sampling at low replicanti count + slow interval. Only active in early replicanti or Celestial 7. |
| **Alchemy (unpredictability)** | `alchemy.js:213` | Poisson draw for reaction count. Only with Ra's unpredictability resource active. |
| **Reality perk point bonus** | `reality.js:303` | Binomial draw for bonus realities/PP from Achievement 154. |
| **Simulated reality glyph selection** | `reality.js:195, 395` | Random glyph choice during auto-reality (when player isn't choosing). |
| **Companion glyph effect** | `glyph-effects.js:664-665` | Purely cosmetic — the "happiness" display value. |

#### Determinism by Game Phase

| Phase | Deterministic? | Notes |
|-------|---------------|-------|
| Pre-Infinity (dimensions, tickspeed, galaxies) | ✅ Fully | No randomness |
| Infinity (ID, challenges, break infinity) | ✅ Fully | No randomness |
| Eternity (TD, time studies, dilation) | ✅ Fully | No randomness |
| Replicanti (fast path: amount ≥ 1000) | ✅ Fully | Uses continuous approximation |
| Replicanti (slow path: amount < 1000) | ❌ Stochastic | Binomial/Poisson sampling |
| Reality (glyph generation) | ✅ Seeded | Reproducible with same seed |
| Reality (perk point bonus) | ❌ Stochastic | Minor: only with Achievement 154 |
| Celestials (alchemy) | ❌ Stochastic | Only with Ra unpredictability |

#### Implications for Fidelity Testing

1. **Phases 1–4 (pre-Reality) can use exact comparison.** No randomness tolerance
   needed — only floating-point precision differences from the Decimal implementation.

2. **Glyph generation is reproducible** — supply the same `player.reality.seed` and the
   xorshift32 sequence matches exactly between JS and Rust.

3. **Strategies for stochastic systems:**
   - **Preferred: Force deterministic paths in test scenarios.** E.g., ensure replicanti
     amount > 1000 so the fast (deterministic) code path is used.
   - **Alternative: Inject seeded RNG.** Replace `fastRandom()` / `Math.random()` with a
     seeded PRNG in both runners, using the same seed.
   - **Fallback: Statistical tolerance.** Run N trials, compare distributions rather than
     exact values.

   The first strategy covers 99% of real gameplay. The stochastic replicanti path is only
   active during a brief window of early replicanti growth.

---

## 9. Module Dependency & Build Order

```
Phase 1 — Foundation:
  break_infinity → ad-core (GameState + data::constants + tick skeleton)

Phase 2 — Core Simulation:
  ad-core: antimatter dimensions + tickspeed + dim boosts + galaxies + sacrifice
  ad-gui: basic dimension tab (shows production, buy buttons)

Phase 3 — First Prestige:
  ad-core: infinity + infinity dimensions + infinity power + normal challenges
  ad-gui: infinity tab
  ad-fidelity: first scenario tests

Phase 4 — Second Prestige:
  ad-core: eternity + time dimensions + time studies + infinity challenges
  ad-gui: eternity tab

Phase 5 — Mid-game:
  ad-core: replicanti + dilation + eternity challenges
  ad-python: basic bindings

Phase 6 — Reality:
  ad-core: reality + glyphs + perks + automator + celestials
  ad-gui: reality tab + celestial tabs
  ad-python: full simulation API
```

---

## 10. Key Design Decisions

### 10.1 No ECS

An Entity-Component-System (Bevy, specs) is overkill here. The game has a fixed, well-known set of entities (8 AD tiers, 8 ID tiers, 8 TD tiers, etc.). A plain struct with named fields is simpler, faster, and easier to understand.

### 10.2 No `dyn` for Hot Paths

Effect computation runs every tick for every dimension. Using trait objects (`Box<dyn Effect>`) would add indirection and prevent inlining. Instead, `EffectSource` is an enum — match arms compile to a jump table, and the compiler can inline common cases.

### 10.3 Immutable Config, Mutable State

`ad-data` is entirely `const`/`static` — game configuration never changes at runtime. `ad-core`'s `GameState` is the only mutable piece. This separation makes it trivial to reason about what can change.

### 10.4 Deterministic Simulation

The engine must be fully deterministic given the same inputs. No `SystemTime`, no random number generation without a seeded RNG. This enables:
- Reproducible numerical experiments
- Fidelity testing
- Save/load via state serialization

### 10.5 Serialization

`GameState` derives `serde::Serialize` + `Deserialize`. Save files are MessagePack (compact, fast). JSON export is available for debugging and Python interop.

### 10.6 Frontend as a Thin Shell

The egui frontend owns a `GameState` and calls `tick()` each frame. It reads state for display but never computes game logic itself. This ensures the headless engine is always the source of truth.

---

## 11. Workspace Layout

```
antimatter-dimensions-rust/
├── Cargo.toml                  # Workspace manifest
├── design-docs/                # Architecture & analysis docs
├── crates/
│   ├── break_infinity/         # Vendored big-number library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── decimal.rs      # Core Decimal type
│   │       ├── arithmetic.rs   # Add/Sub/Mul/Div
│   │       ├── transcendental.rs # Log, Pow, Exp
│   │       └── format.rs       # Display, FromStr
│   ├── ad-core/                # Game simulation engine + config
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── state.rs        # GameState struct
│   │       ├── tick.rs         # Main game loop
│   │       ├── data/           # Static game configuration
│   │       │   ├── mod.rs
│   │       │   ├── dimensions.rs   # Dimension configs (costs, multipliers)
│   │       │   ├── upgrades.rs     # All upgrade definitions
│   │       │   ├── challenges.rs   # Challenge definitions
│   │       │   ├── time_studies.rs # Time study tree
│   │       │   ├── celestials.rs   # Celestial configs
│   │       │   └── constants.rs    # DC equivalents
│   │       ├── dimensions/     # Dimension production & purchasing
│   │       ├── prestige/       # Infinity, Eternity, Reality
│   │       ├── challenges/     # Challenge system
│   │       ├── multipliers.rs  # Effect composition pipeline
│   │       ├── tickspeed.rs
│   │       ├── galaxy.rs
│   │       ├── replicanti.rs
│   │       ├── dilation.rs
│   │       ├── autobuyers.rs
│   │       ├── cache.rs        # Tick-generation caching
│   │       └── simulate.rs     # Batch simulation & logging
│   ├── ad-gui/                 # egui frontend
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── app.rs          # eframe::App implementation
│   │       └── tabs/           # One module per UI tab
│   └── ad-python/              # PyO3 bindings
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
├── fidelity/                   # Fidelity testing
│   ├── scenarios/              # Scenario JSON files
│   ├── js-runner/              # Node.js headless runner
│   │   ├── package.json
│   │   ├── headless-game.js    # Game core without DOM
│   │   └── run-scenario.js
│   └── src/                    # Rust comparator (or in ad-core tests)
│       └── main.rs
└── analysis/                   # Python notebooks/scripts
    ├── requirements.txt
    └── notebooks/
```

---

## 12. Testing Strategy

### Unit Tests (in each crate)

- `break_infinity`: Exhaustive arithmetic tests, edge cases (zero, infinity, very small numbers), comparison with JS implementation output.
- `ad-core`: Test individual systems in isolation:
  - Data module: validate all configs load, no missing references
  - Dimension production for one tick with known multipliers
  - Cost scaling formulas
  - Prestige gain calculations
  - Challenge modifier application

### Integration Tests

- Full game tick sequences: "buy 10 AD1, tick 60s, check antimatter matches expected"
- Prestige cycle tests: "run to infinity, crunch, verify IP gain and reset"

### Fidelity Tests

- Scenario-based comparison against JS (as described in §8)
- Run as CI — any drift beyond tolerance fails the build

### Property Tests (proptest/quickcheck)

- Decimal arithmetic: `a + b - b ≈ a`, `a * b / b ≈ a`, ordering consistency
- Game invariants: antimatter never negative, bought counts monotonically increase

---

## 13. Performance Considerations

### Batch Simulation Hot Path

For numerical experiments, the simulation loop must be fast:

1. **Avoid allocation** — pre-allocate vectors, reuse buffers
2. **Minimize branching** — precompute which effects are active before the tick loop
3. **SIMD potential** — 8 dimension tiers updating in parallel (future optimization)
4. **No logging overhead** — use conditional compilation (`#[cfg(feature = "trace")]`)

### Target Performance

The JS game runs at ~30 ticks/second. A Rust engine should achieve:
- **>100,000 ticks/second** in headless mode (enabling 1 hour of game time per second at 33ms ticks)
- **>1,000,000 ticks/second** for simplified numerical analysis (no autobuyers, no events)

This enables running full game progressions (weeks of game time) in seconds.

---

## 14. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| break_infinity precision differences | Fidelity drift | Log-space comparison with tolerance; test early |
| Multiplier pipeline complexity | Bugs, slow progress | Implement incrementally per phase; fidelity tests catch drift |
| Challenge interactions | Subtle bugs | Centralized modifier system; test each challenge in isolation |
| egui limitations for complex UI | Poor UX | Start minimal; the UI is secondary to the engine |
| Python bindings complexity | Maintenance burden | Expose only high-level API; use serde for state transfer |
| 62k lines to port | Multi-month effort | Phased approach; each phase is independently useful |

---

## 15. Open Questions

1. **Which `break_infinity` crate to vendor?** — Need to search crates.io/GitHub more thoroughly. If nothing suitable exists, writing one from the JS source is ~1 week of work.
2. **Save compatibility with original game?** — Probably not worth pursuing, but supporting import of JS save files (JSON) into the Rust engine would enable interesting "pick up where you left off" analysis.
3. **Automator (scripting language)?** — The original game has a custom scripting language for automation. This is a large, self-contained subsystem. Defer to Phase 6 or later.
4. **Graph rendering in egui?** — For the statistics tab, egui has `egui_plot` which should suffice for basic time-series.

---

*Document created: 2026-06-19*
