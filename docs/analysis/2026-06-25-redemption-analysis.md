# Antimatter Dimensions Redemption — Codebase Analysis

## Table of Contents

1. [Overview & Comparison with Base Game](#1-overview--comparison-with-base-game)
2. [Size Comparison](#2-size-comparison)
3. [New Systems Added](#3-new-systems-added)
   - 3.1 [Mending (New Prestige Layer)](#31-mending-new-prestige-layer)
   - 3.2 [Mending Upgrades](#32-mending-upgrades)
   - 3.3 [Mending Milestones](#33-mending-milestones)
   - 3.4 [Corruption System](#34-corruption-system)
   - 3.5 [Corruption Upgrades](#35-corruption-upgrades)
   - 3.6 [Warp Upgrades](#36-warp-upgrades)
   - 3.7 [Multiversal Dimensions](#37-multiversal-dimensions)
   - 3.8 [Kohler (New Celestial)](#38-kohler-new-celestial)
   - 3.9 [Kohler Upgrades & Milestones](#39-kohler-upgrades--milestones)
   - 3.10 [Kohler Infinity Upgrades](#310-kohler-infinity-upgrades)
   - 3.11 [Matter Dimensions](#311-matter-dimensions)
   - 3.12 [Matter Upgrades](#312-matter-upgrades)
   - 3.13 [Expo Black Hole](#313-expo-black-hole)
   - 3.14 [Destroyer (New Celestial)](#314-destroyer-new-celestial)
   - 3.15 [Transcendents Framework](#315-transcendents-framework)
4. [Modifications to Existing Systems](#4-modifications-to-existing-systems)
5. [New Currencies](#5-new-currencies)
6. [New Player State](#6-new-player-state)
7. [Architecture Assessment](#7-architecture-assessment)
8. [Porting Considerations for Rust](#8-porting-considerations-for-rust)

---

## 1. Overview & Comparison with Base Game

The "Redemption" mod extends Antimatter Dimensions with substantial
post-completion content. After the base game's ending (defeating Pelle),
this mod adds a new prestige layer called **Mending** that resets the
entire game and introduces new mechanics: corrupted runs, Warp upgrades,
Multiversal Dimensions, a new celestial (Kohler) with its own challenge
realm, Matter Dimensions, and a third Black Hole system.

The fundamental design philosophy is the same as the base game — cascading
prestige resets with increasing power — but the mod adds significant
**horizontal complexity** within the Mending layer rather than stacking
prestige layers vertically. After Mending, the player unlocks:
1. Corruption (modifier system for harder Mends with better rewards)
2. Warp upgrades (permanent post-Mending progression)
3. Multiversal Dimensions (new resource production loop)
4. Kohler celestial (a challenge realm with internal upgrades and
   milestones)
5. Matter Dimensions (a sub-system within Infinity Challenge 9)

The mod also introduces a **Transcendents** framework — a planned system
of multiple post-game celestial-like entities (Kohler, Sxy, Hexus, Blight,
Asw, Wollec) though only Kohler appears to be substantially implemented.

---

## 2. Size Comparison

| Metric | Base Game | Redemption Mod | Difference |
|--------|----------:|--------------:|-----------:|
| JS files (src/) | 270 | 313 | +43 |
| Vue components | 409 | 474 | +65 |
| JS lines (src/) | 66,797 | 78,589 | +11,792 (+18%) |
| Vue lines (src/) | 52,333 | 60,308 | +7,975 (+15%) |
| Core JS files | 238 | 281 | +43 |
| Core JS lines | 62,372 | 73,665 | **+11,293 (+18%)** |
| Secret formula lines | 28,637 | 32,636 | +3,999 (+14%) |

**Summary:** The Redemption mod adds approximately **11,300 lines of game
logic** (18% increase) and **65 new Vue components** for the UI. This is
comparable in scope to the Endgame mod (~11,100 lines, 18% increase) and
larger than the Vis mod (~8,500 lines, 14% increase).

### Key File Changes (Lines Added)

| File | Base | Redemption | +Lines | Notes |
|------|-----:|-----------:|-------:|-------|
| `src/core/player.js` | 1,089 | 1,425 | +336 | New state fields |
| `src/game.js` | 1,117 | 1,434 | +317 | New tick logic |
| `src/core/currency.js` | 479 | 624 | +145 | 8 new currencies |
| `src/core/dilation.js` | 273 | 349 | +76 | Extended mechanics |
| `src/core/dimboost.js` | 263 | 339 | +76 | Extended mechanics |
| `src/core/galaxy.js` | 178 | 245 | +67 | Extended mechanics |
| `src/core/constants.js` | 507 | 568 | +61 | New Decimal constants |
| `src/core/replicanti.js` | 576 | 631 | +55 | Extended mechanics |
| `src/core/eternity.js` | 357 | 397 | +40 | Extended mechanics |
| `src/core/reality.js` | 843 | 859 | +16 | Minor integration |
| `src/core/celestials/pelle/pelle.js` | 447 | 463 | +16 | Minor integration |
| `src/core/math.js` | 1,489 | 1,489 | 0 | No changes |

### Entirely New Files (Core Logic)

| File | Lines | Purpose |
|------|------:|---------|
| `expo-black-hole.js` | 687 | 3rd Black Hole system |
| `mending.js` | 624 | Mending prestige reset logic |
| `dimensions/matter-dimension.js` | 424 | Matter Dimension mechanics |
| `celestials/laitela/dark-matter-dimension_rewrite.js` | 299 | DMD rewrite |
| `dimensions/multiversal-dimension.js` | 228 | Multiversal Dimensions |
| `mending-upgrades.js` | 198 | Mending upgrade purchases |
| `kohler-upgrades.js` | 154 | Kohler upgrades |
| `warp-upgrades.js` | 150 | Warp upgrades |
| `kohler-infinity-upgrades.js` | 148 | Kohler Infinity upgrades |
| `matter-upgrades.js` | 146 | Matter upgrades |
| `corruption-upgrades.js` | 143 | Corruption upgrades |
| `celestials/kohler.js` | 96 | Kohler celestial runtime |
| `autobuyers/mending-autobuyer.js` | 122 | Mending automation |
| `autobuyers/memory-autobuyer.js` | 85 | Ra memory automation |
| `kohler-milestones.js` | 76 | Kohler milestones |
| `autobuyers/pelle-upgrade-autobuyer.js` | 64 | Pelle upgrade automation |
| `autobuyers/galgen-autobuyer.js` | 61 | Galaxy Gen automation |
| `celestials/ra/ra-upgrades.js` | 61 | Ra upgrade state |
| `autobuyers/singulaity-cap-autobuyer.js` | 59 | Singularity cap automation |
| `autobuyers/ra-pet-autobuyer.js` | 44 | Ra pet automation |
| `corruption.js` | 39 | Corruption state/scoring |
| `autobuyers/nr-reality-upgrade-autobuyer.js` | 30 | RU automation |
| `autobuyers/nr-imaginary-upgrade-autobuyer.js` | 30 | IU automation |
| `autobuyers/expo-black-hole-power-autobuyer.js` | 29 | ExoBH automation |
| `autobuyers/tesseract-autobuyer.js` | 29 | Tesseract automation |
| `autobuyers/music-glyph-autobuyer.js` | 27 | Music Glyph automation |
| `garble-text.js` | 26 | Text garbling helper |
| `celestials/destroyer.js` | 14 | Destroyer celestial shell |
| **Total new files** | **~4,070** | |

### New Data Files (secret-formula/mending/)

| File | Lines | Purpose |
|------|------:|---------|
| `warp-upgrades.js` | 357 | 25 Warp upgrade definitions |
| `corruption-upgrades.js` | 340 | 25 Corruption upgrade definitions |
| `kohler-upgrades.js` | 248 | 25 Kohler upgrade definitions |
| `matter-upgrades.js` | 232 | 20 Matter upgrade definitions |
| `kohler-infinity-upgrades.js` | 209 | 20 Kohler Infinity upgrade defs |
| `mending-upgrades.js` | 165 | 20 Mending upgrade definitions |
| `corruption.js` | 101 | Corruption penalty tables |
| `kohler-milestones.js` | 68 | Kohler milestone definitions |
| `mending-milestones.js` | 45 | Mending milestone definitions |
| `kohler-unlock-progress.js` | 40 | Kohler unlock progress stages |
| `index.js` | 21 | Export aggregator |
| `celestials/ra-upgrades.js` | 252 | Ra memory upgrades |
| `celestials/quotes/kohler.js` | 29 | Kohler dialogue |
| `celestials/quotes/destroyer.js` | 10 | Destroyer dialogue |
| `challenges/corruption-challenge.js` | 27 | Corruption challenge data |
| **Total data files** | **~2,144** | |

---

## 3. New Systems Added

### 3.1 Mending (New Prestige Layer)

**File:** `src/core/mending.js` (624 lines)

Mending is the new prestige layer above Reality. It triggers when:

```javascript
get canMend() {
  return (Ra.unlocks.exitDoom.isUnlocked
      ? Pelle.isDoomed
      : player.isGameEnd)
    || (MendingMilestone.six.isReached
        && player.antimatter.exponent >= 9e15)
    || Kohler.isRunning;
}
```

Three paths to Mend:
1. **Default:** Complete Pelle (be in Doomed state with exitDoom
   unlocked, or reach game end)
2. **After 10 Mends:** Reach antimatter exponent ≥ 9e15 outside Pelle
   (MendingMilestone.six)
3. **Kohler realm:** Always mendable while Kohler is running

**Reset behavior:** Mending resets *everything* — all prestige layers, all
celestial progress, challenges, glyphs, Black Hole state, etc. (~260 lines
of state clearing). Before the reset, corruption scoring is calculated and
fragments are awarded.

**What persists across Mending:**
- Mend count (`player.mends`)
- Mending Points (`player.mendingPoints`)
- Mending Upgrades
- Warp Upgrades
- Corruption Upgrades (with optional respec)
- Corruption records/fragments
- Multiversal Dimensions and Galaxy Boost Points
- Kohler upgrades and milestones
- Transcendent states

**Currencies earned on Mend:**
- **Mending Points (MP):** Primary currency, used for Mending Upgrades,
  Warp Upgrades, and Multiversal Dimensions
- **Mends:** Count of resets, used for milestone thresholds
- **Corruption Fragments:** Earned from corrupted Mends based on
  corruption score
- **Kohler Points (KP):** Earned if Mending while Kohler is running

### 3.2 Mending Upgrades

**File:** `src/core/mending-upgrades.js` (198 lines)
**Data:** `secret-formula/mending/mending-upgrades.js` (165 lines)

20 upgrades purchased with Mending Points:
- **4 rebuyables:** various boost multipliers
- **16 one-time:** start with boosts, remove requirements, delay
  softcaps, extend glyph effects, Reality/Imaginary upgrade automation

### 3.3 Mending Milestones

**File:** `secret-formula/mending/mending-milestones.js` (45 lines)

Passive bonuses unlocked at Mend counts:

| Mends | Key Reward |
|------:|------------|
| 1 | Start broken, maxed autobuyers, START perk, x1e20 IP, x1000 Rep speed, x1e5 EP, x1e4 eternities, etc. |
| 2 | Pelle dilation upgrades outside Doomed, Achievement 154 at 100% |
| 3 | Start with all perks, both Black Holes permanently active with 3 power upgrades |
| 4 | Teresa container persists, celestial memories start at level 10 |
| 5 | Remove Reality/Imaginary Upgrade requirements, start with 5 rebuyable RU |
| 7 | +3 Glyph slots, equip two Effarig and two Reality Glyphs |
| 8 | Teresa perk shop maxed, Music Glyph autobuyer |
| 10 | **Mend outside Doomed Reality** (unlock early-mend path) |
| 15 | Multiversal Remain multiplier from Glyph count |
| 20 | Non-rebuyable Reality/Imaginary Upgrade autobuyers |

### 3.4 Corruption System

**File:** `src/core/corruption.js` (39 lines)
**Data:** `secret-formula/mending/corruption.js` (101 lines)

Corruption is a modifier system for Mending runs (analogous to Eternity
Challenge conditions). The player sets 10 **hostility levels** (0–11 each)
that impose penalties during the next Mend in exchange for a higher
**corruption score**:

```javascript
calcBaseScore() {
  const corruptionScores =
    [1, 1.2, 1.45, 1.7, 2, 2.5, 3, 3.5, 4, 5, 7, 11];
  let finalScore = corruptionScores[corruption[0]];
  for (let i = 1; i < 10; i++) {
    finalScore *= corruptionScores[corruption[i]];
  }
  return finalScore;
}
```

The final score applies a **simplicial multiplier** based on how many
corruptions are active and their average level, plus an optional exponent
from Corruption Upgrade 20. Higher scores yield more Corruption Fragments.

**Corruption penalty categories** (from the penalty tables):
- Prestige gain nerfs
- Dimension production nerfs
- Time compression effects
- Glyph stat reductions
- Galaxy strength reductions
- Various other modifiers

### 3.5 Corruption Upgrades

**File:** `src/core/corruption-upgrades.js` (143 lines)
**Data:** `secret-formula/mending/corruption-upgrades.js` (340 lines)

25 upgrades purchased with Corruption Fragments:
- **5 rebuyables:** scaling boosts
- **20 one-time:** memory gain, game speed, glyph stats, alchemy cap,
  Black Hole bases, Multiversal Remain boosts, exponentiate corruption
  score (HU20)

Only purchasable during a **corrupted Mend** (when
`player.mending.corruptionChallenge.corruptedMend` is true).

### 3.6 Warp Upgrades

**File:** `src/core/warp-upgrades.js` (150 lines)
**Data:** `secret-formula/mending/warp-upgrades.js` (357 lines)

25 upgrades purchased with Mending Points, unlocked when
`player.reality.warped` is true:
- **5 rebuyables:** with hybrid cost scaling
- **20 one-time:** Infinity Power softcap delay, Ra memory gain, game
  speed softcap removal, galaxy-related boosts, hostility increases,
  Black Hole autobuyer, Multiversal Remain enhancements

Each upgrade has a requirement condition that must be met before it becomes
available for purchase.

### 3.7 Multiversal Dimensions

**File:** `src/core/dimensions/multiversal-dimension.js` (228 lines)

A **5th dimension type** (alongside Antimatter, Infinity, Time, and Dark
Matter Dimensions) with **8 tiers**:

- **Currency:** Purchased with Mending Points
- **Production chain:** tier 8 → 7 → ... → 2 → 1 → **Galaxy Boost
  Points**
- **Base costs:** range from 1e25 (tier 1) to 9.99e999 (tier 8)
- **Cost scaling:** exponential with cost multiplier increases at e2000,
  e8000, e22000 thresholds, plus e6000 super-scaling at 5000 purchases
- **Per-purchase multiplier:** 4x (all tiers)
- **Common multiplier:** cached, includes various upgrade effects
- **Galaxy Boost:** tier 1 output `galBoostPoints` provides a multiplier
  calculated as `(galBoostPoints^(1/log10^0.8)) / 100 + 1`

**Architecture:** Follows the same `DimensionState` base class pattern as
other dimensions.

### 3.8 Kohler (New Celestial)

**File:** `src/core/celestials/kohler.js` (96 lines)

Kohler is the first **Transcendent** — a post-Mending celestial that runs
as an alternative game mode. The player enters Kohler's realm and must
achieve milestones under its restrictions.

**Unlock progression** (100% total, multi-stage):
1. **Stage 1 (15%):** based on corruption record score (log-scaled to
   5e6)
2. **Stage 2 (15%):** based on record Corruption Fragments (scaled to 40)
3. **Stage 3 (30%):** based on antimatter exponent (log10 scaled from
   20–25)
4. **Stage 4 (35%→100%):** requires Multiversal Dimension 3 amount > 0

**Name reveal:** Kohler's name is hidden ("???") until `quoteBits >= 7`.

**Symbol:** `<i class='fa-solid fa-staff-snake'></i>`

**Running states:**
- `player.transcendents.kohler.run` — normal Kohler realm
- `player.transcendents.kohler.trueRun` — "true" final transcendent mode

### 3.9 Kohler Upgrades & Milestones

**File:** `src/core/kohler-upgrades.js` (154 lines)
**Data:** `secret-formula/mending/kohler-upgrades.js` (248 lines)

25 Kohler upgrades purchased with Kohler Points:
- **5 rebuyables:** KP gain, game speed, AD multiplier, IP gain,
  fragment gain
- **20 one-time:** start perks, galaxy boosts, Matter gain, unlock
  Kohler Infinity Upgrades, various progression boosts
- Some upgrades trigger side effects on purchase (setting player state)

**Kohler Milestones** (`kohler-milestones.js`, 76 lines):
Milestone IDs 11–15, 21–25 representing game-state achievements within
Kohler's realm:
- Reach Infinity
- Complete IC4
- Reach Eternity
- Reach Reality
- Complete Effarig
- (And more progression gates)

### 3.10 Kohler Infinity Upgrades

**File:** `src/core/kohler-infinity-upgrades.js` (148 lines)
**Data:** `secret-formula/mending/kohler-infinity-upgrades.js` (209 lines)

20 upgrades purchased with Infinity Points, only available while Kohler
is running and after Kohler Upgrade 20 is purchased:
- **5 rebuyables:** IP gain, ID multiplier, Replicanti speed, game speed,
  energy gain
- **15 one-time:** Matter boosts, energy effects, IC9 unlock, distant
  galaxy delay, Matter Dimension unlocks

### 3.11 Matter Dimensions

**File:** `src/core/dimensions/matter-dimension.js` (424 lines)

A dimension type available only within **Infinity Challenge 9**:
- **4 tiers** (fewer than other dimension types)
- **Production chain:** tier 4 → 3 → 2 → 1 → Matter currency
- **Currency for purchase:** Matter/energy-based
- **Per-10-bought multiplier:** with Matter Boost power system
- **Multiplier sources:** KohlerInfinityUpgrade(19) for tier 1, various
  Matter Upgrades, Corruption Upgrades

**Architecture:** Uses `DimensionState` base class. Gated behind IC9
completion and specific Kohler Infinity Upgrades.

### 3.12 Matter Upgrades

**File:** `src/core/matter-upgrades.js` (146 lines)
**Data:** `secret-formula/mending/matter-upgrades.js` (232 lines)

20 upgrades purchased with Matter currency, only available in IC9:
- **5 rebuyables:** Matter gain, game speed in IC9, IP gain, KP gain,
  energy interactions
- **15 one-time:** unlock Matter Dimensions, energy boosts, various
  scaling improvements

### 3.13 Expo Black Hole

**File:** `src/core/expo-black-hole.js` (687 lines)

A **3rd Black Hole** system (separate from the base game's two Black
Holes), unlocked via `Ra.unlocks.unlock3rdBH`:

- Uses the same interval/power/duration upgrade pattern as base Black
  Holes but with different parameters
- **Interval:** starts at 3600s, ×0.8 per upgrade
- **Power:** starts at 1.05^n (exponential game speed multiplier)
- **Duration:** starts at 10s, ×1.3 per upgrade
- **Upgrade currency:** Imaginary Machines (with hybrid cost scaling)
- Supports pause/unpause, auto-pause modes, phase tracking, permanence
- Has its own autobuyer system
- Disabled during Enslaved runs and Pelle blackhole-disabled states

**Architecture:** Closely mirrors the base `BlackHoleState` class
structure but is a separate implementation (`ExpoBlackHoleState` class)
rather than extending the existing system. This is likely because it uses
`getHybridCostScaling` for upgrade costs rather than the base game's
simpler cost model.

### 3.14 Destroyer (New Celestial)

**File:** `src/core/celestials/destroyer.js` (14 lines)

A minimal placeholder celestial — always unlocked, has quotes but no
gameplay mechanics yet. Likely planned for future content.

### 3.15 Transcendents Framework

**Player state:** `player.transcendents` defines slots for multiple
post-game entities:

```javascript
transcendents: {
  kohler: { run: false, trueRun: false },
  sxy: { run: false },
  hexus: { run: false },
  blight: { run: false },
  asw: { run: false },
  wollec: { run: false },
}
```

Only Kohler is substantially implemented. The others are placeholder
entries for planned content, suggesting a design where each Transcendent
is a challenge realm with its own `run` state (similar to celestials but
at a higher tier).

---

## 4. Modifications to Existing Systems

The Redemption mod makes pervasive changes to existing systems by adding
`MendingUpgrade`, `WarpUpgrade`, `CorruptionUpgrade`, `KohlerUpgrade`,
and `MendingMilestone` checks throughout:

### Game Loop (+317 lines)
- `MultiversalDimensions.tick(realDiff)` — new dimension production
- `CorruptionData.update()` — sync corruption state each tick
- Mend-time tracking (`player.records.thisMend.realTime`,
  `player.records.thisMend.time`)
- `applyAutoprestige(realDiff)` integration
- Expo Black Hole phase updates

### Dilation (+76 lines)
- New dilation upgrades (IDs 11-13) for Pelle content accessible outside
  Doomed (Mending Milestone 2)
- Modified tachyon particle formulas

### Dimension Boost (+76 lines)
- Mending Upgrade effects on boost power
- Kohler and corruption integration
- New boost sources from Warp Upgrades

### Galaxy System (+67 lines)
- Galaxy Boost Points integration (from Multiversal Dimensions)
- Corruption penalty application to galaxy strength
- New scaling delays from Warp Upgrades

### Replicanti (+55 lines)
- Modified growth formulas with mending multipliers
- Kohler Infinity Upgrade effects on replicanti speed

### Eternity (+40 lines)
- Mending milestone passive effects
- Kohler integration

### Currency System (+145 lines)
- 8 entirely new Currency definitions (see §5)

### Many Decimal → Decimal conversions
The Redemption mod converts several time-tracking values from `Number` to
`Decimal`:
- `records.thisInfinity.time` / `bestInfinity.time`
- `records.thisEternity.time` / `bestEternity.time`
- `records.thisReality.time` / `bestReality.time`
- `records.bestRSmin` / `bestRSminVal`
- All `records.thisMend.*` fields

---

## 5. New Currencies

| Currency | Type | Source | Use |
|----------|------|--------|-----|
| `mendingPoints` | Decimal | Mending prestige | Mending Upgrades, Warp Upgrades, Multiversal Dims |
| `mends` | Decimal | Mending count | Milestone thresholds |
| `corruptionFragments` | Decimal | Corrupted Mend score | Corruption Upgrades |
| `raPoints` | Decimal | Ra memory system | Ra upgrades |
| `galBoostPoints` | Decimal | Multiversal Dim 1 output | Galaxy boost multiplier |
| `kohlerPoints` | Decimal | Mending in Kohler realm | Kohler Upgrades |
| `energy` | Decimal | Matter Dimensions/IC9 | Matter Dim boosts |
| `weakMatter` | Decimal | IC9 sub-system | Matter Upgrades (cost) |

---

## 6. New Player State

The Redemption mod adds approximately **336 lines** to `player.js`:

```javascript
player = {
  // ... existing state ...
  weakMatter: DC.D0,
  energy: DC.D0,
  mends: DC.D0,
  corruptedFragments: DC.D0,
  galBoostPoints: DC.D0,
  kohlerMilestoneBits: Array.repeat(0, 2),
  mending: {
    kohlerUpgradeBits: 0,
    kohlerUpgradeReqs: 0,
    upgradeBits: 0,
    warpUpgradeBits: 0,
    cuRespec: false,
    corruptionUpgradeBits: 0,
    corruptionUpgReqs: 0,
    warpUpgReqs: 0,
    reqLock: { mending: 0, warp: 0, corruption: 0 },
    rebuyables: { 1: 0, 6: 0, 11: 0, 16: 0 },
    warpRebuyables: { 1: 0, 2: 0, 3: 0, 4: 0, 5: 0 },
    corruptionRebuyables: { 1: 0, 2: 0, 3: 0, 4: 0, 5: 0 },
    kohlerRebuyables: { 1: 0, 2: 0, 3: 0, 4: 0, 5: 0 },
    corruption: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    corruptionBackup: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    corruptedFragments: 0,
    corruptionChallenge: {
      corruptedMend: false,
      records: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      recordScore: 0,
    },
    spentCF: 0,
    corruptNext: false,
  },
  expoBlackHole: [{ /* interval/power/duration/phase/active */ }],
  expoBlackHolePause: false,
  expoBlackHoleAutoPauseMode: 0,
  expoBlackHolePauseTime: 0,
  expoBlackHoleNegative: 1,
  celestials: {
    // ... existing ...
    destroyer: { quoteBits: 0 },
    kohler: {
      run: false,
      quoteBits: 0,
      unlockProgress: 0,
      unlockMilestone: [false, false, false, false, false, false, false],
    },
  },
  transcendents: {
    kohler: { run: false, trueRun: false },
    sxy: { run: false },
    hexus: { run: false },
    blight: { run: false },
    asw: { run: false },
    wollec: { run: false },
  },
  mendingPoints: DC.D0,
  mendingUpgrades: new Set(),
  mvrmultUpgrades: 0,
  kohlerPoints: new Decimal(0),
  bestKohlerPoints: new Decimal(0),
  records: {
    // ... adds thisMend, bestMend ...
    thisMend: { time: DC.D0, realTime: DC.D0, maxAM: DC.D0,
                maxIP: DC.D0, maxEP: DC.D0, maxRM: DC.D0 },
    bestMend: { time: DC.D0, realTime: DC.D0 },
  },
  reality: {
    // ... existing ...
    warped: false,  // Warp system unlocked flag
  },
};
```

---

## 7. Architecture Assessment

### Positive Aspects

1. **Follows existing patterns:** Mending Upgrades use
   `BitPurchasableMechanicState`, Kohler uses celestial patterns,
   Multiversal/Matter Dimensions use `DimensionState`. No fundamentally
   new architectural paradigms.

2. **Modular upgrade systems:** Each upgrade category (Mending, Warp,
   Corruption, Kohler, KohlerInfinity, Matter) is self-contained with
   its own file pair (logic + data).

3. **Data-driven:** All configs live in `secret-formula/mending/`
   following the established pattern.

4. **Incremental complexity:** Systems build on each other (Mend →
   Corruption → Warp → Multiversal → Kohler → Matter).

### Concerns

1. **Large reset function:** `mendingReset()` is ~260+ lines of direct
   state mutation with deeply nested conditionals for what to preserve.
   Same issue as the Endgame mod's reset function.

2. **Partially implemented content:** The Transcendents framework has 6
   slots but only Kohler is built. Destroyer is a placeholder. This
   suggests the mod is in active development.

3. **Code quality variance:** Comments like "hello, due to some upgrade
   need record to involve, corruption should be at first sry.--sxy" and
   `eslint-disable` directives throughout suggest the codebase has
   multiple contributors with varying style discipline.

4. **Expo Black Hole duplication:** Rather than generalizing the existing
   Black Hole system, the mod creates a parallel 687-line implementation
   (`ExpoBlackHoleState`). This doubles the Black Hole maintenance
   surface area.

5. **Scattered conditionals:** Mending effects are checked via
   `MendingUpgrade(N).isBought`, `WarpUpgrade(N).isBought`,
   `KohlerUpgrade(N).isBought`, `MendingMilestone.X.isReached` scattered
   across existing files.

6. **Time type widening:** Many time fields converted from `Number` to
   `Decimal`, which changes arithmetic patterns throughout the codebase.

---

## 8. Porting Considerations for Rust

### Additional Challenges Beyond Base Game

1. **Corruption Scoring System:** The corruption score calculation uses
   multi-dimensional arrays of penalty values and a combinatorial
   multiplier. In Rust, this maps cleanly to `const` arrays with a pure
   scoring function.

2. **5th Dimension Type:** Multiversal Dimensions follow the same
   `DimensionState` pattern with 8 tiers and hybrid cost scaling. No new
   paradigms needed beyond what Antimatter/Infinity/Time Dimensions
   require.

3. **6th Dimension Type:** Matter Dimensions add a 4-tier dimension
   system gated behind IC9. Smaller but follows the same pattern.

4. **3rd Black Hole:** The Expo Black Hole is architecturally identical
   to the base Black Holes but with different parameters. In Rust, this
   could be handled by parameterizing the existing Black Hole struct
   rather than duplicating it.

5. **Complex Reset Logic:** The Mending reset (~260 lines) has many
   conditional branches based on milestones and upgrades. Same approach
   as Endgame: model as a `ResetPolicy` struct.

6. **Time Type Widening:** Time-tracking fields become `Decimal` in the
   mod. In Rust, using `Decimal` for all time values from the start
   avoids this issue.

7. **Transcendents Framework:** Only Kohler is implemented, but the
   framework suggests a trait-based approach in Rust where each
   Transcendent implements a `TranscendentRealm` trait.

### No New Architectural Paradigms

The Redemption mod introduces **no fundamentally new patterns**. Every
new system uses the same framework:
- Config in `secret-formula/` → `GameMechanicState` subclass → accessor
- Currencies follow the same `Currency` abstraction
- Dimensions follow the same `DimensionState` pattern
- Upgrades follow `BitPurchasableMechanicState` / `RebuyableMechanicState`
- Celestials follow the celestial runtime pattern (state + quotes +
  run flag)

### Comparison with Other Mods

| Aspect | Endgame | Vis | Redemption |
|--------|---------|-----|------------|
| Core lines added | +11,105 (18%) | +8,478 (14%) | +11,293 (18%) |
| New dimension types | 1 (Celestial, 8 tiers) | 1 (Chaos, 12 tiers) | 2 (Multiversal 8T, Matter 4T) |
| New celestials | 1 (Alpha) | 3 (Glitch, Cante, Null) | 2 (Kohler, Destroyer*) |
| Prestige layer | Endgame (above Reality) | Meta (above Reality) | Mending (above Reality) |
| Unique mechanic | Mastery Tree | Challenge-achievements | Corruption scoring |
| Number type pressure | Very high (1e(1e150)) | High | Medium-High |
| Implementation maturity | High | High | Medium (placeholders) |

*Destroyer is a placeholder with no mechanics.

### Revised Phase Plan (Including Redemption)

```
Phase 1-4: (Same as base game analysis)

Phase 5 (Redemption Core):
  Mending prestige layer → Mending Points/Mends
  → Mending Upgrades → Mending Milestones
  → Corruption system (penalty tables + scoring)
  → Corruption Upgrades → Corrupted Mend flow

Phase 6 (Redemption Expansion):
  Warp Upgrades (requirement-gated progression)
  → Multiversal Dimensions (8-tier, Galaxy Boost Points)
  → Expo Black Hole (3rd BH system)
  → Kohler celestial (unlock progression, realm)
  → Kohler Upgrades/Milestones/Infinity Upgrades
  → Matter Dimensions (IC9, 4-tier)
  → Matter Upgrades → Energy system
```

### Estimated Additional Porting Effort

| Category | Lines to Port | Complexity |
|----------|-------------:|:----------:|
| New systems (clean files) | ~4,070 | Low-Medium |
| Modifications to existing systems | ~1,100 | Medium-High |
| Config/data definitions | ~2,144 | Low |
| Scattered conditionals in existing code | ~1,200 | Medium |
| Number type widening changes | ~400 | Medium |
| **Total** | **~8,914** | |

The Redemption mod represents approximately **14% additional porting
effort** beyond the base game, primarily due to:
- The Mending reset function (complex conditional logic)
- The Expo Black Hole (687 lines mirroring base BH logic)
- Integration points with existing systems (many small changes)
- The corruption scoring/penalty system (mathematical formulas)

The new standalone systems (Multiversal Dimensions, Matter Dimensions,
Kohler, upgrade categories) are straightforward since they follow
established patterns.

---

*Document generated by codebase analysis. Last updated: 2026-06-25.*
