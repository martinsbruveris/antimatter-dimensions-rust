---
status: Reference
---

# Antimatter Dimensions — Codebase Analysis

## Table of Contents

1. [Overview](#1-overview)
2. [Technology Stack & Repository Structure](#2-technology-stack--repository-structure)
3. [Architecture](#3-architecture)
4. [Number System](#4-number-system)
5. [Core Game Mechanics](#5-core-game-mechanics)
   - 5.1 [Dimensions](#51-dimensions)
   - 5.2 [Tickspeed](#52-tickspeed)
   - 5.3 [Dimension Boosts & Galaxies](#53-dimension-boosts--galaxies)
   - 5.4 [Sacrifice](#54-sacrifice)
6. [Prestige Layers](#6-prestige-layers)
   - 6.1 [Infinity (Big Crunch)](#61-infinity-big-crunch)
   - 6.2 [Eternity](#62-eternity)
   - 6.3 [Reality](#63-reality)
7. [Mid-to-Late Game Systems](#7-mid-to-late-game-systems)
   - 7.1 [Replicanti](#71-replicanti)
   - 7.2 [Time Dilation](#72-time-dilation)
   - 7.3 [Time Studies](#73-time-studies)
   - 7.4 [Glyphs](#74-glyphs)
   - 7.5 [Celestials](#75-celestials)
8. [Game Loop](#8-game-loop)
9. [Currency System](#9-currency-system)
10. [Data & Configuration Layer](#10-data--configuration-layer)
11. [Player State](#11-player-state)
12. [Challenges](#12-challenges)
13. [Automation (Autobuyers)](#13-automation-autobuyers)
14. [Key Architectural Patterns](#14-key-architectural-patterns)
15. [Considerations for a Rust Rewrite](#15-considerations-for-a-rust-rewrite)

---

## 1. Overview

Antimatter Dimensions is an incremental/idle game built as a Vue.js web application. The
player produces "antimatter" using cascading tiers of dimensions, with each tier
producing the tier below it. The game features a deep prestige hierarchy (Infinity →
Eternity → Reality) and extensive endgame content through the Celestials system.

### Codebase Size

| Category                  | Files | Lines of Code |
|---------------------------|------:|-------------:|
| JavaScript (src/)         |   270 |       66,797 |
| Vue components (src/)     |   409 |       52,333 |
| Core game logic (src/core)|   238 |       62,372 |
| Secret formula (data)     |    ~50|       28,637 |
| **Total**                 |  ~679 |     ~119,130 |

The game logic is overwhelmingly in `src/core/` (~62k lines). The Vue components handle
only presentation and user interaction. This separation is good news for a Rust rewrite —
the entire game simulation can be extracted independently of the UI.

---

## 2. Technology Stack & Repository Structure

**Stack:** Vue 2 + Webpack, with `break_infinity.js` for arbitrary-precision Decimal
numbers.

```
antimatter-dimensions/
├── public/                     # Static assets
├── build/                      # Build scripts
├── src/
│   ├── main.js                 # App entry point
│   ├── game.js                 # Main game loop + prestige formulas (1,100 lines)
│   ├── components/             # Vue UI components (409 files)
│   ├── core/                   # ALL game logic (238 files, 62k lines)
│   │   ├── dimensions/         # Antimatter, Infinity, Time dimension classes
│   │   ├── celestials/         # Endgame celestial mechanics
│   │   ├── autobuyers/         # Automation system
│   │   ├── glyphs/             # Glyph system (generation, effects, auto-processing)
│   │   ├── time-studies/       # Time study tree
│   │   ├── game-mechanics/     # Base classes (Effect, GameMechanicState, Purchasable)
│   │   ├── secret-formula/     # Game data / constants / configurations
│   │   ├── storage/            # Save/load system
│   │   ├── currency.js         # Currency abstraction
│   │   ├── player.js           # Player save state structure
│   │   ├── constants.js        # Pre-cached Decimal constants (DC.*)
│   │   ├── math.js             # Bulk-buy math, cost scaling, utility functions
│   │   ├── tickspeed.js        # Tickspeed mechanics
│   │   ├── galaxy.js           # Galaxy purchasing and scaling
│   │   ├── dimboost.js         # Dimension boost mechanics
│   │   ├── sacrifice.js        # Sacrifice mechanic
│   │   ├── big-crunch.js       # Infinity prestige
│   │   ├── eternity.js         # Eternity prestige
│   │   ├── reality.js          # Reality prestige
│   │   ├── dilation.js         # Time dilation
│   │   ├── replicanti.js       # Replicanti mechanics
│   │   └── ...
│   ├── steam/                  # Steam/Electron platform integration
│   └── utility/                # General utilities
├── package.json
└── vue.config.js
```

**Key dependencies:**
- `break_infinity.js` — Decimal library handling numbers up to ~1e9e15
- `chevrotain` — Parser library (used for the in-game automator scripting language)
- `firebase` — Cloud save support
- `codemirror` — Code editor for the automator

---

## 3. Architecture

The architecture follows a **data-driven game mechanics** pattern:

```
┌─────────────────────────────────────────────────────────────┐
│                    secret-formula/                           │
│  (Static config: costs, effects, thresholds, descriptions)  │
└──────────────────────┬──────────────────────────────────────┘
                       │ consumed by
┌──────────────────────▼──────────────────────────────────────┐
│                  game-mechanics/                             │
│  Effect → GameMechanicState → PurchasableMechanicState       │
│  (Runtime wrappers: isUnlocked, effectValue, purchase())    │
└──────────────────────┬──────────────────────────────────────┘
                       │ composed into
┌──────────────────────▼──────────────────────────────────────┐
│                   Core systems                               │
│  Dimensions, Currencies, Challenges, Prestige, Celestials   │
│  (Game-specific logic, formulas, state transitions)         │
└──────────────────────┬──────────────────────────────────────┘
                       │ orchestrated by
┌──────────────────────▼──────────────────────────────────────┐
│                    game.js                                    │
│  gameLoop() — ticks production, autobuyers, prestige checks │
└──────────────────────┬──────────────────────────────────────┘
                       │ renders via
┌──────────────────────▼──────────────────────────────────────┐
│                  Vue components                              │
│  Presentation layer only — no game logic                    │
└─────────────────────────────────────────────────────────────┘
```

### Core Design Principles

1. **Config-driven mechanics**: Game data (costs, effects, unlock conditions) lives in
   `secret-formula/`. Runtime classes in `game-mechanics/` wrap these configs into
   stateful objects with `isUnlocked`, `effectValue`, `purchase()`, etc.

2. **Effect composition**: Multipliers are composed using helper methods like
   `timesEffectsOf(...)`, `powEffectsOf(...)`, `plusEffectsOf(...)` which iterate over
   any number of effect sources and apply them if active.

3. **Cache invalidation**: Expensive multiplier calculations are cached in `GameCache`
   and invalidated each tick before production runs.

4. **Event-driven resets**: Prestige resets dispatch events via `EventHub`, allowing
   systems to react to resets without tight coupling.

---

## 4. Number System

The game uses `break_infinity.js`, which represents numbers as `(mantissa, exponent)`
pairs supporting values up to approximately `1e9e15`. This is exposed globally as
`window.Decimal`.

### Pre-cached Constants

`src/core/constants.js` defines ~100 frequently-used Decimal values frozen in a `DC`
object to avoid repeated allocations:

```javascript
export const DC = deepFreeze({
  D0:    new Decimal("0"),
  D1:    new Decimal("1"),
  D2:    new Decimal("2"),
  D0_965: new Decimal("0.965"),
  E308:  new Decimal("1e308"),
  // ... ~100 more
});
```

### MathOperations Abstraction

`currency.js` defines a `MathOperations` polymorphism layer with `number` and `decimal`
backends, so that `Currency` methods work identically whether the underlying value is a
JS `number` or a `Decimal`:

```javascript
class NumberMathOperations extends MathOperations {
  add(left, right) { return left + right; }
  // ...
}
class DecimalMathOperations extends MathOperations {
  add(left, right) { return Decimal.add(left, right); }
  // ...
}
```

### Custom Math Utilities

`src/core/math.js` (1,489 lines) provides:
- **`bulkBuyBinarySearch()`** — Binary search for how many items can be bulk-purchased
  with non-linear pricing
- **`ExponentialCostScaling`** — A class modeling exponential cost curves with optional
  super-exponential scaling past a threshold
- **`LinearCostScaling`** — For simpler linear+exponential cost models
- **`FreeTickspeed`** — Calculates free tickspeed upgrades from time shards
- Various helpers: `decimalCostBuyAll()`, `sumGeometricSeries()`, logarithmic utilities

---

## 5. Core Game Mechanics

### 5.1 Dimensions

The game has three types of dimensions, each with 8 tiers:

#### Production Chain

```
AD8 → AD7 → AD6 → AD5 → AD4 → AD3 → AD2 → AD1 → Antimatter
TD8 → TD7 → TD6 → TD5 → TD4 → TD3 → TD2 → TD1 → Time Shards
ID8 → ID7 → ID6 → ID5 → ID4 → ID3 → ID2 → ID1 → Infinity Power
```

Higher tiers produce the tier directly below. Tier 1 produces the associated currency.

#### Base Class: `DimensionState`

```javascript
// src/core/dimensions/dimension.js
class DimensionState {
  get amount()     // Current amount of this dimension
  get bought()     // Total purchased count
  get productionPerSecond()  // Abstract — tier-specific
  productionForDiff(diff) {
    return this.productionPerSecond.times(diff / 1000);
  }
  produceDimensions(dimension, diff) {
    dimension.amount += this.productionForDiff(diff);
  }
}
```

#### Antimatter Dimensions

**File:** `src/core/dimensions/antimatter-dimension.js` (676 lines)

**Cost model:**
```javascript
// Base costs per tier: [10, 100, 1e4, 1e6, 1e9, 1e13, 1e18, 1e24]
// Cost multipliers:    [1e3, 1e4, 1e5, 1e6, 1e8, 1e10, 1e12, 1e15]
// Cost = baseCost * baseMultiplier^(bought/10)
// with ExponentialCostScaling for late-game super-scaling
```

**Production formula:**
```javascript
get productionPerSecond() {
  let amount = this.totalAmount;
  let production = amount.times(this.multiplier).times(Tickspeed.perSecond);
  // Challenge-specific modifiers...
  return production;
}
```

**Multiplier computation** (`getDimensionFinalMultiplierUncached(tier)`):
1. Start with `DC.D1`
2. Apply common multiplier (achievements, shop, infinity power, break-infinity upgrades,
   time studies, infinity/eternity challenges)
3. Apply buy-10 multiplier: `buyTenMultiplier^(bought/10)`
4. Apply dimension boost multiplier
5. Apply tier-specific effects (sacrifice for tier 8, various achievements)
6. Apply power effects (glyphs, alchemy, achievements, celestial nerfs)
7. Apply dilation (if active): `dilatedValueOf(multiplier)`
8. Apply celestial modifiers (Effarig, V nerfs)

**Purchasing:** Dimensions are bought individually or in batches of 10. Every 10
purchases triggers the `buy10Multiplier`. Bulk-buying uses
`ExponentialCostScaling.getMaxBought()`.

#### Infinity Dimensions

**File:** `src/core/dimensions/infinity-dimension.js` (415 lines)

- Unlocked progressively (require antimatter + IP thresholds)
- Purchased with Infinity Points
- Cost scaling uses `LinearCostScaling`
- Production feeds into Infinity Power, which provides a multiplier to all Antimatter
  Dimensions

#### Time Dimensions

**File:** `src/core/dimensions/time-dimension.js` (347 lines)

- Tiers 1–4 unlocked with Eternity; tiers 5–8 require Time Studies
- Purchased with Eternity Points
- Production feeds into Time Shards → free Tickspeed upgrades
- Cost scaling has an additional breakpoint at `e6000` for super-exponential growth

### 5.2 Tickspeed

**File:** `src/core/tickspeed.js` (268 lines)

Tickspeed is a global multiplier applied to all Antimatter Dimension production. It
starts at one "tick per second" and increases via purchases and galaxies.

**Galaxy-based tickspeed multiplier:**
```javascript
// For galaxies >= 3:
// baseMultiplier = 0.8
// perGalaxy = 0.965
// tickspeedMultiplier = perGalaxy^(galaxies - 2) * baseMultiplier
// Each galaxy makes the multiplier ~3.5% smaller (faster production)
```

The effective galaxy count includes player galaxies + replicanti galaxies + tachyon
galaxies.

### 5.3 Dimension Boosts & Galaxies

**Dimension Boosts** (`src/core/dimboost.js`, 263 lines):
- Require a certain amount of the highest unlocked dimension
- The first 4 boosts unlock new dimension tiers (5th through 8th)
- Each boost multiplies all lower dimensions:
  ```javascript
  // multiplier = boostPower^(purchasedBoosts + 1 - tier)
  ```
- Resets all dimension amounts and antimatter

**Antimatter Galaxies** (`src/core/galaxy.js`, ~200 lines):
- Require 8th dimension amounts (scaling with galaxy count)
- Each galaxy improves the tickspeed multiplier
- Cost formula with three scaling regions:
  - **Normal** (galaxies < `costScalingStart`): `baseCost + galaxies * costMult`
  - **Distant** (quadratic scaling): adds `galaxies² + galaxies`
  - **Remote** (exponential scaling past galaxy 800): multiplied by `1.002^(galaxies -
    799)`
- Reset: all dimensions, boosts, antimatter, tickspeed purchases

### 5.4 Sacrifice

**File:** `src/core/sacrifice.js` (149 lines)

Sacrifice consumes all 1st dimension amount to boost 8th dimension production:

```javascript
// Normal formula: ((log10(AD1) / 10) / max(log10(sacrificed) / 10, 1))^exponent
// Post-IC2:       (AD1 / sacrificed)^exponent
// exponent = base * (1 + ach32 + ach57) * (1 + ach88 + TS228) * triad304
```

---

## 6. Prestige Layers

The game has a hierarchical prestige structure. Each higher layer resets all lower layers
but grants powerful new resources and multipliers.

```
Reality  (resets everything below, grants Reality Machines + Glyphs)
  └─ Eternity  (resets everything below, grants Eternity Points)
       └─ Infinity / Big Crunch  (resets dimensions, grants Infinity Points)
            └─ Galaxy  (resets dimensions, improves tickspeed)
                 └─ Dimension Boost  (resets dimensions, unlocks/multiplies dimensions)
```

### 6.1 Infinity (Big Crunch)

**File:** `src/core/big-crunch.js` + `src/game.js`

**Trigger:** Antimatter reaches `1e308` (Number.MAX_VALUE), or the relevant challenge
goal.

**Infinity Points formula:**
```javascript
// Pre-break: IP = 308 / div  (a constant)
// Post-break: IP = 10^(log10(maxAM) / div - 0.75)
// where div starts at 308, reduced by Achievement(103) and TimeStudy(111)
// Then multiplied by totalIPMult (many sources)
```

**What resets:** Antimatter, all Antimatter Dimensions, tickspeed, dimension boosts,
galaxies.

**What persists:** Infinity Points, infinity upgrades (conditionally), challenge
completions, achievements.

### 6.2 Eternity

**File:** `src/core/eternity.js` (357 lines)

**Trigger:** Infinity Points reach `1e308`.

**Eternity Points formula:**
```javascript
// EP = 5^(log10(maxIP + gainedIP) / (308 - pelleRecursion) - 0.7) * totalEPMult
```

**What resets:** Everything from Infinity, plus: Infinity Points, Infinity Dimensions,
Infinity upgrades (conditional), Replicanti.

**What persists:** Eternity Points, Time Dimensions, Time Studies, Eternity Challenges
completions, Eternity milestones.

### 6.3 Reality

**File:** `src/core/reality.js` (843 lines)

**Trigger:** Eternity Points reach `1e4000`.

**Rewards:** Reality Machines, Perk Points, a Glyph, and the Automator.

**What resets:** Everything from Eternity, plus: Eternity Points, Time Studies, Time
Dimensions, Dilation state, Eternity Challenges.

**What persists:** Reality Machines, Glyphs, Perks, Reality Upgrades, Black Holes,
Celestial progress.

---

## 7. Mid-to-Late Game Systems

### 7.1 Replicanti

**File:** `src/core/replicanti.js` (576 lines)

Replicanti are a self-replicating resource that provides multipliers and galaxies.

- Grow exponentially based on an interval and a chance
- Cap at `1e308` by default; reaching the cap enables Replicanti Galaxy purchases
- Each Replicanti Galaxy acts like an Antimatter Galaxy for tickspeed
- Growth slows down past `1e308` with configurable `scaleFactor` per `scaleLog10` OoMs
- Fast-path code handles both sub-308 and above-308 growth with bulk galaxy purchasing

### 7.2 Time Dilation

**File:** `src/core/dilation.js` (273 lines)

Time Dilation is a modified eternity run where all dimension multipliers are "dilated":

```javascript
// Dilation formula:
function dilatedValueOf(value) {
  const log10 = value.log10();
  const penalty = 0.75 * Effects.product(DilationUpgrade.dilationPenalty);
  return Decimal.pow10(Math.sign(log10) * Math.pow(Math.abs(log10), penalty));
}
// Effect: log10(mult) → log10(mult)^0.75  (dramatically reduces large multipliers)
```

**Tachyon Particles:** Gained at the end of a dilated eternity run:
```javascript
// TP = (log10(antimatter) / 400)^1.5 * tachyonGainMultiplier
```

**Dilated Time:** Produced passively from Tachyon Particles; used to buy Dilation
Upgrades.

**Tachyon Galaxies:** Free galaxies purchased with Dilated Time.

### 7.3 Time Studies

**File:** `src/core/time-studies/` (multiple files)

Time Studies form a tree of upgrades purchased with Time Theorems. The tree has:
- Normal studies (paths: Antimatter/Infinity/Time × Active/Passive/Idle × Light/Dark)
- Eternity Challenge studies (gating EC entry)
- Dilation studies (gating Dilation and Reality unlock)
- Triad studies (powerful late-game studies)

Time Theorems are purchased with AM, IP, or EP (each with escalating costs).

### 7.4 Glyphs

**Files:** `src/core/glyphs/`, `src/core/glyph-effects.js`

Glyphs are equippable items gained on Reality, each with random effects based on type:
- **Types:** Power, Infinity, Replication, Time, Dilation, Effarig, Reality, Cursed,
  Companion
- Each glyph has a level, rarity (strength), and 1-4 random effects
- Effects use bitmask storage for compact representation
- Effects are combined across equipped glyphs using type-specific combiners (add,
  multiply, etc.)
- Glyph sacrifice provides Alchemy resources (a late-game system)

### 7.5 Celestials

**Files:** `src/core/celestials/` (14 files, 3,572 lines)

Seven Celestials form the endgame content after the first Reality:

| Celestial | Key Mechanic | Resource |
|-----------|-------------|----------|
| **Teresa** | Special Reality run, IP storage | Perk Points |
| **Effarig** | Multi-stage Reality, glyph forge | Relic Shards |
| **Nameless (Enslaved)** | Time storage/release, stored time | Stored Time |
| **V** | Achievement-like hard goals | V-achievements |
| **Ra** | Pet system (memories/chunks), alchemy | Memories, Alchemy Resources |
| **Lai'tela** | Dark Matter Dimensions, entropy, continuum | Dark Matter, Dark Energy, Singularities |
| **Pelle** | "The Doomed" — final boss, disables most mechanics | Remnants, Reality Shards |

Each Celestial has a "Reality" (a special run with modified rules) and progressive
unlocks. Pelle is the final content, gating the game's ending.

---

## 8. Game Loop

**File:** `src/game.js` — `gameLoop()` (lines 422–646)

The game loop runs on a fixed interval (typically 33ms = ~30 FPS). Each tick:

```
1. Calculate realDiff (wall-clock time since last tick, capped at 1 day)
2. Hibernation check: if realDiff > 60s, switch to simulateTime() instead
3. Real-time mechanics (Ra memory, Dark Matter Dims, time storage)
4. Autobuyers tick
5. Invalidate cached multipliers (GameCache.*.invalidate())
6. Calculate game speed factor:
   - Black Hole speed × Time Glyph × Singularity Milestone
   - Nerfed by Effarig/Laitela/Pelle
   - Clamped to [1e-300, 1e300]
7. diff = realDiff × gameSpeedFactor  (game time)
8. Update all timers (real time played, infinity time, eternity time, reality time)
9. Pre-production IP generation
10. Passive prestige generation (infinities, eternities)
11. Production (in order):
    - TimeDimensions.tick(diff)
    - InfinityDimensions.tick(diff)
    - AntimatterDimensions.tick(diff)
12. Free tickspeed from Time Shards
13. Update prestige rates (IP/min, EP/min)
14. Replicanti growth
15. Dilated Time accumulation
16. Time Theorem generation
17. Black Hole phase updates
18. Auto-unlock perks
19. Celestial-specific ticks (Ra, Enslaved, Lai'tela, Pelle)
20. Automator update
21. UI update
```

### Game Speed

Game speed is a multiplicative factor applied to `diff`. Sources include:
- Black Holes (can provide massive speedups; two black holes with pulsing activation)
- Time Glyphs
- Singularity Milestones
- Enslaved time storage (absorbs game speed, can release it later)
- Celestial nerfs (Effarig, Lai'tela cap speed)
- Pelle upgrades

### Offline/Away Progress

When the game detects a long gap between ticks (>60s), it uses `simulateTime()` to
simulate the missed time in accelerated ticks, with configurable tick chunking.

---

## 9. Currency System

**File:** `src/core/currency.js` (479 lines)

All game currencies use a unified `Currency` abstraction:

```javascript
class Currency {
  get value()           // Current amount
  set value(value)      // Sets amount (with side effects like record tracking)
  add(amount)           // Increase
  subtract(amount)      // Decrease (clamped to 0)
  purchase(cost)        // Subtract cost if affordable; returns boolean
  reset()               // Reset to startingValue
  get startingValue()   // Value after prestige reset
}
```

Two backends:
- `NumberCurrency` — uses JS `number` (for counts like realities, perk points)
- `DecimalCurrency` — uses `Decimal` (for antimatter, IP, EP, etc.)

### Currency List

| Currency | Type | Setter Side Effects |
|----------|------|-------------------|
| `antimatter` | Decimal | Tracks max AM records, triggers autobuyer notifications |
| `infinityPoints` | Decimal | Tracks max IP per eternity/reality |
| `eternityPoints` | Decimal | Tracks max EP, best EP glyph set |
| `timeTheorems` | Decimal | Manages maxTheorem, respec on reset |
| `infinities` | Decimal | Count of infinities performed |
| `eternities` | Decimal | Count of eternities performed |
| `realities` | Number | Count of realities performed |
| `realityMachines` | Decimal | Hardcapped; tracks max RM |
| `tachyonParticles` | Decimal | Dilation resource |
| `dilatedTime` | Decimal | Tracks max DT |
| `infinityPower` | Decimal | From Infinity Dimensions |
| `timeShards` | Decimal | From Time Dimensions → free tickspeed |
| `darkMatter` | Decimal | Lai'tela resource |
| `darkEnergy` | Number | Lai'tela resource |
| `singularities` | Number | Lai'tela resource |
| `remnants` | Number | Pelle resource |
| `realityShards` | Decimal | Pelle resource |
| `replicanti` | Decimal | Self-replicating resource |
| `perkPoints` | Number | Reality perk currency |
| `relicShards` | Number | Effarig resource |
| `imaginaryMachines` | Number | Hardcapped endgame currency |

---

## 10. Data & Configuration Layer

**File:** `src/core/secret-formula/` (28,637 lines across ~50 files)

All game data is declarative configuration consumed by runtime mechanic classes.

### Game Database

`game-database.js` assembles the master `GameDatabase` object:

```javascript
GameDatabase = {
  achievements,          // Normal + secret achievements
  celestials,            // Per-celestial configs, alchemy, navigation, quotes
  challenges,            // Normal, Infinity, Eternity challenge definitions
  eternity,              // Milestones, time studies, dilation studies
  infinity,              // Upgrades, break-infinity upgrades
  reality,               // Glyphs, perks, automator, upgrades, imaginary upgrades
  tabs,                  // UI tab definitions
  // ...
}
```

### Configuration Pattern

Each mechanic is defined as a config object:

```javascript
// Example: a Normal Time Study
{
  id: 91,
  cost: 4,
  requirement: [73],
  reqType: TS_REQUIREMENT_TYPE.AT_LEAST_ONE,
  description: "Antimatter Dimension multiplier based on time spent in this Eternity",
  effect: () => Decimal.pow10(Math.min(Time.thisEternity.totalMinutes, 20) * 15),
  cap: DC.E300,
  formatEffect: value => formatX(value, 2, 1)
}
```

The runtime class `GameMechanicState` wraps this, providing:
- `effectValue` (with capping)
- `isEffectActive` / `canBeApplied`
- `isUnlocked` / `isBought`
- Event registration

### Accessor Pattern

Collections are turned into indexed accessor functions:

```javascript
// Creates: TimeStudy(91), TimeStudy(101), etc.
static createAccessor(gameData) {
  const index = mapGameData(gameData, config => new this(config));
  const accessor = id => index[id];
  accessor.index = index;
  return accessor;
}
```

---

## 11. Player State

**File:** `src/core/player.js` (1,089 lines)

The entire game state is a single `window.player` object, serialized for save/load.

### Key Sections

```javascript
player = {
  antimatter: DC.E1,                    // Starting currency
  dimensions: {
    antimatter: [/* 8 tiers: {bought, costBumps, amount} */],
    infinity:   [/* 8 tiers: {isUnlocked, bought, amount, cost, baseAmount} */],
    time:       [/* 8 tiers: {cost, amount, bought} */],
  },
  sacrificed: DC.D0,
  challenge: {
    normal:   { current, bestTimes, completedBits },
    infinity: { current, bestTimes, completedBits },
    eternity: { current, unlocked, requirementBits },
  },
  auto: { /* ~20 autobuyer configurations with intervals, modes, flags */ },
  records: {
    thisInfinity:  { time, realTime, maxAM, bestIPmin, ... },
    thisEternity:  { time, realTime, maxIP, bestEPmin, ... },
    thisReality:   { time, realTime, maxEP, maxDT, ... },
    bestInfinity:  { time, bestIPminEternity, ... },
    bestEternity:  { time, bestEPminReality, ... },
    bestReality:   { time, RM, bestEP, glyphSets, ... },
    recentInfinities:  [/* last 10 */],
    recentEternities:  [/* last 10 */],
    recentRealities:   [/* last 10 */],
    totalTimePlayed, realTimePlayed, totalAntimatter, ...
  },
  timestudy: { theorem, studies: [], maxTheorem, presets, preferredPaths },
  replicanti: { amount, unl, chance, interval, galaxies, ... },
  dilation: { active, tachyonParticles, dilatedTime, totalTachyonGalaxies, upgrades, ... },
  reality: {
    realityMachines, glyphs: { active, inventory, ... },
    perks, upgradeBits, imaginaryUpgradeBits, automator, ...
  },
  celestials: {
    teresa:   { ... },
    effarig:  { relicShards, ... },
    enslaved: { stored, isStoringReal, ... },
    v:        { ... },
    ra:       { pets, alchemy, ... },
    laitela:  { darkMatter, darkEnergy, singularities, entropy, ... },
    pelle:    { doomed, remnants, realityShards, rifts, ... },
  },
  options: { /* UI/gameplay preferences */ },
}
```

### Bitfield Patterns

Many boolean collections use bitfields for compact storage:
- `achievementBits: Array.repeat(0, 17)` — 17 × 32-bit integers for ~180 achievements
- `challenge.normal.completedBits` — bitmask of completed challenges
- `infinity.upgradeBits` — bitmask of purchased upgrades
- `reality.upgradeBits` — bitmask of purchased reality upgrades

---

## 12. Challenges

### Normal Challenges (12)

- Modify basic game rules (e.g., "only 6 dimensions", "all dims cost the same")
- Completions tracked via bitfield
- Goal: reach Infinity under the restriction

### Infinity Challenges (8)

- Harder modifications, unlocked at antimatter thresholds
- Completion rewards provide permanent multipliers
- Goal: reach Infinity under stricter restrictions

### Eternity Challenges (12)

- Multi-completion challenges (up to 5 completions each)
- Goals scale with each completion (`goal * goalIncrease^completions`)
- Some have secondary requirements (e.g., "no Time Studies", "limited galaxies")
- Pelle-specific alternative goals for some ECs
- State uses `player.eternityChalls.eterc{N}` for completion counts

---

## 13. Automation (Autobuyers)

**File:** `src/core/autobuyers/` (multiple files)

Autobuyers automatically perform game actions. They share a common framework:

- **State:** Stored in `player.auto.*` with interval, mode, activation flags
- **Tick:** `Autobuyers.tick()` runs all active autobuyers each game loop tick
- **Upgrade path:** Early-game autobuyers have upgradeable intervals; late-game perks
  make them instant
- **Types:** Dimension buyers (×8), tickspeed, dimension boost, galaxy, big crunch,
  eternity, reality, replicanti galaxy, time study, sacrifice, and more

---

## 14. Key Architectural Patterns

### 1. Effect Composition

The most pervasive pattern — multipliers are built by chaining effects from many sources:

```javascript
multiplier = multiplier.timesEffectsOf(
  Achievement(48), Achievement(56), TimeStudy(91), TimeStudy(101),
  InfinityChallenge(3), InfinityChallenge(3).reward,
  EternityChallenge(10), AlchemyResource.dimensionality,
  PelleUpgrade.antimatterDimensionMult
);
multiplier = multiplier.powEffectsOf(
  AlchemyResource.power, Achievement(183), PelleRifts.paradox
);
```

**For Rust:** This pattern maps naturally to a `Vec<Box<dyn Effect>>` or a trait-based
approach, but the sheer number of conditional effects (~50+ per dimension) means the
multiplier pipeline will need careful design.

### 2. Tiered Accessor Pattern

Dimensions and similar tiered systems use `createAccessor()` to produce `Dimension(tier)`
callable functions with an `.all` property for iteration:

```javascript
const AntimatterDimension = AntimatterDimensionState.createAccessor();
AntimatterDimension(1).amount  // Access tier 1
AntimatterDimensions.all       // Iterate all 8
```

### 3. Lazy/Cached Computation

Expensive calculations are wrapped in `GameCache`:

```javascript
GameCache.antimatterDimensionCommonMultiplier = new Lazy(() => antimatterDimensionCommonMultiplier());
// Invalidated each tick:
GameCache.antimatterDimensionCommonMultiplier.invalidate();
```

### 4. Prestige Reset Chains

Each prestige layer calls the reset function of the layer below:

```
Reality reset → Eternity reset → Infinity reset → softReset (dims + tickspeed)
```

What is preserved at each level depends on milestones, upgrades, and which celestial is
running.

### 5. Global Mutable State

The entire game state lives in `window.player`. There is no immutability, no state
management library, and no state transitions — just direct mutation. This is the biggest
architectural concern for a Rust rewrite.

---

## 15. Considerations for a Rust Rewrite

### What to Rewrite

The entire `src/core/` directory (62k lines) contains the simulation logic. The Vue
`src/components/` (52k lines) is presentation-only and would not be rewritten — instead,
a Rust backend would either:
- Compile to WASM and expose an API to a web frontend
- Run natively with a TUI or egui frontend
- Run headlessly for numerical analysis

### Key Challenges

1. **Decimal Arithmetic:** `break_infinity.js` needs a Rust equivalent. Options:
   - Port `break_infinity.js` directly (mantissa + exponent representation)
   - Use an existing crate like `num-bigfloat` or write a custom `Decimal` type
   - For numerical analysis, consider whether `f64` suffices for logarithmic
     representations

2. **Effect Composition:** The ~200 effect sources that compose
   multiplicatively/additively need a systematic representation. A Rust approach could
   use:
   - An enum of all effect sources
   - Trait objects for the `Effect` interface
   - A compile-time registry of effects per mechanic

3. **Global Mutable State:** JavaScript's `window.player` won't work in Rust. Options:
   - A single `GameState` struct passed by `&mut` reference
   - An ECS-like architecture for more modularity
   - The `player` struct is ~1,100 lines of nested objects — Rust structs would be
     verbose
     but type-safe

4. **Dynamic Dispatch:** JavaScript uses prototype chains and duck typing extensively.
   The `GameMechanicState → PurchasableMechanicState → ...` hierarchy would map to Rust
   traits, but the config-driven design (where `effect` is a closure) needs adaptation.

5. **Cost Scaling Math:** The `ExponentialCostScaling`, `LinearCostScaling`, and
   `bulkBuyBinarySearch` utilities are pure math and straightforward to port.

6. **Challenge System:** Challenges modify game rules via scattered `if
   (Challenge.isRunning)` checks throughout the codebase. A Rust rewrite could centralize
   these as rule modifiers on the game state, making the system more maintainable.

7. **Caching:** Rust's borrow checker makes lazy caching patterns more involved than
   JavaScript's `Lazy`. Consider `OnceCell`, `RefCell`, or a tick-based invalidation
   system.

### Suggested Approach for Numerical Analysis

For analyzing the game's numerical behavior without a full rewrite:

1. **Start with the production pipeline:** Implement just the dimension → currency
   production chain with static multipliers.

2. **Add prestige cycles:** Model the Infinity/Eternity/Reality reset loop with
   simplified gain formulas.

3. **Layer in complexity gradually:** Add challenges, glyphs, celestials as needed for
   the specific analysis.

4. **Use logarithmic representation internally:** Since most interesting behavior is
   exponential, working in log-space (storing `log10(value)` instead of `value`) can
   simplify arithmetic and avoid the need for a full Decimal library for many analyses.

### Module Dependency Map

For planning a phased Rust implementation:

```
Phase 1 (Core simulation):
  Decimal type → Currency → DimensionState → AntimatterDimensions
  → Tickspeed → DimBoost → Galaxy → Sacrifice
  → GameLoop (simplified)

Phase 2 (Prestige):
  Infinity (BigCrunch) → InfinityDimensions → InfinityPower
  → Eternity → TimeDimensions → TimeShards → FreeTickspeed
  → Challenges (Normal, Infinity)

Phase 3 (Mid-game):
  Replicanti → ReplicantiGalaxies
  → TimeDilation → TachyonParticles → DilatedTime
  → TimeStudies → TimeTheorems
  → EternityChallenges

Phase 4 (Endgame):
  Reality → RealityMachines → Glyphs → GlyphEffects
  → Perks → Automator
  → Celestials (Teresa → Effarig → Enslaved → V → Ra → Lai'tela → Pelle)
```

---

*Document generated by codebase analysis. Last updated: 2026-06-11.*
