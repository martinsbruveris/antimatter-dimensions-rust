---
status: Implemented
---

# How To Play (H2P) Modal — Design Notes

**Date:** 2026-06-27
**Focus:** How the original game's "How To Play" modal is structured, what
gates control entry visibility, how its text is formatted, and how the Rust
reimplementation currently mirrors (and diverges from) it.

## Table of Contents

1. [Overview](#1-overview)
2. [Original Architecture](#2-original-architecture)
3. [Entry Visibility Gates](#3-entry-visibility-gates)
   - 3.1 [Per-entry `isUnlocked()`](#31-per-entry-isunlocked)
   - 3.2 [Inline gating inside `info()` bodies](#32-inline-gating-inside-info-bodies)
   - 3.3 [Default-selection (`tab`, forced tab)](#33-default-selection-tab-forced-tab)
4. [Number & Text Formatting](#4-number--text-formatting)
5. [Search & Relevance](#5-search--relevance)
6. [Reimplementation Status](#6-reimplementation-status)
7. [Porting Checklist](#7-porting-checklist)
8. [Recommendations](#8-recommendations)

---

## 1. Overview

In the original game the "How To Play" (H2P) modal is the in-game manual: a
full-screen popup with a searchable list of entries on the left and the
selected entry's body on the right. Entries appear progressively as the
player unlocks the mechanics they describe, and their text is generated at
runtime so the numbers shown always match the player's current notation and
settings.

The reimplementation ports the early-game subset of this modal. This doc
records what the original does so the port can be extended faithfully.

**Key source files (original):**

- `src/core/secret-formula/h2p.js` — the entry data (`GameDatabase.h2p.tabs`)
  and the search function.
- `src/components/modals/H2PModal.vue` — the two-pane modal UI.

**Key source files (reimplementation):**

- `crates/ad-gui/frontend/src/data/h2p.js` — ported entries (static HTML).
- `crates/ad-gui/frontend/src/components/H2PModal.vue` — the modal UI.

---

## 2. Original Architecture

The modal is a single component (`H2PModal.vue`) driven entirely by the
`GameDatabase.h2p.tabs` array. Each entry is an object:

| Field        | Purpose                                                              |
|--------------|---------------------------------------------------------------------|
| `name`       | Internal name; also the display title (unless `alias` is set).      |
| `alias`      | Optional display name for the tab list.                             |
| `id`         | Unique id, generated at load (used by search relevance + sorting).  |
| `info()`     | Function returning the entry's HTML body (rendered via `v-html`).   |
| `isUnlocked()` | Boolean gate — whether the entry is visible/searchable at all.    |
| `tags`       | Keywords the search matches in addition to the name.               |
| `tab`        | `"tabKey/subtabKey"` (or `"tabKey"`) the entry is the default for.  |

The component:

1. Filters `tabs` by `search(query)` **and** `tab.isUnlocked()`
   (`matchingTabs`).
2. On open, selects the entry whose `tab` matches the player's current
   tab/subtab (see §3.3).
3. Renders the active entry's `name` as a title and `info()` as the body.

There are ~47 entries in the full game, spanning every prestige layer from
Antimatter Dimensions to Pelle.

---

## 3. Entry Visibility Gates

Visibility is controlled at three levels.

### 3.1 Per-entry `isUnlocked()`

This is the primary gate: an entry is listed only if `isUnlocked()` returns
true. Grouped by the underlying flag:

**Always visible — `() => true`**
This Modal, Your savefile, Customization, Offline Progress, Effect Stacking,
Common Abbreviations, Antimatter Dimensions, Dimension Boosts, Antimatter
Galaxies, Achievements, Autobuyers.

**Early-game mechanic flags**

| Entry                 | Gate                  |
|-----------------------|-----------------------|
| Tickspeed             | `Tickspeed.isUnlocked` |
| Dimensional Sacrifice | `Sacrifice.isVisible` (after 5th Dimension Boost) |

**Infinity layer — `PlayerProgress.infinityUnlocked()`**
Infinity, Normal Challenges.

**Break Infinity — `Autobuyer.bigCrunch.hasMaxedInterval || PlayerProgress.eternityUnlocked()`**
Break Infinity, Infinity Dimensions, Infinity Challenges.

**Replicanti — `Replicanti.areUnlocked || PlayerProgress.eternityUnlocked()`**

**Eternity layer — `PlayerProgress.eternityUnlocked()`**
Eternity, Eternity Milestones, Time Dimensions, Time Studies, Eternity
Challenges.

**Dilation — `DilationTimeStudyState.studies[1].isBought || PlayerProgress.realityUnlocked()`**

**Reality layer — `PlayerProgress.realityUnlocked() || TimeStudy.reality.isBought`**
Reality, Glyphs, Perks.

**Automator — `Player.automatorUnlocked`**
Automator Overview, Automator Technical Details.

**Black Hole — `player.blackHole[0].unlocked`**

**Celestials (each its own unlock flag)**

| Entry                              | Gate                                          |
|------------------------------------|-----------------------------------------------|
| Celestials / Teresa                | `Teresa.isUnlocked`                           |
| Effarig                            | `TeresaUnlocks.effarig.canBeApplied`          |
| Advanced Glyph Mechanics           | `EffarigUnlock.adjuster.isUnlocked`           |
| The Nameless Ones                  | `EffarigUnlock.eternity.isUnlocked`           |
| Tesseracts                         | `Enslaved.isCompleted`                        |
| V                                  | `Achievement(151).isUnlocked`                 |
| Ra                                 | `VUnlocks.raUnlock.isUnlocked`                |
| Glyph Alchemy Resources/Reactions  | `Ra.unlocks.unlockGlyphAlchemy.canBeApplied`  |
| Imaginary Machines                 | `MachineHandler.isIMUnlocked`                 |
| Lai'tela                           | `Laitela.isUnlocked`                          |
| Continuum                          | `ImaginaryUpgrade(15).isBought`               |
| Singularities                      | `Laitela.isUnlocked`                          |
| Pelle                              | `Pelle.isUnlocked`                            |
| Pelle Strikes                      | `PelleStrikes.infinity.hasStrike`             |
| The Galaxy Generator               | `Pelle.hasGalaxyGenerator`                    |

Note these are progress *flags*, not raw resource amounts — they are the same
unlock predicates the game uses to reveal the corresponding tabs/mechanics, so
an H2P entry becomes visible at the same moment its feature does.

### 3.2 Inline gating inside `info()` bodies

Even an always-visible entry can grow as the player progresses, because
`info()` is a function that interpolates conditionals. The clearest example is
**Common Abbreviations**, whose body appends rows guarded by:

- `PlayerProgress.infinityUnlocked()` → IP, NC, IC rows
- `InfinityDimension(1).isUnlocked || PlayerProgress.eternityUnlocked()` → ID
- `PlayerProgress.replicantiUnlocked()` → RG
- `PlayerProgress.eternityUnlocked()` → EP, TT, TD, EC
- `PlayerProgress.dilationUnlocked()` → TP, DT, TG
- `PlayerProgress.realityUnlocked()` → RM, AP, BH
- `MachineHandler.isIMUnlocked` → iM
- `Laitela.isUnlocked` → DM, DE

So an entry's *presence* is governed by `isUnlocked()`, but its *contents* can
be further gated inline.

### 3.3 Default-selection (`tab`, forced tab)

These do not control visibility, only which entry is focused when the modal
opens:

- **`tab` field** — `"tabKey/subtabKey"`. On open, the modal selects the first
  entry whose `tab` matches the player's current tab+subtab, else the current
  tab, else falls back to entry 0 ("This Modal").
- **`ui.view.h2pForcedTab`** — an override another UI element can set to force
  a specific entry on open (cleared after use).
- **`Tutorial.emphasizeH2P()`** — while the first-time H2P tooltip is active,
  entry 0 is forced regardless of current tab. Also, before the player's first
  Dimension Boost the modal always opens on entry 0; only afterward does it
  auto-select by current tab.

---

## 4. Number & Text Formatting

Every entry body is built by a function (`info: () => \`...\``) rather than a
static string, specifically so embedded numbers render through the game's
notation system. The helpers used in the early-game entries:

| Helper                        | Meaning                                              | Example render |
|-------------------------------|------------------------------------------------------|----------------|
| `formatInt(n)`                | Integer in current notation                          | `10`           |
| `format(x, places, under1k)`  | General number, mantissa precision                   | `0.02`         |
| `formatX(x, p, u)`            | Multiplier, prefixed with `×`                         | `×2`, `×1.125` |
| `formatPercents(x, p)`        | Percentage                                           | `100%`, `0.2%` |
| `formatPostBreak(x, p)`       | Number allowed to exceed `Number.MAX_VALUE`          | `1.797693e308` |

Consequences:

- Numbers in H2P **track the player's chosen notation** (Scientific,
  Engineering, Letters, …) and precision. The text is not authored with literal
  numerals.
- Some entries pull live constants — e.g. the Antimatter Dimensions entry lists
  base prices/cost multipliers via
  `Array.range(1, 8).map(t => format(AntimatterDimension(t)._baseCost, 2, 2))`,
  so the manual stays correct if the constants change.
- Values that depend on upgrades resolve live too (e.g. `Galaxy.remoteStart`
  defaults to 800 but a Reality upgrade can change it; the entry shows the
  current value).

See [`2026-06-25-number-formatting.md`](2026-06-25-number-formatting.md) for the
notation system itself.

---

## 5. Search & Relevance

The left-hand list is filtered by `h2p.search(query)`:

- Builds a search index from each entry's name + `tags`.
- Scores entries by a typo-tolerant keyboard-distance metric (a weighted
  Levenshtein variant where substitution cost depends on physical key
  distance across several keyboard layouts), producing a `relevance` value
  (lower = closer match).
- Empty query → all entries at a neutral relevance.
- Results are sorted by id, then by relevance, so close matches float to the
  top; the modal also visually separates "relevant" from "irrelevant" results
  via a threshold (`topThreshold`) and a divider border.
- Crucially, the modal then **also** filters by `tab.isUnlocked()`, so search
  can never surface a locked entry.

This is a lot of machinery for fuzzy matching; it is not essential to a
faithful early-game port (see §6).

---

## 6. Reimplementation Status

Current port (`crates/ad-gui/frontend/`):

- **Entries** (`data/h2p.js`): This Modal, Common Abbreviations, Antimatter
  Dimensions, Tickspeed, Dimension Boosts, Antimatter Galaxies, Dimensional
  Sacrifice, Achievements, Infinity, Autobuyers — the subset whose mechanics
  exist here. The Autobuyers entry is trimmed to the autobuyers this
  reimplementation has (AD + Tickspeed autobuyers, the global pause/resume and
  enable/disable-all controls); the original's paragraphs on the Dimension
  Boost / Galaxy / Sacrifice / crunch autobuyers, Dynamic Amount, and IP-on-
  crunch are omitted until those exist.
- **Numbers are pre-resolved.** The original's format helpers were resolved to
  the values the original renders under default settings (`formatInt(10)` →
  `10`, `formatX(2)` → `×2`, `formatPercents(1)` → `100%`,
  `formatPostBreak(MAX_VALUE,6)` → `1.797693e308`). Base-price/cost-multiplier
  lists use this repo's constants (`ad-core` `AD_BASE_COSTS` /
  `AD_COST_MULTIPLIERS`).
- **`isUnlocked` gating is wired for implemented mechanics** (gate #1). Each
  entry has an `isUnlocked(flags)` predicate; the modal filters the tab list by
  it. The flags come from the engine snapshot:
  - `tickspeedUnlocked` ← `tickspeed_unlocked` (JS `Tickspeed.isUnlocked`,
    i.e. a 2nd Antimatter Dimension has been bought) → gates **Tickspeed**.
  - `sacrificeUnlocked` ← `sacrifice_unlocked` (≥ 5 Dimension Boosts) → gates
    **Dimensional Sacrifice**.
  - `infinityUnlocked` ← `infinity_unlocked` (has performed ≥ 1 Big Crunch;
    persists across crunches) → gates **Infinity**.

  Every other entry is `() => true` (always visible), since its mechanic is
  reachable from the start of a run.
- **Inline body gating is wired** (gate #2). An entry's `info` may be a plain
  string or a `(flags) => string` function. **Common Abbreviations** uses the
  function form to show the `IP` row only once `infinityUnlocked` is true,
  mirroring the original's inline `PlayerProgress.infinityUnlocked()` guard.
- **Default-selection is ported**: the modal picks the *unlocked* entry whose
  `tab` matches the current tab/subtab, falling back to the first unlocked
  entry ("This Modal"). The forced-tab and tutorial-emphasis behaviours are
  **not** ported.
- **Search is simplified** to a plain substring filter over name + tags (the
  fuzzy keyboard-distance relevance is dropped). It runs over the unlocked
  entries only, so it can never surface a locked entry — matching the original.

### Supporting engine changes

Gate #1 required two new pieces of snapshot state (`crates/ad-core`):

- `GameState::tickspeed_unlocked()` — derived (`dimensions[1].bought > 0`).
- `GameState::infinity_unlocked` — a new persistent `bool` field, `false` at
  start, set `true` in `big_crunch()` and **not** reset by it (like
  `total_antimatter`). Carries `#[serde(default)]` for save compatibility.

Both are surfaced on the GUI snapshot (`GameView`) as `tickspeed_unlocked` /
`infinity_unlocked`.

### Divergences at a glance

| Aspect              | Original                                  | Reimplementation                          |
|---------------------|-------------------------------------------|-------------------------------------------|
| Entry bodies        | `info()` functions, runtime-formatted     | String or `(flags) => string`, pre-resolved numbers |
| Notation awareness  | Tracks player notation                    | Fixed (default notation)                  |
| `isUnlocked` gating | Per-entry flags + inline body conditions  | Wired for implemented mechanics (Tickspeed, Sacrifice, Infinity) |
| Default selection   | `tab` + forced tab + tutorial emphasis    | `tab` match only (over unlocked entries)  |
| Search              | Typo-tolerant relevance ranking           | Substring filter (over unlocked entries)  |

---

## 7. Porting Checklist

Tracks porting each original H2P entry into `data/h2p.js`. `[x]` = ported,
`[ ]` = not yet. Sub-items list the original `isUnlocked()` predicate for
entries with a conditional unlock; entries with no sub-item are always visible
(`() => true`). Order follows the original `GameDatabase.h2p.tabs`. We are
deliberately porting these incrementally as the underlying mechanics land —
an entry should only be ported (and its unlock wired) once its feature exists.

**Early game**

- [x] This Modal
- [ ] Your savefile
- [ ] Customization
- [ ] Offline Progress
- [ ] Effect Stacking
- [x] Common Abbreviations
  - Body grows inline as resources unlock. Each abbreviation row is its own
    conditional in the original `info()`; track each row here:
    - [x] AM / AD / AG — always shown (`() => true`)
    - [x] IP (Infinity Point) — `PlayerProgress.infinityUnlocked()` (wired as `infinityUnlocked`)
    - [ ] NC (Normal Challenge) — `PlayerProgress.infinityUnlocked()`
    - [ ] IC (Infinity Challenge) — `PlayerProgress.infinityUnlocked()`
    - [ ] ID (Infinity Dimension) — `InfinityDimension(1).isUnlocked || PlayerProgress.eternityUnlocked()`
    - [ ] RG (Replicanti Galaxy) — `PlayerProgress.replicantiUnlocked()`
    - [ ] EP (Eternity Point) — `PlayerProgress.eternityUnlocked()`
    - [ ] TT (Time Theorem) — `PlayerProgress.eternityUnlocked()`
    - [ ] TD (Time Dimension) — `PlayerProgress.eternityUnlocked()`
    - [ ] EC (Eternity Challenge) — `PlayerProgress.eternityUnlocked()`
    - [ ] TP (Tachyon Particle) — `PlayerProgress.dilationUnlocked()`
    - [ ] DT (Dilated Time) — `PlayerProgress.dilationUnlocked()`
    - [ ] TG (Tachyon Galaxy) — `PlayerProgress.dilationUnlocked()`
    - [ ] RM (Reality Machine) — `PlayerProgress.realityUnlocked()`
    - [ ] AP (Automator Point) — `PlayerProgress.realityUnlocked()`
    - [ ] BH (Black Hole) — `PlayerProgress.realityUnlocked()`
    - [ ] iM (Imaginary Machine) — `MachineHandler.isIMUnlocked`
    - [ ] DM (Dark Matter) — `Laitela.isUnlocked`
    - [ ] DE (Dark Energy) — `Laitela.isUnlocked`
- [x] Antimatter Dimensions
- [x] Tickspeed
  - Unlock: `Tickspeed.isUnlocked` (a 2nd Antimatter Dimension bought) — wired
    as `tickspeed_unlocked`.
- [x] Dimension Boosts
- [x] Antimatter Galaxies
- [x] Dimensional Sacrifice
  - Unlock: `Sacrifice.isVisible` (≥ 5 Dimension Boosts) — wired as
    `sacrifice_unlocked`.
- [x] Achievements

**Infinity**

- [x] Infinity
  - Unlock: `PlayerProgress.infinityUnlocked()` (≥ 1 Big Crunch) — wired as
    `infinity_unlocked`.
- [ ] Normal Challenges
  - Unlock: `PlayerProgress.infinityUnlocked()`
- [x] Autobuyers — _ported, trimmed_. The entry itself is always visible
  (`() => true`), but its body describes per-autobuyer/feature sections that
  the original only includes once that autobuyer or upgrade exists. Track each
  section (each needs the corresponding mechanic before its paragraph is added
  back):
    - [x] Intro + "unlocked based on total antimatter" note
    - [x] Autobuyer Interval (incl. "challenge needed before interval upgrade")
    - [x] AD Autobuyer Bulk Buy (doubles past 100 ms minimum interval)
    - [x] AD Autobuyer Buy Quantity (single / until 10)
    - [x] Tickspeed Autobuyer Buy Quantity (single, or max after Tickspeed Challenge C9)
    - [x] Pause/Resume Autobuyers (master switch)
    - [x] Enable/Disable All Autobuyers
    - [x] Hotkey `A` + `Alt`+hotkey toggle
    - [ ] Automatic Dimension Boost Customization — needs the Dimension Boost autobuyer
    - [ ] Max Galaxies — needs the Antimatter Galaxy autobuyer
    - [ ] IP on crunch — needs Break Infinity (the Big Crunch autobuyer's IP threshold)
    - [ ] Sacrifice Autobuyer — needs the Dimensional Sacrifice autobuyer
    - [ ] Dynamic Amount — needs upgraded prestige autobuyers (dynamic prestige threshold mode)
- [ ] Break Infinity
  - Unlock: `Autobuyer.bigCrunch.hasMaxedInterval || PlayerProgress.eternityUnlocked()`
- [ ] Infinity Dimensions
  - Unlock: `Autobuyer.bigCrunch.hasMaxedInterval || PlayerProgress.eternityUnlocked()`
- [ ] Infinity Challenges
  - Unlock: `Autobuyer.bigCrunch.hasMaxedInterval || PlayerProgress.eternityUnlocked()`
- [ ] Replicanti
  - Unlock: `Replicanti.areUnlocked || PlayerProgress.eternityUnlocked()`

**Eternity**

- [ ] Eternity
  - Unlock: `PlayerProgress.eternityUnlocked()`
- [ ] Eternity Milestones
  - Unlock: `PlayerProgress.eternityUnlocked()`
- [ ] Time Dimensions
  - Unlock: `PlayerProgress.eternityUnlocked()`
- [ ] Time Studies
  - Unlock: `PlayerProgress.eternityUnlocked()`
- [ ] Eternity Challenges
  - Unlock: `PlayerProgress.eternityUnlocked()`
- [ ] Time Dilation
  - Unlock: `DilationTimeStudyState.studies[1].isBought || PlayerProgress.realityUnlocked()`

**Reality**

- [ ] Reality
  - Unlock: `PlayerProgress.realityUnlocked() || TimeStudy.reality.isBought`
- [ ] Glyphs
  - Unlock: `PlayerProgress.realityUnlocked() || TimeStudy.reality.isBought`
- [ ] Perks
  - Unlock: `PlayerProgress.realityUnlocked() || TimeStudy.reality.isBought`
- [ ] Automator Overview
  - Unlock: `Player.automatorUnlocked`
- [ ] Automator Technical Details
  - Unlock: `Player.automatorUnlocked`
- [ ] Black Hole
  - Unlock: `player.blackHole[0].unlocked`

**Celestials**

- [ ] Celestials
  - Unlock: `Teresa.isUnlocked`
- [ ] Teresa, Celestial of Reality
  - Unlock: `Teresa.isUnlocked`
- [ ] Effarig, Celestial of Ancient Relics
  - Unlock: `TeresaUnlocks.effarig.canBeApplied`
- [ ] Advanced Glyph Mechanics
  - Unlock: `EffarigUnlock.adjuster.isUnlocked`
- [ ] The Nameless Ones, Celestial of Time
  - Unlock: `EffarigUnlock.eternity.isUnlocked`
- [ ] Tesseracts
  - Unlock: `Enslaved.isCompleted`
- [ ] V, Celestial of Achievements
  - Unlock: `Achievement(151).isUnlocked`
- [ ] Ra, Celestial of the Forgotten
  - Unlock: `VUnlocks.raUnlock.isUnlocked`
- [ ] Glyph Alchemy Resources
  - Unlock: `Ra.unlocks.unlockGlyphAlchemy.canBeApplied`
- [ ] Glyph Alchemy Reactions
  - Unlock: `Ra.unlocks.unlockGlyphAlchemy.canBeApplied`
- [ ] Imaginary Machines
  - Unlock: `MachineHandler.isIMUnlocked`
- [ ] Lai'tela, Celestial of Dimensions
  - Unlock: `Laitela.isUnlocked`
- [ ] Continuum
  - Unlock: `ImaginaryUpgrade(15).isBought`
- [ ] Singularities
  - Unlock: `Laitela.isUnlocked`
- [ ] Pelle, Celestial of Antimatter
  - Unlock: `Pelle.isUnlocked`
- [ ] Pelle Strikes
  - Unlock: `PelleStrikes.infinity.hasStrike`
- [ ] The Galaxy Generator
  - Unlock: `Pelle.hasGalaxyGenerator`

---

## 8. Recommendations

Ordered by value vs. effort for an early-game port:

1. ~~**Add `isUnlocked` to gated entries.**~~ **Done** (2026-06-27). Tickspeed,
   Sacrifice, and Infinity are now gated on `tickspeed_unlocked` /
   `sacrifice_unlocked` / `infinity_unlocked` from the snapshot. The
   Common Abbreviations IP row is gated inline on `infinityUnlocked`.
2. **Decide on notation fidelity.** If/when the frontend gains a number
   formatter (today it receives pre-formatted strings from Rust), the H2P
   bodies could be regenerated through it so they track the player's notation,
   matching the original's runtime `info()` approach. Until then, static text at
   default notation is a reasonable, clearly-documented compromise. (The
   `info` field already supports a `(flags) => string` form, so it can take a
   formatter argument later without a structural change.)
3. **Skip the fuzzy search** unless/until the entry count grows enough to
   warrant it. The substring filter is adequate for ~9 entries.
4. **Defer forced-tab / tutorial emphasis** until a tutorial system exists;
   they have no behavioural effect without one.

When extending to later layers, keep the entry's `isUnlocked` aligned with the
same flag that reveals the feature itself, and keep `tags`/`tab` populated so
the existing search and default-selection keep working.
