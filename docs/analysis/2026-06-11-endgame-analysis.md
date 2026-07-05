---
status: Reference
---

# Antimatter Dimensions Endgame — Codebase Analysis

## Table of Contents

1. [Overview & Comparison with Base Game](#1-overview--comparison-with-base-game)
2. [Size Comparison](#2-size-comparison)
3. [New Systems Added](#3-new-systems-added)
   - 3.1 [Endgame Prestige Layer](#31-endgame-prestige-layer)
   - 3.2 [Celestial Points & Doomed Particles](#32-celestial-points--doomed-particles)
   - 3.3 [Celestial Dimensions](#33-celestial-dimensions)
   - 3.4 [Celestial Matter](#34-celestial-matter)
   - 3.5 [Endgame Masteries (Skill Tree)](#35-endgame-masteries-skill-tree)
   - 3.6 [Endgame Skills](#36-endgame-skills)
   - 3.7 [Endgame Upgrades](#37-endgame-upgrades)
   - 3.8 [Endgame Milestones](#38-endgame-milestones)
   - 3.9 [Break Eternity Upgrades](#39-break-eternity-upgrades)
   - 3.10 [Expansion Packs](#310-expansion-packs)
   - 3.11 [Galactic Power](#311-galactic-power)
   - 3.12 [Ethereal](#312-ethereal)
   - 3.13 [Alpha (New Celestial)](#313-alpha-new-celestial)
   - 3.14 [Pelle Destruction Upgrades](#314-pelle-destruction-upgrades)
4. [Modifications to Existing Systems](#4-modifications-to-existing-systems)
5. [New Currencies](#5-new-currencies)
6. [New Player State](#6-new-player-state)
7. [Architecture Assessment](#7-architecture-assessment)
8. [Porting Considerations for Rust](#8-porting-considerations-for-rust)

---

## 1. Overview & Comparison with Base Game

The "endgame" repository extends Antimatter Dimensions with substantial post-completion
content. After the base game's ending (defeating Pelle and reaching the credits), this
mod adds a new prestige layer called "Endgame" that resets the entire game and introduces
a cascading system of new mechanics, currencies, and progression paths.

The fundamental design philosophy is the same as the base game — cascading prestige
resets with increasing power — but layered on top of the existing Reality/Celestial
system. The new content effectively adds a **5th prestige layer** above Reality.

---

## 2. Size Comparison

| Metric | Base Game | Endgame Mod | Difference |
|--------|----------:|------------:|-----------:|
| JS files (src/) | 270 | 303 | +33 |
| Vue components | 409 | 463 | +54 |
| JS lines (src/) | 66,797 | 78,418 | +11,621 (+17%) |
| Vue lines (src/) | 52,333 | 59,453 | +7,120 (+14%) |
| Core JS files | 238 | 270 | +32 |
| Core JS lines | 62,372 | 73,477 | **+11,105 (+18%)** |
| Secret formula lines | 28,637 | ~29,700 | +1,063 |

**Summary:** The endgame mod adds approximately **11,100 lines of game logic** (18%
increase) and **54 new Vue components** for the UI. This is a significant but manageable
expansion — roughly equivalent in scope to adding one and a half celestials' worth of
content.

### Key File Changes (Lines Added)

| File | Base | Endgame | +Lines | Notes |
|------|-----:|--------:|-------:|-------|
| `src/game.js` | 1,117 | 1,473 | +356 | New tick logic, prestige formulas |
| `src/core/player.js` | 1,089 | 1,288 | +199 | New state fields |
| `src/core/math.js` | 1,489 | 1,649 | +160 | New cost scaling functions |
| `src/core/currency.js` | 479 | 625 | +146 | 8 new currencies |
| `src/core/replicanti.js` | 576 | 701 | +125 | Extended mechanics |
| `src/core/constants.js` | 507 | 582 | +75 | New Decimal constants |
| `src/core/dilation.js` | 273 | 348 | +75 | Extended dilation |
| `src/core/galaxy.js` | 178 | 234 | +56 | Galactic power integration |
| `src/core/eternity.js` | 357 | 407 | +50 | Break Eternity integration |
| Pelle directory | 1,013 | 1,588 | +575 | New Pelle mechanics + destruction |

### Entirely New Files (Core Logic)

| File | Lines | Purpose |
|------|------:|---------|
| `endgame.js` | 746 | Endgame prestige reset logic |
| `endgame-masteries/` (6 files) | 738 | Skill tree system |
| `endgame-upgrades.js` | 128 | Endgame upgrades |
| `endgame-skills.js` | 147 | Skill currency purchasing |
| `break-eternity-upgrades.js` | 61 | New upgrade layer |
| `expansion-packs.js` | 56 | Celestial expansions |
| `galactic-power.js` | 57 | Galaxy-based resource |
| `ethereal.js` | 29 | Late endgame resource |
| `celestials/alpha.js` | 77 | New celestial |
| `dimensions/celestial-dimension.js` | 284 | 4th dimension type |
| `pelle-destruction-upgrades.js` | 205 | New Pelle upgrade type |
| `secret-formula/endgame/` (8 files) | 1,057 | Endgame data definitions |
| New autobuyers (6 files) | 321 | Automation for new systems |
| **Total new files** | **~3,900** | |

---

## 3. New Systems Added

### 3.1 Endgame Prestige Layer

**File:** `src/core/endgame.js` (746 lines)

The Endgame is a new prestige layer above Reality. It triggers when antimatter during
Pelle (the "doomed" state) reaches `1e9e15`:

```javascript
export function isEndgameAvailable() {
  return player.celestials.pelle.records.totalEndgameAntimatter.add(1).log10().gte(9e15);
}
```

**Reset behavior:** Endgame resets *everything* — all prestige layers, all celestial
progress, achievements (conditionally), glyphs, Reality upgrades, Imaginary upgrades, and
more. This is by far the most comprehensive reset in the game (~560 lines of state
clearing).

**What persists across Endgame:**
- Endgame count (`player.endgames`)
- Celestial Points & Doomed Particles
- Endgame Masteries (skill tree)
- Endgame Upgrades
- Expansion Packs (purchased)
- Galactic Power, Ethereal Power
- Conditional carryovers based on Endgame Masteries/Upgrades (e.g., achievements, perks,
  reality upgrades, Ra levels)

### 3.2 Celestial Points & Doomed Particles

The two primary currencies earned on Endgame:

```javascript
// Celestial Points formula:
cp = log10(totalEndgameAntimatter + 1) / 9e15
// (with Achievement 197 bonus multiplier)

// Doomed Particles formula:
dp = log10(totalEndgameAntimatter + 1) / 9e15
// (capped at 1e100)
```

Both are used to purchase Endgame Skills, which fund the Mastery Tree.

### 3.3 Celestial Dimensions

**File:** `src/core/dimensions/celestial-dimension.js` (284 lines)

A **4th type of dimension** (alongside Antimatter, Infinity, and Time Dimensions):

- 8 tiers following the same production chain pattern (tier N produces tier N-1)
- Purchased with Celestial Points
- Produce **Celestial Matter** (tier 1 output)
- Unlock requirements: progressively higher CP thresholds (1, 10, 100, 1e4, 1e10, ...)
- Cost scaling: simple exponential (`cost * costMultiplier` per purchase)
- Per-purchase multiplier: 2x base (boosted by singularity milestones)
- Common multiplier includes Endgame Upgrade effects and Ethereal sector boost
- Production has a softcap with configurable threshold and exponent
- Hardcapped at 2^1024 purchases per dimension

**Architecture:** Follows the same `DimensionState` base class pattern as other
dimensions. This is a clean extension of the existing framework — no new paradigms
needed.

### 3.4 Celestial Matter

Celestial Matter is produced by Celestial Dimension 1, then processed through a
softcap/instability curve:

```javascript
// From game loop:
const uncapped = Decimal.min(unnerfedCelestialMatter, CelestialDimensions.SOFTCAP);
const instability = Decimal.pow(
  Decimal.max(unnerfedCelestialMatter / SOFTCAP, 1),
  1 / CelestialDimensions.softcapPow
);
celestialMatter = Decimal.min(uncapped * instability, Number.MAX_VALUE);
```

Celestial Matter provides a **game speed multiplier** (when active), making it a very
powerful resource:

```javascript
if (celestialMatter > 0 && celestialMatterMultiplier.isActive) {
  factor = factor.times(Decimal.pow(celestialMatter, conversionExponent));
}
```

### 3.5 Endgame Masteries (Skill Tree)

**Directory:** `src/core/endgame-masteries/` (738 lines)

A tree-structured upgrade system purchased with Endgame Skills:

- **~50+ masteries** defined in `secret-formula/endgame/endgame-masteries.js`
- Tree has **paths**: Compression paths (AD/ID/TD/CD) and Currency paths (IP/EP/RM/iM)
- Limited path choices per tree (similar to Time Studies' exclusive paths)
- Supports import/export strings (like Time Studies)
- Respec available on Endgame
- Permanent masteries that persist even through respec

**Requirement types:**
```javascript
EM_REQUIREMENT_TYPE.AT_LEAST_ONE   // Any prerequisite
EM_REQUIREMENT_TYPE.ALL            // All prerequisites
EM_REQUIREMENT_TYPE.COMPRESSION_PATH  // Path-limited
EM_REQUIREMENT_TYPE.CURRENCY_PATH     // Path-limited
```

**Examples of mastery effects:**
- Start with 100 Realities
- Start with 1,000,000 Reality Machines
- Start with all Reality Upgrades unlocked
- Galaxies are 10% stronger
- Keep achievements across Endgame
- Start with glyphs

### 3.6 Endgame Skills

**File:** `src/core/endgame-skills.js` (147 lines)

The currency used to buy Endgame Masteries. Purchased from three sources:
- Galaxy Generator Galaxies (GG) — cost: 1e10 * 1e2^amount
- Celestial Points (CP) — cost: 1 * 10^amount
- Doomed Particles (DP) — cost: 1 * 10^amount

Bulk-buying supported. Respec returns all skills.

### 3.7 Endgame Upgrades

**File:** `src/core/endgame-upgrades.js` (128 lines)

Two types:
1. **Rebuyable** (IDs 1-5): Purchased with Celestial Points
   - Delay various softcaps (Infinity, Time, Celestial dimensions)
   - Increase Dark Matter hardcap
2. **One-time** (IDs 6+): Requirement-gated + CP cost
   - Significant milestone upgrades (e.g., "Start with 1e7 Perk Points")
   - Conditional checks that can "fail" if requirements are missed

### 3.8 Endgame Milestones

**File:** `src/core/secret-formula/endgame/endgame-milestones.js` (89 lines)

Passive bonuses unlocked at Endgame counts:
- 1: Rift Fill speed boost
- 2: Remnant-based Galaxy strength
- 5: Galaxy Generator animation speed
- 10: New Galaxy Generator upgrade
- 15: Improved Remnant formula
- 25: Remove 1e300 game speed hardcap
- 50: Start with first 6 Celestials unlocked
- 100: Reality Shards multiply Dilated Time
- 250: Endgames boost Galaxy production
- 1,000: Endgame autobuyer
- 10,000: Antimatter production power boost
- 1,000,000: Galaxy Generator instability reduction

### 3.9 Break Eternity Upgrades

**File:** `src/core/break-eternity-upgrades.js` (61 lines) **Data:**
`secret-formula/endgame/break-eternity-upgrades.js` (175 lines)

A new upgrade layer (analogous to "Break Infinity" for the Eternity mechanic):
- 10 rebuyable upgrades (max 10 each), purchased with Antimatter:
  - Square AD/ID/TD multipliers (^2 each purchase)
  - Square-root Replicanti interval
  - Square Tachyon Particle gain
  - Delay galaxy scaling (+10,000 galaxies per purchase)
  - Double Infinity Power conversion rate
  - Raise EP Multiplier softcap threshold
  - Double Replicanti Galaxy scaling start
  - Double Dilated Time per-purchase multiplier
- 5 one-time upgrades with extreme costs (1e30 to 1e100):
  - Uncap 2x IP Multiplier
  - Uncap TG Threshold upgrade
  - Double Tesseracts
  - Uncap Glyph Sacrifice
  - Add 3 more Glyph Slots

### 3.10 Expansion Packs

**File:** `src/core/expansion-packs.js` (56 lines) **Data:**
`secret-formula/endgame/expansion-packs.js` (84 lines)

Seven Expansion Packs (one per Celestial), unlocked with extreme antimatter amounts:

| Pack | Cost | Key Effects |
|------|------|-------------|
| Teresa | 1e(1e30) | Uncap Teresa's Canister, Charged Perk Upgrades |
| Effarig | 1e(1e50) | Improved Alchemy, kept Alchemy on Endgame, auto-Effarig |
| Nameless | 1e(1e70) | Better time storage, Tesseracts boost Endgames |
| V | 1e(1e90) | V auto-unlocks, double Space Theorems |
| Ra | 1e(1e110) | Ra kept on Endgame, new celestial effects, uncapped levels |
| Lai'tela | 1e(1e130) | Improved DM/DE mechanics, Hadronize unlock |
| Pelle | 1e(1e150) | Galaxy Generator improvements, dimension power boost |

These use extreme costs only achievable after many Endgame cycles.

### 3.11 Galactic Power

**File:** `src/core/galactic-power.js` (57 lines)

A passive resource generated from galaxies:

```javascript
// Gain formula:
galaxyFactor = max(allGalaxies / 100000, 1)
celestialMatterFactor = max(log10(celestialMatter+1)/10)^4, 1)
imaginaryFactor = max(log10(imaginaryMachines+1)^2.5, 1)
base = galaxyFactor * celestialMatterFactor * imaginaryFactor / 1e7
// Then raised to a multi-stage exponent based on galaxy count thresholds
```

**8 rewards** unlocked at Galactic Power thresholds (0, 1e10, 1e20, 1e50, 1e100, 1e150,
1e200, MAX_VALUE):
- Galaxy strength boost
- Remote Galaxy scaling delay
- Remote Galaxy weakening
- Galaxy Generator instability delay
- Replicanti Galaxy multiplier
- Tachyon Galaxy threshold reduction
- Galaxy Generator instability reduction
- **Ethereal unlock** (at Number.MAX_VALUE GP)

### 3.12 Ethereal

**File:** `src/core/ethereal.js` (29 lines)

The deepest endgame resource, unlocked at max Galactic Power:

```javascript
// Ethereal Power gain per second:
cpFactor = (log10(celestialPoints+1) / 100)^10
singFactor = (log10(singularities+1) / 20000)^3
rmFactor = (log10(log10(realityMachines+1)+1) / 5)^75
gpFactor = (log10(max(galacticPower, MAX_VALUE)) / 308.25)^5
return cpFactor * singFactor * rmFactor * gpFactor / 1000
```

**Cosmic Sectors:** Ethereal Power advances through sectors with increasingly high
thresholds (`sector^sector`). Each sector provides a **sector boost** of
`2^((sector-1)²)` that multiplies Celestial Dimension production.

### 3.13 Alpha (New Celestial)

**File:** `src/core/celestials/alpha.js` (77 lines)

A new 8th Celestial unlocked via Imaginary Upgrade 30:
- Has a **staged progression** (28 stages) — each stage is a game goal to achieve within
  Alpha's Reality (a heavily nerfed run)
- Stages progress through the entire game: from "Reach 4th Dimension Boost" to "Reach
  Reality"
- Provides a `celestialMatterConversionNerf` that scales with stage progression
- Currently partially implemented (some code is commented out)

### 3.14 Pelle Destruction Upgrades

**File:** `src/core/celestials/pelle/pelle-destruction-upgrades.js` (205 lines)

A new upgrade category within Pelle that "destroys" (sacrifices) previously earned
resources for bonuses:
- Achievable by sacrificing achievements, upgrades, reality upgrades, imaginary upgrades,
  celestial completions, perks, alchemy resources, and Pelle strikes

---

## 4. Modifications to Existing Systems

The endgame mod makes pervasive changes to existing systems by adding `EndgameMastery`,
`EndgameUpgrade`, `ExpansionPack`, and `BreakEternityUpgrade` checks throughout:

### Game Loop (+356 lines)
- New production ticks: `CelestialDimensions.tick(realDiff)`
- Celestial Matter calculation with softcap
- Galactic Power and Ethereal Power accumulation
- Endgame milestone tracking
- Endgame-specific game speed modifications (Celestial Matter as game speed multiplier)
- Peak game speed tracking per endgame
- Expansion Pack automated effects (auto-pour Teresa, auto-Effarig, auto-V achievements)
- Endgame Mastery passive effects (perk point generation)
- Break Eternity check for new prestige tier
- Extended timer tracking (`thisEndgame.time`, `thisEndgame.realTime`)

### Replicanti (+125 lines)
- Modified growth formulas with endgame multipliers
- New replicanti galaxy interactions

### Dilation (+75 lines)
- New dilation upgrades (IDs 11-15) for Pelle content
- Modified tachyon particle formulas

### Galaxy System (+56 lines)
- Galactic Power integration into galaxy strength
- New scaling delays from Break Eternity and Endgame effects
- Modified instability thresholds

### Dimension Boost (+42 lines)
- Endgame Mastery effects on boost power
- New boost sources

### Pelle (+575 lines)
- New destruction upgrade system
- Extended Galaxy Generator mechanics
- Modified game end conditions for Endgame trigger
- New Pelle strikes and rift modifications

### Many Decimal → Decimal conversions
The endgame mod converts several values that were `Number` type in the base game to
`Decimal` type (e.g., `player.galaxies`, `player.dimensionBoosts`, `singularities`,
`darkEnergy`), likely because they can exceed `Number.MAX_VALUE` in the endgame.

---

## 5. New Currencies

| Currency | Type | Source | Use |
|----------|------|--------|-----|
| `endgames` | Number | Endgame prestige count | Milestone thresholds |
| `celestialPoints` | Decimal | Endgame prestige | Celestial Dims, Endgame Upgrades, Skills |
| `doomedParticles` | Decimal | Endgame prestige (capped 1e100) | Endgame Skills |
| `celestialMatter` | Decimal | Celestial Dim 1 (softcapped) | Game speed multiplier |
| `unnerfedCelestialMatter` | Decimal | Raw CD1 output | Intermediate calculation |
| `endgameSkills` | Decimal | Purchased with GG/CP/DP | Endgame Mastery purchases |
| `galacticPower` | Decimal | Passive from galaxies | Reward thresholds |
| `etherealPower` | Decimal | Passive from multiple sources | Sector advancement |

---

## 6. New Player State

The endgame mod adds approximately **200 lines** to `player.js`:

```javascript
player = {
  // ... existing state ...
  breakEternityUpgrades: new Set(),
  breakEternityRebuyables: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  dimensions: {
    // ... existing ...
    celestial: [/* 8 tiers: {isUnlocked, bought, amount, cost, baseAmount} */],
  },
  celestials: {
    // ... existing ...
    alpha: { unlockBits: 0, run: false, quoteBits: 0, stage: 0 },
  },
  endgames: 0,
  endgame: {
    celestialPoints: DC.D0,
    doomedParticles: DC.D0,
    celestialMatter: DC.D0,
    unnerfedCelestialMatter: DC.D0,
    celestialMatterMultiplier: { isActive: true },
    pelleDestruction: { /* Sets for various sacrifice types */ },
    respec: false,
    galacticPower: DC.D0,
    rebuyables: { 1: 0, 2: 0, 3: 0, 4: 0, 5: 0 },
    upgradeBits: 0, upgReqs: 0, reqLock: 0,
    partEndgamed: 0,
    expansionPacks: { areUnlocked: false, boughtPacks: new Set() },
    ethereal: { power: DC.D0, sector: 1 },
  },
  endgameMasteries: {
    skills: DC.D0, maxSkills: DC.D0,
    ggBought: 0, cpBought: 0, dpBought: 0,
    masteries: [],
    shopMinimized: false,
    preferredPaths: [[], []],
    presets: [/* 6 slots */],
    permanentMasteries: [],
  },
  records: {
    // ... adds thisEndgame, bestEndgame, recentEndgames, permanent ...
  },
};
```

---

## 7. Architecture Assessment

### Positive Aspects

1. **Follows existing patterns perfectly:** Celestial Dimensions use `DimensionState`,
   Endgame Upgrades use `BitPurchasableMechanicState`, Expansion Packs use
   `SetPurchasableMechanicState`. No new architectural patterns introduced.

2. **Clean separation:** All new mechanics are in their own files with clear boundaries.

3. **Data-driven:** All endgame configs live in `secret-formula/endgame/` following the
   established pattern.

4. **Incremental complexity:** Each system builds on the previous ones (Endgame → CP/DP →
   Skills → Masteries → Expansion Packs → Galactic Power → Ethereal).

### Concerns

1. **Massive reset function:** `Endgame.resetStuff()` is ~560 lines of direct state
   mutation with deeply nested conditionals based on which masteries/upgrades/packs are
   purchased. This is the messiest part of the codebase.

2. **Number → Decimal conversions:** Several values changed from JS `number` to `Decimal`
   (galaxies, dimension boosts, singularities, dark energy) which means arithmetic
   operations on these values changed throughout the codebase.

3. **Scattered conditionals:** Endgame effects are checked via
   `EndgameMastery(N).isBought`, `EndgameUpgrade(N).isBought`, and
   `ExpansionPack.X.isBought` scattered across existing files, making it harder to trace
   the full effect of any single upgrade.

4. **Partially implemented features:** Alpha celestial has commented-out code, suggesting
   the mod is still in development.

---

## 8. Porting Considerations for Rust

### Additional Challenges Beyond Base Game

1. **Even Larger Numbers:** The endgame pushes numbers to `1e(1e150)` for Expansion Pack
   costs and similar extreme values. The `break_infinity.js` Decimal might not suffice; a
   `break_eternity.js`-style representation (tetration-capable) may be needed. In Rust,
   this means the number type needs to handle towers of exponents.

2. **4th Dimension Type:** Celestial Dimensions are straightforward to port (same base
   class) but they add another production chain that runs every tick.

3. **Complex Reset Logic:** The Endgame reset (560 lines) has ~50+ conditional branches
   based on purchased upgrades. In Rust, this could be modeled as a `ResetPolicy` struct
   that encodes what to preserve, making the logic more maintainable.

4. **Mastery Tree with Constraints:** The Endgame Mastery tree has path limitations (only
   N compression paths, only M currency paths). Import/export/validation logic exists.
   This needs a proper graph data structure in Rust.

5. **Number Type Widening:** Several fields changed from `number` to `Decimal`:
   - `player.galaxies`: was `number`, now `Decimal` (can exceed 2^53)
   - `player.dimensionBoosts`: was `number`, now `Decimal`
   - `player.celestials.laitela.singularities`: was `number`, now `Decimal`
   - `player.celestials.laitela.darkEnergy`: was `number`, now `Decimal`

This means in Rust, these should all be the "big number" type from the start, or the type
system should allow seamless promotion.

6. **Game Speed as Decimal:** Game speed can now exceed `1e300` (milestone at 25 Endgames
   removes the cap), meaning game speed itself needs `Decimal` arithmetic in the endgame.

### No New Architectural Paradigms

The good news is that the endgame mod introduces **no fundamentally new patterns**. Every
new system uses the same framework:
- Config in `secret-formula/` → `GameMechanicState` subclass → accessor function
- Currencies follow the same `Currency` abstraction
- Dimensions follow the same `DimensionState` pattern
- Upgrades follow `BitPurchasableMechanicState` / `RebuyableMechanicState`

This means a Rust rewrite that handles the base game's patterns will naturally
accommodate the endgame content with just more instances of the same traits/structs.

### Revised Phase Plan (Including Endgame)

```
Phase 1-4: (Same as base game analysis)

Phase 5 (Endgame Core):
  Endgame prestige layer → Celestial Points/Doomed Particles
  → CelestialDimensions → CelestialMatter → Game Speed integration
  → Endgame Skills → Endgame Masteries (tree with constraints)
  → Endgame Upgrades → Endgame Milestones

Phase 6 (Endgame Expansion):
  Break Eternity Upgrades
  → Expansion Packs (conditional carryover modifiers)
  → Galactic Power → Galactic Power Rewards
  → Ethereal → Cosmic Sectors
  → Alpha celestial
  → Pelle Destruction Upgrades
```

### Estimated Additional Porting Effort

| Category | Lines to Port | Complexity |
|----------|-------------:|:----------:|
| New systems (clean files) | ~3,900 | Low-Medium |
| Modifications to existing systems | ~1,500 | Medium-High |
| Config/data definitions | ~1,100 | Low |
| Scattered conditionals in existing code | ~1,500 | Medium |
| Number type widening changes | ~500 | Medium |
| **Total** | **~8,500** | |

The endgame mod represents approximately **14% additional porting effort** beyond the
base game, primarily due to:
- The massive reset function (needs careful translation)
- Number type widening (requires early design decisions about numeric types)
- Integration points with existing systems (many small changes across many files)

The new standalone systems (Celestial Dimensions, Galactic Power, Ethereal, Mastery Tree)
are relatively straightforward since they follow established patterns.

---

*Document generated by codebase analysis. Last updated: 2026-06-11.*
