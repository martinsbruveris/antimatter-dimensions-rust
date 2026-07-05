---
status: Reference
---

# Antimatter Dimensions Vis — Codebase Analysis

## Table of Contents

1. [Overview & Comparison with Base Game](#1-overview--comparison-with-base-game)
2. [Size Comparison](#2-size-comparison)
3. [New Systems Added](#3-new-systems-added)
   - 3.1 [Meta (New Prestige Layer)](#31-meta-new-prestige-layer)
   - 3.2 [Glitch (New Celestial)](#32-glitch-new-celestial)
   - 3.3 [Glitch Challenges](#33-glitch-challenges)
   - 3.4 [Glitch Reality & Speed Upgrades](#34-glitch-reality--speed-upgrades)
   - 3.5 [Glitch Rifts](#35-glitch-rifts)
   - 3.6 [Chaos Dimensions](#36-chaos-dimensions)
   - 3.7 [Plynia](#37-plynia)
   - 3.8 [Challenger & Hard Challenger Upgrades](#38-challenger--hard-challenger-upgrades)
   - 3.9 [Cante (New Celestial)](#39-cante-new-celestial)
   - 3.10 [Null (New Celestial)](#310-null-new-celestial)
   - 3.11 [Glitch Glyph Type](#311-glitch-glyph-type)
4. [Modifications to Existing Systems](#4-modifications-to-existing-systems)
5. [New Currencies](#5-new-currencies)
6. [New Player State](#6-new-player-state)
7. [Architecture Assessment](#7-architecture-assessment)
8. [Porting Considerations for Rust](#8-porting-considerations-for-rust)

---

## 1. Overview & Comparison with Base Game

The "vis" mod (the name likely derives from the Latin word for "power, force,
or strength") extends Antimatter Dimensions with substantial post-Reality
content. While the endgame mod adds a clean 5th prestige layer above Reality,
the vis mod takes a different approach: it adds a new prestige layer called
**Meta** and three new celestials (**Glitch**, **Cante**, **Null**), each
with interconnected mechanics, resources, and progression paths.

The mod's content sits between Pelle (the base game's final celestial) and
the Meta prestige reset. The progression broadly follows: complete the base
game → Glitch celestial (challenge-achievements and Rift Force) → Challenger
upgrades (rework Pelle/Doom) → Chaos Dimensions → Plynia → Meta prestige →
Cante celestial (Replicators) → Null celestial (Cycles/Parallax/Corruption).

The fundamental design philosophy differs from the endgame mod in several
ways:
- **Interconnected rather than linear:** Systems cross-reference each other
  extensively (e.g., Challenger Essence scales from Chaos Cores, Chaos
  Dimensions scale from Challenger Essence, Plynia resets Chaos Dimensions).
- **Challenge-achievement hybrids:** Instead of numbered challenge modes,
  Glitch challenges are one-time unlockable achievements with permanent
  effects, gated by specific game-state conditions.
- **Multiple celestials instead of one prestige layer:** Rather than stacking
  a single deep prestige system, the mod adds three celestials, each with
  distinct internal reset loops.

---

## 2. Size Comparison

| Metric | Base Game | Vis Mod | Difference |
|--------|----------:|--------:|-----------:|
| JS files (src/) | 270 | 303 | +33 |
| Vue components | 409 | 438 | +29 |
| JS lines (src/) | 66,797 | 73,500 | +6,703 (+10%) |
| Vue lines (src/) | 52,333 | 56,232 | +3,899 (+7%) |
| Core JS files | 238 | 281 | +43 |
| Core JS lines | 62,372 | 70,850 | **+8,478 (+14%)** |
| Secret formula lines | 28,637 | 30,952 | +2,315 (+8%) |

**Summary:** The vis mod adds approximately **8,500 lines of game logic**
(14% increase) and **29 new Vue components** for the UI. This is comparable
in scope to the endgame mod (~11,100 lines, 18% increase), though somewhat
smaller. The new content is roughly equivalent to adding two celestials'
worth of mechanics.

### Entirely New Files (Core Logic)

| File | Lines | Purpose |
|------|------:|---------|
| `glitchChallengeUG.js` | 421 | All Glitch challenge + Challenger state |
| `meta.js` | 385 | Meta prestige reset logic |
| `celestials/laitela/dmd.js` | 331 | Rewritten Dark Matter Dimensions |
| `celestials/glitch/glitch.js` | 286 | Glitch celestial runtime |
| `dimensions/chaos-dimension.js` | 258 | 4th dimension type (12 tiers) |
| `celestials/cante/canteReplicator.js` | 250 | 10-tier self-replicator engine |
| `glitchRealityUpgrades.js` | 227 | Glitch Reality + Speed upgrade logic |
| `celestials/null/nullCycle.js` | 226 | 16-tier ring/cycle engine |
| `celestials/glitch/glitchrift.js` | 123 | Glitch Rift milestone system |
| `meta-fabricator-upgrades.js` | 107 | Meta Fabricator upgrade logic |
| `celestials/null/nullParallax.js` | 88 | Null Parallax reset layer |
| `celestials/null/nullCorrupt.js` | 64 | Null Corruption reset layer |
| `celestials/glitch/plynia.js` | 63 | Chaos Dimension reset sublayer |
| `celestials/null/null.js` | 60 | Null celestial shell |
| `celestials/cante/cante.js` | 60 | Cante celestial shell |
| `celestials/null/nullUpgrades.js` | 36 | Null upgrade purchase wrapper |
| `celestials/cante/canteUpgrades.js` | 33 | Cante upgrade purchase wrapper |
| `storage/be-migrations.js` | 264 | Save migration for new state |
| `secret-formula/reality/core-glyph-info.js` | 490 | Centralized glyph metadata |
| `secret-formula/celestials/glitchupgrades.js` | 208 | 16 Glitch power upgrades |
| `secret-formula/celestials/cante.js` | 197 | 20 Cante upgrades + unlock data |
| `secret-formula/celestials/glitchrift.js` | 185 | 4 Glitch Rift definitions |
| `secret-formula/celestials/null.js` | 165 | 20 Null upgrades + passcode data |
| `secret-formula/celestials/glitchspeed.js` | 103 | 8 Glitch Speed upgrades |
| `secret-formula/meta/metaUpgrades.js` | 218 | 25 Meta Fabricator upgrades |
| `secret-formula/glitch/` (6 files) | 620 | Challenge data (34 challenges) |
| `secret-formula/meta/` (other files) | 106 | Meta milestones + stability |
| `secret-formula/celestials/quotes/` (3) | 311 | Glitch/Cante/Null dialogue |
| New autobuyers (13 files) | 696 | Automation for new systems |
| **Total new files** | **~6,581** | |

### Key File Changes (Lines Added)

| File | Base | Vis | +Lines | Notes |
|------|-----:|----:|-------:|-------|
| `secret-formula/news.js` | 7,323 | 8,057 | +734 | New news ticker entries |
| `math.js` | 1,489 | 1,850 | +361 | New cost scaling classes |
| `player.js` | 1,089 | 1,402 | +313 | New state fields |
| `secret-formula/celestials/ra.js` | 296 | 513 | +217 | New Ra pets/unlocks |
| `dimensions/antimatter-dimension.js` | 676 | 846 | +170 | Glitch/Meta integration |
| `devtools.js` | 530 | 692 | +162 | Dev tools for new systems |
| `secret-formula/celestials/v.js` | 252 | 413 | +161 | V extreme mode + meta |
| `secret-formula/achievements/` | 1,386 | 1,526 | +140 | New achievements |
| `celestials/pelle/pelle.js` | 447 | 584 | +137 | Challenger/Joined system |
| `game.js` | 1,117 | 1,251 | +134 | New tick logic |
| `autobuyers/autobuyers.js` | 161 | 275 | +114 | New autobuyer registry |
| `currency.js` | 479 | 588 | +109 | 8 new currencies |
| `replicanti.js` | 576 | 675 | +99 | Extended mechanics |
| `format.js` | 217 | 312 | +95 | New format helpers |
| `reality.js` | 843 | 934 | +91 | Glitch/Meta integration |
| `dimboost.js` | 263 | 348 | +85 | Glitch/Meta effects |
| `dimensions/time-dimension.js` | 347 | 424 | +77 | Glitch/Meta effects |
| `black-hole.js` | 670 | 738 | +68 | Extended mechanics |
| `eternity.js` | 357 | 424 | +67 | Meta integration |
| `dilation.js` | 273 | 335 | +62 | Extended dilation |
| `celestials/ra/ra.js` | 449 | 510 | +61 | 3 new Ra pets |
| `galaxy.js` | 178 | 230 | +52 | Meta/Glitch effects |
| `celestials/V.js` | 234 | 281 | +47 | V extreme mode |

### Removed Files

| File | Lines | Reason |
|------|------:|--------|
| `celestials/navigation.js` | 1,992 | Celestial map removed |
| `payments.js` | 153 | In-app purchases removed |
| `storage/cloud-saving.js` | 306 | Cloud saving removed |
| `celestials/laitela/dark-matter-dimension.js` | 278 | Replaced by `dmd.js` |
| `secret-formula/reality/glyph-types.js` | 97 | Merged into `core-glyph-info.js` |
| `secret-formula/reality/glyph-sacrifices.js` | 98 | Merged into `core-glyph-info.js` |
| `secret-formula/celestials/navigation-sigils/` | 296 | Map removed |
| **Total removed** | **~3,220** | |

---

## 3. New Systems Added

### 3.1 Meta (New Prestige Layer)

**File:** `src/core/meta.js` (385 lines)

Meta is the new prestige layer above Reality. It triggers when antimatter
reaches `ee50` (formally `DC.PREMETAMAX`) and the `VUnlocks.metaReset`
achievement is earned.

**Reset behavior:** Meta resets *everything* — all prestige layers, all
celestial progress, Glitch, Pelle/Doom state, Chaos Dimensions, challenges,
and more. This is by far the most comprehensive reset in the mod (~300 lines
of state clearing).

**What persists across Meta (base):**
- Meta count (`player.meta.metas`)
- Meta Relays (`player.meta.metaRelays`)
- Meta Fabricator upgrades
- Meta Milestones
- Companion glyphs

**What persists conditionally (via milestones/upgrades):**
- Glyphs (levels reset to 1) — 4 Metas
- Glitch Challenges + Achievements — 4 Metas
- Infinity always broken — 4 Metas
- Reality upgrade requirements — 2 Metas
- Imaginary upgrade requirements — 3 Metas
- Effarig complete + permanent Black Holes — best Meta < 30m
- Ra glitch memory + Lai'tela autobuyers — best Meta < 15m
- Perks — MetaFabricatorUpgrade(18)
- Challenger bits + Pelle joined state — MetaFabricatorUpgrade(15)
- Chaos Cores + Plynia — MetaFabricatorUpgrade(22)

**Currencies earned:**
- **Meta Relays (MR):** Primary currency. Formula scales with antimatter
  (log-log), Meta count, and milestones/upgrades. Used to buy Meta Fabricator
  Upgrades.
- **Metas:** Count of Meta resets. Used for milestone thresholds and as a
  multiplier source.

### Meta Fabricator Upgrades

**File:** `src/core/meta-fabricator-upgrades.js` (107 lines)
**Data:** `src/core/secret-formula/meta/metaUpgrades.js` (218 lines)

25 upgrades purchased with Meta Relays:
- **5 rebuyables:** Reality Amplifiers, AD/ID/TD power, game speed,
  Singularity cap, Chaos Dimension power
- **20 one-time:** passive Rift Force, passive IP/EP/RM/infinities/
  eternities, preserve Teresa/Nameless/V/Challenger/perks/Ra/charged
  upgrades, unlock autobuyers for RU/IU/Glitch/Chaos/Singularity, start
  Metas with fully destabilized Lai'tela, Space Theorem generation, higher
  antimatter hardcap, unlock Cante/Null Ra memories

### Meta Milestones

**File:** `src/core/secret-formula/meta/metaMilestones.js` (60 lines)

8 milestones unlocked by Meta count or best Meta time:

| Milestone | Requirement | Key Effect |
|-----------|-------------|------------|
| 1 | 1 Meta | IP/EP/RM/Ra memory ^1.15; challenges completable anytime |
| 2 | 2 Metas | Start with 1e6 Realities; keep RU requirements |
| 3 | 3 Metas | Boost milestone 1; keep IU requirements |
| 4 | 4 Metas | Keep glyphs (level reset); keep challenges/achievements |
| 5 | Best < 1h | Faster MR gain |
| 6 | Best < 30m | Keep Effarig complete; permanent Black Holes |
| 7 | Best < 15m | Keep Ra glitch memory + Lai'tela autobuyers |
| 8 | Best < 5m | Reality/Glitch glyphs auto-level to alchemy cap |

### 3.2 Glitch (New Celestial)

**File:** `src/core/celestials/glitch/glitch.js` (286 lines)

Glitch is the mod's central celestial, unlocked after completing the Gamma
Glitch Rift (which requires completing all 14 Reality-era Glitch challenges).
Its symbol is not defined explicitly; it uses the "glitchyfishys" pet name
in Ra.

**Design concept:** Glitch Reality combines restrictions from multiple
existing celestials into a single run. The player selects up to **10 augment
effects**, each mimicking a different celestial's constraint:

| ID | Augment | Effect |
|----|---------|--------|
| 0 | Teresa Reality | Teresa's restrictions active |
| 1 | Effarig Reality | Effarig's restrictions active |
| 2 | Nameless Reality | Nameless ones' restrictions active |
| 3 | Nameless dim limit | Dimension limit active |
| 4 | Nameless low tachyon | Reduced Tachyon Particle gain |
| 5 | V's Reality | V's restrictions active |
| 6 | Ra no dim boost | Dimension Boosts disabled |
| 7 | Ra static tickspeed | Tickspeed frozen |
| 8 | Lai'tela's Reality | Lai'tela's restrictions at N dims |
| 9 | Timed decay | Timed resource decay |

The core resource loop:
1. Run Glitch Reality with augments active (forced cursed glyphs)
2. Generate **Rift Force** (RF) based on AM, IP, EP while all 9+ augments
   are active (or passively via MetaFabricatorUpgrade(6))
3. Convert RF into **Chaos Cores** (CC) for Chaos Dimensions
4. Spend RF on Glitch Reality Upgrades
5. Spend Challenger Essence on Challenger/Hard Challenger Upgrades

**Rift Force gain formula:**
```javascript
// Simplified:
form(value) = GlitchSpeedUpgrade(2).isBought
  ? (value + 1).log10() ^ 0.2
  : log10(log10(value + 1) + 1)
AM_factor = form(antimatter) ^ 1.25
IP_factor = form(IP) ^ 2
EP_factor = form(EP) ^ 3.5
total = AM * IP * EP   // with various multipliers
RF = total / 24 * upgrades * (CC boost)
```

**Notable quirk:** `Glitch.isUnlocked` has a side effect — it assigns
`player.celestials.pelle.joined = true` rather than checking it. This looks
like a bug where `=` was used instead of `===`.

### 3.3 Glitch Challenges

**Files:** `src/core/secret-formula/glitch/` (6 data files, 620 lines)
**Runtime:** `src/core/glitchChallengeUG.js` (421 lines)

Glitch challenges are **not** traditional numbered challenge modes. They are
**one-time challenge-achievement hybrids**: the player must reach a specific
game state under specific restrictions, then the challenge unlocks
permanently as a bitfield. Each completed challenge provides a permanent
passive effect and also fills the corresponding Glitch Rift.

**Total: 34 challenges across 4 eras:**

**Pre-Infinity (8)** — bought with Antimatter:

| Name | Restriction | Effect |
|------|-------------|--------|
| Dimless | 1e15 AM, no DB/galaxies | AD mult from Dim Boosts |
| Dimensional Limits | 1e25 AM, 1 DB, no galaxies | AD mult from tickspeed |
| Vector 2 | 1e35 AM, ≤2 DB, no galaxies | 25x oscillating AD mult |
| 8th Dimensional | 1e50 AM, ≤4 DB, 1 of each dim | 1st AD from 8th AD amount |
| Galactic Inforcement | 1e7 AM, no DB, 1 galaxy | Extra tickspeed from amount |
| Galactic Limitation | (details vary) | Tickspeed from DB×galaxies |
| Galactic Capacity | (details vary) | x2 IP |
| Galactic Instance | (details vary) | x3 IP |

**Break Infinity (6)** — bought with IP:

| Name | Key Effect |
|------|------------|
| Limited Space | IP from galaxies |
| Infinitely Limiting | x10 IP |
| Chaos | IC1 without IDs → x1e50 IP |
| Cosmic Infinity | Galaxy-based IP mult |
| Replicated Infinity | Replicanti speed from amount |
| Cloned Replication | Replicanti speed from RG count |

**Eternity (6)** — bought with EP:

| Name | Key Effect |
|------|------------|
| Eternal Forces | x5 EP |
| Eternity Power | Lower free-tickspeed threshold |
| Rifted | +3 RG |
| Study Forever | +150 free tickspeed |
| Wrong Timeline | EP from IP |
| Logical Tectonics | +1500 free tickspeed |

**Reality (14)** — bought with Rift Force:

| Name | Key Effect |
|------|------------|
| Realitize | x1e25 EP |
| Dialated | x15 TP |
| Real Time Complex | +25 perk points and realities |
| Immensity | TD x1e100 |
| Sacrificial Power | EC4/EC8 path req removed |
| Limiting Reality | +25 perk points |
| External Dilation | Extra glyph effect |
| Alternative Realitive | Auto/60x real-time storage |
| I hate V's Achievements | +10 Space Theorems |
| Ra forgot to make this | Unlock Glitch memory in Ra |
| Astral Confrontment | x50 DM/DE |
| Galactic Overload | Singularities scale with amount |
| Pre-Galactic | Unlock 2 Teresa shop items |
| Overlight Powerforce | Unlock Pelle Glitch Rift |

### 3.4 Glitch Reality & Speed Upgrades

**File:** `src/core/glitchRealityUpgrades.js` (227 lines)
**Data:** `src/core/secret-formula/celestials/glitchupgrades.js` (208 lines)
and `glitchspeed.js` (103 lines)

Two separate upgrade tracks, both purchased with Rift Force:

**Power track (16 upgrades):**
- **4 rebuyables:** more Rift Force gain, higher glyph sacrifice cap, higher
  refinement cap, more Singularities
- **12 one-time:** RF-powered AD boost, fewer forced cursed glyphs (5→4→2),
  higher Lai'tela cap, keep IP/EP/eternities on Glitch run, stronger Teresa/
  DMD/Singularity scaling, AD ^1.25, etc.

**Speed track (8 upgrades):**
Unlocked only after at least 1 Pelle reset (`pelleResets > 0`):
- Weaker celestial prestige nerfs (Teresa/V/Lai'tela IP powers)
- Altered Rift Force formula (stronger)
- Charge an Infinity upgrade in Glitch run
- **Unlock Chaos Dimensions**
- RF → Chaos Cores/Challenger Essence scaling
- Glitch glyph sacrifice scaling
- Chaos Dimension 1 multiplier from RF
- 1st AD power boost

### 3.5 Glitch Rifts

**File:** `src/core/celestials/glitch/glitchrift.js` (123 lines)
**Data:** `src/core/secret-formula/celestials/glitchrift.js` (185 lines)

Analogous to Pelle's Rifts, Glitch Rifts fill as the player completes Glitch
challenges in each era. Each rift has milestone rewards:

| Rift | Era | Fill Count | Key Milestones |
|------|-----|------------|----------------|
| Alpha | Pre-Infinity | 8 | 1st AD x7, 8th AD x30, x1.5 game speed, DB boost |
| Beta | Break-Infinity | 6 | IP conversion boost, lower free tickspeed threshold |
| Delta | Eternity | 6 | Rep speed, free tickspeed, weaker dilation nerf |
| Gamma | Reality | 14 | TD power, x10 perk points, glyph rarity, DMD power, +50 Ra cap, **unlock Glitch** |

Rift fill percentage = (challenges bought in era) / (total challenges in
era). Each milestone triggers at 25%, 50%, 75%, and 100% fill.

### 3.6 Chaos Dimensions

**File:** `src/core/dimensions/chaos-dimension.js` (258 lines)

A **4th dimension type** (alongside Antimatter, Infinity, and Time), with
**12 tiers** (more than any other dimension type):

- **Unlock:** Glitch Speed Upgrade 4 ("Unlimited Dimensions")
- **Currency:** Chaos Cores (produced by tier 1, spent on purchases)
- **Production chain:** tier 12 → 11 → ... → 2 → 1 → Chaos Cores
- **Cost scaling:** Exponential with a scalingCostThreshold at `e5000`
- **Per-purchase multipliers:** range from 10 (tier 1) to 1e200 (tier 12)

**Multiplier sources:**
- Hard Challenger Upgrades 4/5
- Glitch Speed Upgrades 5/7
- Meta Fabricator Upgrades 5/17
- Glitch glyph sacrifice effect
- Plynia multiplier (`100^plynias`)
- Glitch glyph `glitchChaosPow` power effect

**Architecture:** Follows the standard `DimensionState` base class pattern.
Uses `ExponentialCostScaling` (from the new `math.js` additions).

### 3.7 Plynia

**File:** `src/core/celestials/glitch/plynia.js` (63 lines)

A sub-reset layer for Chaos Dimensions, unlocked by Hard Challenger
Upgrade 6:

- **Requirement:** Chaos Dimension at tier `min(12, 8 + plynias)` must have
  ≥4 (or more, scaling) amount
- **Reset:** Resets Chaos Cores and all Chaos Dimension amounts
- **Reward:** Each Plynia gives a multiplicative boost (`100^plynias`) and
  power boost (`1 + 0.1 * plynias`) to all Chaos Dimensions
- **Progression:** Scaling requirement increases, but the boost grows
  exponentially

### 3.8 Challenger & Hard Challenger Upgrades

**Data:** `src/core/secret-formula/glitch/challenger.js` (135 lines) and
`challengerHard.js` (73 lines)
**Runtime:** part of `src/core/glitchChallengeUG.js`

Purchased with **Challenger Essence** (earned from "destroying" Pelle via
Armageddon with a special flow that increments `pelleResets`).

**Challenger Upgrades (22):**
These mostly re-enable base-game systems during Pelle's Doomed state:

| ID | Name | Effect |
|----|------|--------|
| 0 | A broken chain | Infinity/Break Infinity upgrades while Doomed |
| 1 | Fairly earned | All achievements enabled while Doomed |
| 2 | Unblock knowledge | All Time Studies effective while Doomed |
| 3 | Forever real | All Reality upgrades while Doomed |
| 4 | Permanently | All perks while Doomed |
| 5 | Gone for miles | All Eternity Milestones while Doomed |
| 6 | Rereset | Infinity/Eternity multipliers while Doomed |
| 7 | Multiplicitvity | IP/EP multipliers while Doomed |
| 8 | Automating automation | Pelle upgrade + Galaxy Gen autobuyers |
| 9 | Delivered dilation | Dilation upgrades while Doomed |
| 10 | Bugged Glitch | All Glitch challenges active while Doomed |
| 11 | Al-chemical | Alchemy active while Doomed |
| 12-21 | Various | Teresa/Effarig/Nameless/V/Ra/Lai'tela systems, Rift auto-fill, passive remnants, all glyph effects |

**Hard Challenger Upgrades (8):**
Post-Challenger cross-system boosters:
- Challenger Essence self-boost
- Galaxy Generator ×1e50
- Unlock Glitch layer-2 (speed) upgrades
- CE boosted by Chaos Cores
- Chaos Dimensions boosted by CE / gamespeed
- Unlock Plynia
- CE boosted by Rift Force × Chaos Cores

### 3.9 Cante (New Celestial)

**Files:** `src/core/celestials/cante/` (343 lines total)
**Data:** `src/core/secret-formula/celestials/cante.js` (197 lines)
**Symbol:** ξ

Cante is a post-Meta celestial unlocked from Ra (`Ra.unlocks.canteUnlock`).
It introduces a **self-replicating dimension** system with two internal
reset layers.

**Nerf profile:** AD nerfed to 1e-25; ID/TD not nerfed.

**Core loop:**
1. Unlock Replicator tiers (gated by Metas, Artificial Matter, Null unlock,
   Chaotic Matter thresholds)
2. Replicators self-replicate: each tier's `tick()` makes it produce more
   of itself (not a lower tier — this is unique among dimension-like systems)
3. **Reforge:** when any tier exceeds `Number.MAX_VALUE`, reset it for
   **Artificial Matter** (ArtM) with heavy softcaps
4. Buy upgrades with Artificial Matter
5. **Purge:** at `1e75000` ArtM, reset Replicators + early upgrades for
   **Chaotic Matter** (CM)
6. Buy advanced upgrades with Chaotic Matter

**Cante Replicators (10 tiers):**
- Use `HyperExponentialCostScaling` for costs
- Purchased with Artificial Matter
- Self-replicating production (unique mechanic)
- Multiplier from Chaotic Matter, peak gamespeed, tier power, upgrades
- Multiple softcap breakpoints at 1.79e308, 1e1000, 1e1000000, 1e1E20,
  etc.

**Cante Upgrades (20):**
- IDs 0-8: bought with Artificial Matter (cost scaling reduction, ArtM-based
  multipliers, cross-tier effects, peak gamespeed effects)
- IDs 9-19: bought with Chaotic Matter (weaker softcaps, higher-tier
  unlocks, autobuyers, passive generation, "upgrades no longer reset on
  Purge")

### 3.10 Null (New Celestial)

**Files:** `src/core/celestials/null/` (474 lines total)
**Data:** `src/core/secret-formula/celestials/null.js` (165 lines)
**Symbol:** Θ

Null is another post-Meta celestial unlocked from Ra
(`Ra.unlocks.nullUnlock`). It introduces a **ring/cycle** dimension system
with two nested reset layers (Parallax and Corruption).

**Nerf profile:** AD/ID/TD all nerfed to 1e-35 (harshest of all celestials).

**Extra feature:** A **passcode puzzle** (`NullData.passcode`) using SHA-256
hashes, with the correct code changing based on Parallax/Corruption state.

**Null Cycles (16 tiers):**
Unlike normal dimensions, Null Cycles form a **ring/loop**:
- Tier 1 → produces tier 2
- Tier 2 → produces tier 3
- ...
- Highest unlocked tier → loops back to tier 1 AND generates **Abyssal
  Matter**
- In corrupt mode: also generates **Corrupt Matter**

Tier unlocking depends on Parallax count (non-corrupt) or is automatic
(corrupt mode — all 16 tiers). Purchased with Abyssal Matter using
`HyperExponentialCostScaling`.

**Null Parallax (1st reset layer):**
- Requirement: Cycle 1 amount ≥ threshold (1e14, 1e22, 1e30, 1e48, 1e70,
  1e100, then scaling)
- Reset: +1 Parallax, reset Abyssal Matter + Cycles + non-corrupt upgrades
- Reward: permanent Cycle multiplier (`2^parallaxes`) and power, plus more
  unlocked tiers

**Null Corruption (2nd reset layer):**
- Requirement: Parallax count ≥ threshold (14, 16, 18, 20, 25, 40, 50, 80,
  then quadratic)
- Reset: +1 Corrupt, reset Parallax + Abyssal Matter + Cycles +
  non-corrupt upgrades
- Reward: permanent multiplier (`4^corrupts`) and power, unlock
  corrupt-only upgrades and all 16 cycle tiers

**Null Upgrades (20):**
- IDs 1-12: bought with Abyssal Matter (cycle multipliers from unlock count,
  first/last cycle amounts, powers, weaker decay)
- IDs 13-20: bought with Corrupt Matter (autobuyers, bulk Parallax, final
  upgrade "Break a chain...")

### 3.11 Glitch Glyph Type

**File:** `src/core/secret-formula/reality/core-glyph-info.js` (490 lines)

A new glyph type added alongside the existing types:

- **Type ID:** `"glitch"`
- **Effects:**
  - `glitchChaosPow` — raises Chaos Dimension multipliers
  - `glitchADCelPow` — raises AD power inside celestial realities
- **Properties:** non-generated (manually created), customizable after
  creation, max 1 equipped
- **Sacrifice:** provides a Chaos Dimension multiplier
- **Alchemy:** has its own alchemy resource

The mod also consolidates `glyph-types.js` and `glyph-sacrifices.js` from
the base game into a single `core-glyph-info.js` that centralizes type
lists, rarity tables, sacrifice info, alchemy mappings, symbols, colors,
and generation/customization rules.

---

## 4. Modifications to Existing Systems

The vis mod makes pervasive changes to existing systems by adding Glitch,
Meta, Challenger, and Rift checks throughout:

### Game Loop (+134 lines)
- New production ticks: `ChaosDimensions.tick()`, `CanteReplicators.tick()`,
  `NullCycles.tick()`
- Real-time Rift Force generation
- Per-tick passive Artificial Matter / Chaotic Matter
- Passive V metaTheorems
- Meta-based IP/EP powers
- Game speed now boosted by Glitch Rift alpha, V gamespeed power, Meta
  Fabricator 3
- Glitch run and V extreme run clear/restore logic
- `gainedMetaRelays()` and `gainedMetas()` reward functions

### Antimatter Dimensions (+170 lines)
- Glitch challenge bonuses added directly into AD multipliers
- Glitch Rift alpha effects on 1st/8th AD
- Meta Fabricator 2 AD/ID/TD power
- New `applyNDNerfs()` handles Effarig/V/Glitch/V-extreme/Pelle-joined
  interactions
- AD buying now has "you will fail Glitch challenge" guards
- Enslaved dim-8 restriction can be bypassed by Glitch augment
- AD cap lifted to `PREMETAMAX ^ MetaFabricatorUpgrade(23)`

### Pelle (+137 lines)
- Disabled-mechanic table now driven by Challenger upgrades
- New "joined/conjoined" Pelle state (post-Challenger/Remake)
- 6th Pelle strike/rift: glitch
- Armageddon can branch into "destroy Pelle" flow → Challenger Essence +
  `pelleResets`
- Pelle loop runs while doomed OR joined
- Rifts can auto-activate/fill faster from Challenger upgrades
- Special doomed glyph effects expanded (Effarig/Reality/Cursed)

### Reality (+91 lines)
- Manual Reality warns if it would fail unfinished Glitch challenges
- Simulated realities include perk bonuses (+2 glyph choices)
- Reality rewards include extra realities/perk points from Glitch systems
- Glitch reality completion shows its own modal
- Glyph auto-leveling supports reality/glitch glyph auto-cap
- Finishing Reality correctly leaves/restores Glitch/V-extreme runs

### Ra (+61 lines in ra.js, +217 lines in ra.js data)
- **3 new Ra pets:** Glitch ("glitchyfishys"), Cante, Null
- Each pet provides memory-based unlocks feeding into respective celestials
- New unlocks include Meta boosts, celestial unlocks, antimatter scaling

### V (+47 lines)
- New **V Extreme mode** (`v.runExtreme`) — an even harder V reality with
  IP ^0.01
- **Meta Theorems** — passive theorem generation

### Dark Matter Dimensions (replaced: 278 → 331 lines)
- Full Decimal-first rewrite
- New `ExponentialCostScaling` for interval/DM/DE
- Bulk ascension instead of +1 at a time
- Powers scale with Glitch Reality Upgrade(12) (^1.05)
- Dark Energy no longer shut off when Pelle is Doomed
- More cached/Decimal-aware tick loop

### Math (+361 lines)
- Decimal polynomial solvers: quadratic/cubic/depressed cubic
- `Decimal.cbrt`, `Decimal.powNeg`
- Hybrid/infinite-cost helpers: `findFirstInfiniteCostPurchase`,
  `getHybridCostScaling`
- New scaling classes: `LinearCostScaling`, `ExponentialCostScaling`,
  `HyperExponentialCostScaling`
- Decimal bulk-buy helpers

### Other Notable Changes
- `dimboost.js` (+85): Glitch/Meta effects on boost power
- `time-dimension.js` (+77): Glitch/Meta effects
- `eternity.js` (+67): Meta integration
- `dilation.js` (+62): Extended dilation mechanics
- `galaxy.js` (+52): Meta/Glitch effects on galaxy strength
- `replicanti.js` (+99): Extended mechanics
- `black-hole.js` (+68): Extended mechanics
- `format.js` (+95): New number format helpers
- `constants.js` (-59): Some constants removed/reorganized
- `glyph-effects.js` (-57): Consolidated into `core-glyph-info.js`

---

## 5. New Currencies

| Currency | Type | Source | Use |
|----------|------|--------|-----|
| `riftForce` | Decimal | Glitch Reality (RF formula) | Glitch Reality/Speed Upgrades |
| `chaosCores` | Decimal | Chaos Dimension 1 production | Chaos Dimension purchases |
| `challengersEssence` | Decimal | Pelle destruction (Armageddon) | Challenger/Hard Challenger Upgrades |
| `metaRelays` | Decimal | Meta prestige | Meta Fabricator Upgrades |
| `metas` | Decimal | Meta prestige count | Milestone thresholds, multipliers |
| `artificialMatter` | Decimal | Cante Replicator Reforge | Cante Replicator purchases, upgrades |
| `chaosMatter` | Decimal | Cante Replicator Purge | Cante upgrades (advanced) |
| `abyssalMatter` | Decimal | Null Cycle production | Null Cycle purchases, upgrades |
| `corruptMatter` | Decimal | Null Cycle (corrupt mode) | Null upgrades (advanced) |

Total: **9 new currencies** (vs base game's ~15 and endgame mod's 8).

---

## 6. New Player State

The vis mod adds approximately **313 lines** to `player.js`. Key new state
subtrees:

```javascript
player = {
  // ... existing state ...
  dimensions: {
    // ... existing ...
    chaos: Array(12).map(() => ({ bought, amount })),
  },
  records: {
    thisMeta: { time, realTime, trueTime, maxAM, MR, bestMRmin, ... },
    bestMeta: { time, realTime, trueTime, MRmin },
    recentMetas: [...],
  },
  reality: {
    glyphs: {
      sac: { /* existing + */ glitch: DC.D0 },
      createdGlitchGlyph: false,
    },
  },
  celestials: {
    // ... existing ...
    v: { runExtreme, metaTheorems, wantsHard, wantsExtreme },
    ra: {
      pets: {
        // ... existing + glitchyfishys, cante, null ...
      },
    },
    pelle: { /* + joined state, rifts.glitch */ },
    glitch: {
      run: false,
      augment: { effectbits: 0 },
      riftForce: DC.D0,
      chaosCores: DC.D0,
      plynia: DC.D0,
      rifts: { alpha, beta, delta, gamma, epsilon },
      upgrades: {
        unlockbits, broughtbits, speedunlockbits,
        speedbroughtbits, rebuyable: { 1-4 },
      },
    },
    cante: {
      run: false,
      replicators: Array(10),
      replicatorUnlockbits,
      artificialMatter: DC.D0,
      chaoticMatter: DC.D0,
      upgradeBits, purges,
    },
    null: {
      run: false,
      cycle: Array(16),
      isUnlocked,
      abyssalMatter: DC.D0,
      corruptMatter: DC.D0,
      parallax: DC.D0,
      corrupt: DC.D0,
      upgradeBits, corruptUpgradeBits,
    },
  },
  glitch: {
    preinfinity: { upgradebits },
    breakinfinity: { upgradebits },
    eternity: { upgradebits },
    reality: { upgradebits },
    challengersEssence: DC.D0,
    challengerUpgradebits: 0,
    hardChallengerUpgradebits: 0,
  },
  meta: {
    metaRelays: DC.D0,
    metas: DC.D0,
    upgrades: { metaBits, rebuyable: { 1-25 } },
  },
  pelleResets: 0,
};
```

Notable type changes from base game:
- `player.infinityRebuyables`: `number[]` → `Decimal[]`
- Various values throughout use `Decimal` more pervasively

---

## 7. Architecture Assessment

### Positive Aspects

1. **Follows existing patterns:** Chaos Dimensions use `DimensionState`,
   Cante Replicators use `DimensionState`, Null Cycles use `DimensionState`,
   upgrades use `BitPurchasableMechanicState` / `RebuyableMechanicState`.
   No fundamentally new architectural patterns.

2. **Data-driven:** All new configs live in `secret-formula/` following the
   established pattern (`secret-formula/glitch/`, `secret-formula/meta/`,
   `secret-formula/celestials/{cante,null,glitchrift,...}`).

3. **Clean celestial separation:** Each new celestial (Glitch, Cante, Null)
   has its own directory with self-contained logic.

4. **Interesting novel mechanics:** Self-replicating Cante Replicators and
   the ring/loop Null Cycles introduce genuinely different dimension-like
   systems beyond the standard production chain.

### Concerns

1. **Code quality is lower than the base game:** Many spelling errors in
   variable names (`broughtbits`, `Inforcement`, `Dialated`, `reduse`),
   commented-out code left in, side-effectful getters
   (`Glitch.isUnlocked`), and inconsistent formatting. The mod has a
   "work-in-progress" feel.

2. **Heavy cross-system coupling:** Glitch challenge effects are wired
   directly into AD/IP/EP multiplier calculations, Challenger upgrades
   modify Pelle's disabled-mechanic table, Meta resets must know about
   every system. This makes the dependency graph complex.

3. **Placeholder systems:** `metaStability.js` defines 4 entries with
   `[REDACTED]` descriptions and no real runtime usage — it appears to be
   planned but unimplemented scaffolding. Similarly,
   `player.celestials.glitch.rifts.epsilon` exists in save state but has
   no corresponding rift definition.

4. **Removed features:** Cloud saving, in-app payments, and the celestial
   navigation map have been removed. The celestial map removal is
   significant — the base game uses it to visualize celestial progression.

5. **Scattered conditionals:** Like the endgame mod, effects are checked
   via `GlitchSpeedUpgrade(N).isBought`,
   `MetaFabricatorUpgrade(N).isBought`, `ChallengerUpgrade(N).isBought`
   scattered across many existing files, making it harder to trace the
   full effect of any single upgrade.

---

## 8. Porting Considerations for Rust

### Comparison with Endgame Mod

The vis mod and endgame mod are roughly similar in scope (~8.5k vs ~11.1k
new logic lines), but they are **mutually exclusive** — they modify the
same base game files in incompatible ways. A Rust implementation would need
to choose one (or implement both behind feature flags).

| Aspect | Endgame Mod | Vis Mod |
|--------|-------------|---------|
| New prestige layer | 1 (Endgame) | 1 (Meta) |
| New celestials | 1 (Alpha) | 3 (Glitch, Cante, Null) |
| New dimension types | 1 (Celestial, 8 tiers) | 1 (Chaos, 12 tiers) |
| New currencies | 8 | 9 |
| Challenge systems | 0 | 34 challenge-achievements + 30 upgrades |
| Internal reset layers | 0 | 3 (Plynia, Parallax, Corruption) |
| Mastery tree | Yes (50+ nodes) | No (flat upgrades only) |
| Novel dimension mechanics | No | Yes (self-replication, ring/loop) |
| Code quality | High | Moderate |

### Additional Challenges Beyond Base Game

1. **Novel dimension production models:** The standard downward chain
   (tier N produces tier N-1) is insufficient. Cante Replicators use
   self-replication and Null Cycles use a ring topology. In Rust, the
   dimension production trait/enum needs to support these variants.

2. **Complex reset graph:** Meta resets ~300 lines of state with ~30+
   conditional branches. Combined with Plynia, Parallax, and Corruption
   resets inside celestials, the reset dependency graph is intricate.

3. **Cross-system multiplier chains:** Glitch challenge effects feed into
   AD/IP/EP formulas, which affect Rift Force, which buys upgrades that
   affect Chaos Dimensions, which produce Chaos Cores that boost
   Challenger Essence, which buys upgrades that modify Pelle. Tracing
   multiplier sources is complex.

4. **New cost scaling classes:** `HyperExponentialCostScaling` and
   `LinearCostScaling` are new to the vis mod and used by Cante
   Replicators and Null Cycles. These need to be implemented alongside
   the existing `ExponentialCostScaling`.

### No New Architectural Paradigms

As with the endgame mod, the vis mod introduces **no fundamentally new
patterns**. Every new system uses:
- Config in `secret-formula/` → `GameMechanicState` subclass → accessor
- Currencies follow the `Currency` abstraction
- Dimensions follow `DimensionState` (with variant production models)
- Upgrades follow `BitPurchasableMechanicState` / `RebuyableMechanicState`

A Rust implementation handling the base game's patterns would accommodate
the vis mod content with additional instances of the same traits/structs,
plus production model variants for dimensions.
