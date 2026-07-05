# Exotic Matter Dimensions — Codebase Analysis

**Source:** `../incrementals/exotic-matter-dimensions`
**Companion documents:**
[game overview](2026-07-03-exotic-matter-dimensions-overview.md) ·
[feature decomposition](2026-07-03-exotic-matter-dimensions-features.md)

## Table of Contents

1. [Overview](#1-overview)
2. [Technology Stack & Repository Structure](#2-technology-stack--repository-structure)
3. [Architecture](#3-architecture)
4. [Number System](#4-number-system)
5. [The Stat Pipeline (miscStats / stat DAG)](#5-the-stat-pipeline-miscstats--stat-dag)
6. [Player State & Save System](#6-player-state--save-system)
7. [Game Loop](#7-game-loop)
8. [Core Mechanics Implementation](#8-core-mechanics-implementation)
9. [Data & Configuration Layer](#9-data--configuration-layer)
10. [UI Layer](#10-ui-layer)
11. [Automation](#11-automation)
12. [Key Architectural Patterns](#12-key-architectural-patterns)
13. [Code Quality & Modularity Assessment](#13-code-quality--modularity-assessment)
14. [Good Ideas to Borrow](#14-good-ideas-to-borrow)

---

## 1. Overview

Exotic Matter Dimensions is a single-page browser game written in **plain JavaScript
with no framework, no build system, and no external dependencies** beyond a vendored
fork of `break_eternity.js`. All code is hand-written by one developer (alemaninc) and
loaded as classic `<script>` tags in a fixed order from a single `index.html`.

### Codebase Size

| File | Lines | Role |
|------|------:|------|
| `statbreakdown.js` | 4,392 | Stat pipeline: every derived quantity + its UI breakdown |
| `main.js` | 3,451 | Save schema, all player actions, resets, mechanics glue |
| `break_eternity_alemaninc.js` | 3,154 | Vendored/forked Decimal library |
| `achievements.js` | 2,831 | 142 achievements (9 tiers) + secret achievements |
| `researchtree.js` | 2,733 | 192-node research tree, costs, canvas rendering |
| `study13.js` | 1,183 | Endgame challenge builder (bindings + rewards) |
| `gameloop.js` | 1,054 | `tick()` + per-tab `updateHTML()` |
| `data.js` | 975 | Declarative configs: Studies, luck, prismatic, wormhole upgrades, corruption |
| `newsticker.js` | 720 | News ticker content and machinery |
| `emd_utility.js` | 500 | Game-specific utilities, constants pool, formatters |
| `alemaninc_utility.js` | 338 | Generic utilities (arrays, roman numerals, DOM helper) |
| `howtoplay.js` | 287 | In-game manual (per-feature help texts) |
| `navigation.js` | 274 | Tab/subtab visibility and switching |
| `oldsaves.js` | 222 | Legacy save migration |
| `previousprestige.js` | 158 | Best/last run records |
| `initialize.js` | 99 | Ordered boot sequence with load screen |
| `htmlgenerator.js`, `iconGen.js` | 102 | Static DOM generation from JS |
| `index.html` + aux HTML | ~2,700 | Static shell, changelog, credits, save bank |
| **Total JS** | **~22,500** | |

For comparison, Antimatter Dimensions is ~119k lines; EMD delivers a genuinely deep
incremental in roughly one-fifth of that, at the cost of UI polish and separation of
concerns.

---

## 2. Technology Stack & Repository Structure

**Stack:** Vanilla ES2020+ JavaScript, `<script>`-tag loading, `localStorage`
persistence, one `<canvas>` (research-tree connection lines), CSS custom properties for
theming. No package.json, no bundler, no tests, no linting config.

```
exotic-matter-dimensions/
├── index.html                  # Single page; loads 17 scripts in dependency order
├── alemaninc_utility.js        # Generic helpers (author's personal stdlib)
├── break_eternity_alemaninc.js # Forked Decimal library
├── emd_utility.js              # Constants pool `c`, ops `o`, formatters, popup system
├── data.js                     # Declarative configs (Studies, luck, prismatic, ...)
├── iconGen.js                  # Icon/HTML snippet builders
├── researchtree.js             # Research definitions + tree logic + canvas
├── study13.js                  # Study XIII bindings & rewards
├── achievements.js             # Achievement definitions + tier system + locks
├── previousprestige.js         # Run records
├── navigation.js               # Tabs, subtabs, visibility predicates
├── main.js                     # basesave, g, all mechanics functions, save/load
├── statbreakdown.js            # miscStats definitions + stat evaluation engine
├── howtoplay.js                # Help texts
├── newsticker.js               # News items
├── htmlgenerator.js            # Builds repetitive DOM at init
├── gameloop.js                 # updateHTML() + tick() + auto_tick()
├── initialize.js               # Boot steps (runs last)
├── globalstyle.css, interface.css
├── changelog.html, credits.html, savebank.html, savesegmenter.html
└── img/
```

**Script order is the module system.** Each file assumes the globals of everything
loaded before it; `initialize.js` runs an ordered list of `initSteps` (load save,
generate HTML, compute stat order, start intervals) with a progress bar. There is a
`debugActive` flag (unlocked by a URL hash) that enables extra validation at boot.

---

## 3. Architecture

The architecture has three load-bearing ideas:

```
┌──────────────────────────────────────────────────────────────┐
│ g  (single global save object, cloned from basesave)         │
│    raw player state: currencies, axis counts, toggles, ...   │
└──────────────┬───────────────────────────────────────────────┘
               │ read by
┌──────────────▼───────────────────────────────────────────────┐
│ miscStats → stat   (142 derived stats)                       │
│   declarative modifier pipelines, dependency-sorted,         │
│   recomputed in full every tick into the `stat` object       │
└──────────────┬───────────────────────────────────────────────┘
               │ consumed by
┌──────────────▼───────────────────────────────────────────────┐
│ tick(time)          — mutates g using stat.* rates           │
│ updateHTML()        — rewrites innerHTML of the active tab   │
│ mechanics functions — buyAxis(), stardustReset(), ... in     │
│                       main.js, invoked by inline onClick     │
└──────────────────────────────────────────────────────────────┘
```

1. **All state in one mutable global `g`**, initialised as a deep clone of `basesave`
   (which doubles as the save schema — see §6).
2. **All derived values in the `stat` object**, computed by a declarative,
   dependency-ordered pipeline (§5). Mechanics code never recomputes multipliers ad hoc;
   it reads `stat.exoticmatterPerSec`, `stat.tickspeed`, `stat.XAxisCost`, etc.
3. **UI as a per-tick imperative repaint** of whichever tab is open, via a tiny DOM
   helper (`d.innerHTML`, `d.display`, `d.class`), with the static DOM skeleton
   generated once at boot by `htmlgenerator.js`.

There are no classes anywhere in the game code — everything is object literals,
closures, and functions attached to namespaces (`achievement`, `study13`,
`previousPrestige`, `corruption`, `autobuyerMeta`, …).

---

## 4. Number System

### 4.1 The Decimal fork

`break_eternity_alemaninc.js` is a fork of Patashu's `break_eternity.js`, representing
numbers as `(sign, layer, mag)` and supporting values up to 10↑↑9e15. The game routinely
operates at layer 1–2 (`ee15`-scale cost walls) and uses super-logarithms and tetration
in real formulas (`quad_slog`, `decimalPowerTower`, Study XIII names keyed on slog).

The fork adds a large family of gameplay-oriented methods used everywhere:

| Method | Meaning |
|---|---|
| `linearScaling / linearSoftcap` | piecewise-linear cost scaling and its inverse |
| `semiexpScaling / semilogSoftcap` | semi-exponential scaling pair |
| `exponentialScaling`, `superexpScaling` | harder scaling regimes |
| `logarithmicSoftcap` | `start × (1 + ln(x/start) × p)`-style cap |
| `convergentSoftcap` | asymptotic cap toward a hard limit |
| `dilate(p)` | `10^(log10(x)^p)` — exponent-space compression |
| `layerplus(n)`, `layerf(fn)` | shift/transform the layer representation |
| `decimalPowerTower([a,b,c])` | `a^(b^c)` for Decimal arrays |
| `affordGeometricSeries / sumGeometricSeries` | bulk-buy helpers |
| `decibel`, `fracDecibel_arithmetic` | "nice number near 10^(x/10)" quantiser |
| `add1Log`, `add1PowSub1`, `simplex`, `fix` | misc formula sugar |

`Decimal.prototype.fix(identity)` is the NaN firewall: every mutation through the `o`
operations object calls it, and in debug builds a NaN triggers a full game halt with an
error popup ("flagged by alemaninc's systems").

### 4.2 Constants pool

`emd_utility.js` builds a frozen constant object `c` with ~300 pre-allocated Decimals
(`c.d0_75`, `c.e25`, `c.ee15`, `c.inf` = 1.8e308, …), the same pattern as AD's `DC`.
All hot-path code uses these instead of allocating.

### 4.3 Mutation helpers

Incremental writes to Decimal fields of `g` go through a six-operation helper that
addresses fields **by string key**:

```javascript
const o = {   // o = "operations"
  add(variable,value) { g[variable]=g[variable].add(value).fix(0); },
  sub(variable,value) { g[variable]=g[variable].sub(value).fix(0); },
  mul(variable,value) { g[variable]=g[variable].mul(value).fix(1); },
  div(variable,value) { g[variable]=g[variable].div(value).fix(1); },
  pow(variable,value) { g[variable]=g[variable].pow(value).fix(1); },
  root(variable,value){ g[variable]=g[variable].root(value).fix(1); }
};
```

Three things are going on here:

1. **NaN firewall.** Every result passes through `Decimal.prototype.fix(identity)`:
   if the value is NaN, `error()` fires — halting both game loops and opening a popup
   that includes the current save *and* the save as of session start as exportable
   text — and the field is replaced by the operation's identity (0 for add/sub, 1 for
   mul/div/pow/root) so the state on disk stays loadable. The firewall is thus also the
   player's data-recovery path; corrupted values can never persist silently through the
   per-tick autosave. (A non-crashing variant, `fix(x, false)`, is used inside display
   formulas where NaN is expected and merely clamped.)

2. **String keys enable the family metaprogramming the game runs on.** Because the
   field is a name, one code path can serve whole families:
   `o.add(type+"Axis", c.d1)` covers all 12 axis types,
   `o.sub(autobuyers[id].resource, cost)` lets each autobuyer charge a different
   currency, and the scope-family increment helpers are one-liners over name lists —
   `for (i of exoticmatterVariables) o.add(i, x)` — which is what makes the
   nested-reset-scope accumulators (see the features report) cheap to maintain.
   Sibling helpers `toggle(name)` / `multitoggle(name, options)` do the same for
   boolean and enum fields.

3. **No side-channel semantics.** Unlike AD's `Currency` class — whose setters hook
   record tracking, autobuyer notifications, and hardcaps — `o` is pure arithmetic
   plus NaN screening. Record updates are the caller's job (the `increment*`
   functions), and nothing prevents bypassing `o` entirely.

And bypassed it is: `o` covers read-modify-write operations, but **direct assignment is
used wherever a value is computed or reset outright** — reset functions zero fields
with `g.exoticmatter = c.d0`, bulk-buy writes the solved amount with
`g[axisCodes[j]+"Axis"] = amount`, and chroma/lumen updates assign with a manual
`.fix(c.d0)` appended. Plain-`number` fields (`timePlayed`, `stars`, `galaxies`,
`dilatedTime`) are mutated with bare `+=`/`=` and get their own ad-hoc guards (the tick
loop checks the time counters are still `number`s and resets them with an apologetic
error popup referencing a known leak).

The net trade: centralised NaN handling and very cheap family/scope metaprogramming,
paid for with stringly-typed state access that no tool can type-check or find
references for — `g["dark"+x+"Axis"]` is invisible to grep for `darkXAxis` — and a
mutation discipline that holds only by convention, since half the codebase writes to
`g` directly anyway.

### 4.4 Formatting

`BEformat`/`format`/`noLeadFormat`/`formatFrom1`/`timeFormat`/`rateFormat` plus a
`formulaFormat` namespace that renders algebraic formulas as HTML (including generic
renderers for each softcap type). The global `showFormulas` hotkey flips nearly every
displayed number between value and formula — this works because every effect site
provides both a `text()`/value and a `formula()` string.

---

## 5. The Stat Pipeline (miscStats / stat DAG)

This is the most interesting subsystem and the closest thing EMD has to AD's
`GameCache` + effect composition — but it is more principled.

### 5.1 Definitions

`statbreakdown.js` declares 142 stats in `miscStats`. Each is one of two types:

```javascript
// "combined": a plain function
miscStats.wormholeDarkAxisReq = {type:"combined", value:function(){
  if (g.activeStudy===0) return c.e3;
  return N(studies[g.activeStudy].goal());
}}

// "breakdown": an ordered pipeline of named modifiers
miscStats.exoticmatterPerSec = {
  type:"breakdown", label:"Exotic Matter gain", category:"Exotic Matter gain",
  modifiers:[
    statTemplates.base("1",c.d1,false),
    { label:"X Axis",
      func:function(prev){return prev.mul(stat.XAxisEffect.pow(stat.realXAxis));},
      text:function(){return "× "+...},          // breakdown-table cell
      dependencies:["XAxisEffect","realXAxis"],  // DAG edges
      show:function(){return ...} },             // row visibility
    ...,                       // ~30 more: multipliers, then exponents,
    statTemplates.tickspeed()  // then dilations, then tickspeed
  ]
}
```

Modifiers are grouped by comment convention in a fixed order — base, multipliers,
exponents, dilations, tickspeed — mirroring the "hyper level" order used in cost
functions. Reusable modifier shapes live in `statTemplates` (mastery multiplier,
research multiplier, Study IX dilation, Binding 25 layer-shift, etc.).

### 5.2 Evaluation

At boot, `statGeneration(name)` computes each stat's depth in the dependency DAG (max
of dependencies + 1, memoised); `statOrder` is the list of stat names sorted by depth.
Every tick, `updateStats()` recomputes **all 142 stats in that order** into the flat
`stat` object. There is no caching or invalidation — the whole DAG is recomputed 20
times per second. In debug mode, `updateStats()` evaluates everything **twice** and
errors if any value changed between passes, which catches undeclared dependencies —
a cheap but effective consistency check.

### 5.3 One definition, three uses

The same modifier list powers:

1. **Simulation** — `updateStat(id)` folds `func` over the pipeline.
2. **UI breakdown** — the Statistics → Stat Breakdown tab renders each modifier's
   `label`, `text(prev)`, and running subtotal as a table row (skipping rows whose
   `show()` is false or which didn't change the value).
3. **What-if arithmetic** — `calcStatUpTo(id,label)` evaluates the pipeline up to a
   named stage (e.g. "base dark matter gain before dark-star effects") and
   `calcStatWithDifferentBase(id,base)` re-runs the pipeline from a hypothetical base,
   which the UI uses for "current → after next purchase" arrows.

This is a genuinely good pattern: effect composition, tooltip explanation, and
projection all derive from a single source of truth, so they cannot drift apart.

### 5.4 Cost

The obvious downside is performance and rigidity: everything is recomputed every tick
whether needed or not (fine at this scale — the game targets 20 fps and even ships a
frame-time stat), dependencies are stringly-typed and must be maintained by hand, and
the modifier `show`/`text` functions mix presentation into the math layer.

---

## 6. Player State & Save System

### 6.1 `basesave` as schema

`main.js` opens with a ~265-line `basesave` object literal containing every persistent
field: currencies (as `c.*` Decimal constants), 36 axis counts (`XAxis`…`antiOAxis` as
flat fields), toggles, autobuyer settings, records, per-feature sub-objects
(`study9`, `study12`, `luckUpgrades`, `prismaticUpgrades`, …). The live game state is
`var g = decimalStructuredClone(basesave)`.

Fields that anticipate unreleased content are already present
(`timeThisSpacetimeReset`, `previousStardustRuns.eternity`, `zipPoints`,
`corruptionsUnlocked`), so old saves stay forward-compatible.

### 6.2 Save / load

- **Save:** `localStorage.setItem("save", JSON.stringify(g))` after every tick (if
  autosave on). Decimals serialise via their `toJSON`/string form.
- **Load:** `getSavedGame(saved, game, base)` recursively walks the *saved* object and
  copies values into a fresh `basesave` clone, keyed by what exists in `basesave`:
  unknown saved fields are dropped, missing fields keep defaults, Decimal fields are
  revived with validation (`Decimal.valid`). This makes schema evolution mostly
  automatic — adding a field to `basesave` is the whole migration for the common case.
- **Explicit migrations** for structural changes live inline in `load()` (e.g. old
  `ownedAchievements` array → `achievement` map) and in `oldsaves.js` for pre-1.0 saves
  (which are approximated rather than converted: the loader reconstructs a plausible
  state from a few key fields).
- **Export/import** is `btoa(JSON)`; import sanity-checks a list of expected keys and
  warns before overwriting. Importing an actual Antimatter Dimensions save string is
  detected and awards a secret achievement.

There is no versioned migration chain, no checksum, and no cloud saving; a separate
`savebank.html` page implements a community save-sharing catalogue keyed by EMD
Level/Score.

### 6.3 Derived-state reconstruction

Counters like `totalAchievements`, `totalStars`, `totalResearch` are plain globals
recomputed at load, then maintained incrementally — a small consistency risk the code
accepts for speed.

---

## 7. Game Loop

Two fixed intervals, started by `initialize.js`:

```javascript
gameloop     = window.setInterval(auto_tick, 50);   // 20 fps simulation + UI
fineGrainLoop = window.setInterval(fineGrainTick, 10); // news ticker, notifications
```

`auto_tick()` computes wall-clock `deltatime`, applies the **time state** machine
(0 = normal, 1 = Overclock: spend dilated time for a speed factor, 2 = frozen: bank all
time as dilated time, 3 = equalized: force exactly 50 ms frames, banking the excess),
then calls `tick(deltatime × overclockFactor)` followed by `updateHTML()`.

`tick(time)` in order:

1. EMD level refresh; negative-time error guard; time-state normalisation.
2. Accumulate the four nested clocks (`timePlayed`, per-Stardust/Wormhole/Spacetime) in
   both real and tickspeed-adjusted ("truetime") variants.
3. Corruption unlock checks; story popups.
4. Dilation-upgrade unlocks (tickspeed thresholds).
5. Achievement event dispatch (`achievementEvents.gameloop`, random "lucky" secret
   achievements, milestone loops).
6. Study housekeeping (Study IX 9-second self-reset, Binding 236 timeout, forced exit
   if the enabling research is missing — a live bug guard that apologises to the player).
7. **Incrementer section** (deliberately last so resets work): energies multiply
   (`g[energy] *= energyPerSec^dt`), then EM, mastery power, stardust drip, dark
   matter, knowledge, chroma generation/lumen conversion, luck shards, prismatic,
   antimatter — all as `stat.*PerSec × time`.
8. **Automation section**: five interval autobuyers accumulate progress and fire
   `buyMax*` functions; stardust/wormhole automators evaluate their mode predicates and
   trigger `attempt*Reset(false)`; the research autobuyer buys all free research in a
   loop with an explicit 1-second wall-clock bailout ("Infinite Loop" error).
9. Autosave.

Notable: **offline progress is not simulated at all.** On load, elapsed time becomes
dilated time (`g.dilatedTime += (now - g.timeLeft)/1000`), which the player spends via
Overclock. This is a design decision, not a limitation (see overview doc §3.5).

There is also a self-diagnosing memory guard: if any of the time counters stops being a
`number`, it is reset with an in-game error popup referencing a known leak under
investigation.

---

## 8. Core Mechanics Implementation

### 8.1 Axes (the "generators")

Three families (normal / dark / anti) × 12 types share one implementation pattern, all
in `main.js`:

```javascript
function axisCost(type, axis) {
  axis = Decimal.semiexpScaling(axis, stat.axisSuperscalingStart, realAxisSuperscalePower(type));
  axis = Decimal.linearScaling(axis, stat.axisScalingStart, realAxisScalePower(type));
  // 12 hand-written base curves:
  if (type==="X")      cost = c.d6.pow(axis).mul(c.d5);
  else if (type==="Y") cost = c.d1_5.pow(axis.simplex(2)).mul(c.e2);
  ...
  else if (type==="O") cost = axis.add(c.d35).div(c.d30).layerplus(3);
  cost = corruption.value("axis", cost);        // cost wall past a threshold
  cost = cost.pow(realAxisCostExponent(type));  // all ^-type reductions
  return cost.div(realAxisCostDivisor(type));   // all ÷-type reductions
}
```

Every curve has a **hand-derived closed-form inverse** in `maxAffordableAxis(type)`
(apply inverse reductions, invert the base curve, then apply the *softcap inverses* in
reverse order). Buy-max is therefore O(1) per type per tick with no binary search —
at the price of two formulas to keep in sync per axis type per family (~72 pairs).
A code comment admits the risk: *"maxAffordableAxis() doesn't seem to work properly
because people are getting negative EM"* — patched by clamping EM to ≥ 0.

`realAxisCostExponent/Divisor` are the extension points where research, masteries,
study rewards, and bindings hook in per-type cost modifiers.

### 8.2 Resets

Reset functions are imperative field-by-field zeroing in `main.js`
(`stardustReset()`, `wormholeReset()`, `gainDarkStar()`, `gainGalaxy()`), each encoding
its "keep" rules inline (e.g. Stardust Upgrade 2 retains a percentage of the first N
axes; Wormhole clamps stardust upgrades to `[0,1,0,5,0]`; Wormhole Milestone 7 stops
dark stars resetting dark matter). `wormholeReset()` also owns Study completion logic —
checking the goal, bumping completions, respeccing research, and unlocking features —
which makes it the single most complex function in the game (~110 lines). There is no
generic reset framework; each layer is bespoke.

### 8.3 Challenges (Studies)

Studies are config objects in `data.js` (`studies[1..13]`) with
`unlockReq/description/goal/reward/reward_desc` plus per-study custom functions (Study
VII's luck-essence cosine, Study IX's experientia, Study XII's fortitude softcap).
Restrictions are enforced by **scattered `StudyE(n)` checks** across the codebase
(≈70 call sites), exactly like AD's `Challenge.isRunning` checks. Study XIII gets its
own file with `bindings` (~87 nodes, each `{description, adjacent_req, lv, effect,
nameMod}`) and `rewards` (~24 named rewards with `breakpoints` and `scaling |
composite | single` types).

### 8.4 Corruption

A generic "cost wall" wrapper in `data.js`: past a per-family threshold, base costs are
mapped by `start^((log(x)/log(start))^power)` with an exact inverse for buy-max. Applied
inside the three axis-cost functions and dark-star requirement.

---

## 9. Data & Configuration Layer

There is no single game database; declarative configs are spread by feature:

- `data.js` — Studies, luck runes/upgrades (with the `cascade` cross-boost graph),
  prismatic upgrades (limited/unlimited/refundable variants with per-variable
  `eff/format/formula` maps), wormhole upgrades, corruption.
- `researchtree.js` — 192 research nodes keyed `r{row}_{col}`, each
  `{description, adjacent_req, condition[], visibility, type: normal|permanent|study,
  basecost, icon, effect(power), group}`. Factory functions stamp out families
  (Study-V research, prismal research, luck research, antimatter research, finality
  chains). Group metadata (`researchGroupList`) defines intra-group cost interference,
  and `researchPower()` centralises every effectiveness modifier.
- `achievements.js` — `achievementList[tier][id]` with
  `{name, description, check, event, progress, reward, effect?, yellowBreakpoints?,
  flavor}`; `achievement.maxForLocks` declares, per lockable achievement, exactly which
  actions/limits the lock enforces (axis caps, reset bans, research bans…).
- `statbreakdown.js` — as described in §5, also functions as the master registry of
  effect *application order*.
- Various in-file config arrays (dilation upgrades, stardust boost texts,
  wormhole milestones, galaxy effects, light/lumen data).

Configs are **not** passive data — nearly every field is a closure reading `g`/`stat`,
and validation (`validateResearch`, modifier `show` checks) runs only in debug mode at
boot.

A recurring convention: every effectful entity exposes `eff()` (value), `format()`
(display), and `formula()` (HTML algebra), which is what makes the global
formula-display mode possible.

---

## 10. UI Layer

- **Static skeleton:** `index.html` (461 lines) holds the tab scaffolding;
  `htmlgenerator.js` string-builds all repetitive DOM (axis buttons, mastery/star
  tables, achievement grid, stat panels, upgrade lists) once at boot. `iconGen.js`
  provides inline-HTML icon snippets.
- **Per-tick repaint:** `updateHTML()` (gameloop.js, ~800 lines) is a giant
  `if (g.activeTab===...)` chain that rewrites `innerHTML`/classes/styles for the
  visible tab only. There is no diffing and no data binding; cheap because only one tab
  paints at a time.
- **Events:** inline `onClick="buyAxis('X',true)"` handler strings throughout; popups
  (`popup({text, input?, buttons:[[label, jsString]]})`) execute their button actions
  via string-injected JS. This is effectively `eval`-driven UI wiring — compact but
  fragile and XSS-adjacent (acceptable for a local single-player game).
- **Navigation:** `navigation.js` defines per-tab/subtab `visible()`/`glow()`
  predicates; glow highlights actionable subtabs (configurable per feature).
- **Canvas:** only the research tree's connection lines use canvas; nodes are DOM
  buttons whose font size is auto-shrunk to fit (`resizeResearch` measuring against a
  hidden `#foo` element).
- **Theming:** CSS variables per resource colour; 11 selectable themes; one secret
  animated theme.
- **Extras:** progress bar with a declarative milestone list, notification queue with
  hand-rolled easing, news ticker with speed/dilation options, hotkey system with
  rebindable keys stored in the save.

---

## 11. Automation

`autobuyers` (main.js) declares five interval-based autobuyers
(axis, dark axis, stardust upgrade, star, research), each
`{baseInterval, baseCost, costGrowth, resource, unlockReq}`. `autobuyerMeta` derives
interval/cost/softcap/cap generically: each upgrade level cuts the interval by 5% (1%
past the softcap) down to a floor of 0.1 s (0.05 s with Wormhole Upgrade 9); upgrade
costs grow geometrically in a *feature-appropriate resource* (e.g. the stardust-upgrade
autobuyer charges stelliferous energy).

Above these sit two threshold **automators** (stardust, wormhole) with six trigger
modes (amount / time / ×previous / ^previous / ×current / ^current), a **star
allocator** that replays a saved build, and per-autobuyer cap inputs read directly from
DOM `<input>` values each frame (the DOM is the source of truth for those settings —
an odd but consistent choice).

Unlocks come from Wormhole Milestones (Tier-5 achievement count), mirroring AD's
eternity-milestone QoL ladder.

---

## 12. Key Architectural Patterns

1. **Single-source declarative stat pipelines** (§5) — the standout pattern:
   simulation, breakdown UI, formula display, and what-if projection all read one
   modifier list. AD achieves the first with `timesEffectsOf(...)` but has nothing for
   the other three.

2. **Closed-form inverses everywhere.** Every cost curve, softcap, and corruption map
   ships with its inverse, so bulk-buy and "next threshold" displays are exact and O(1).
   The discipline this requires is visible (paired `*Scaling`/`*Softcap` methods).

3. **Ordered effect algebra.** Effects apply in a documented order — add, then
   multiply, then exponentiate, then dilate ("hyper-1 … hyper-3.5") — both in cost
   reductions and stat pipelines. This keeps balance predictable when dozens of sources
   stack.

4. **`basesave` as schema + tolerant recursive load** — one object literal defines
   state shape, defaults, and (implicitly) the migration policy.

5. **Feature flags via `g.featuresUnlocked` strings** (`unlocked("Stardust")`), with
   story popups fired exactly once per feature by `unlockFeature()`.

6. **Event-tagged achievements**: achievements declare an `event`
   (`"axisBuy"`, `"stardustReset"`, `"gameloop"`, …) and call sites dispatch
   `addAchievements(event)`, avoiding per-tick scanning of all 142 checks.

7. **Achievement locks as declarative action guards** (`achievement.maxForLocks`):
   mechanics functions consult the lock table before acting — a lightweight,
   data-driven "challenge mode" implementation.

8. **Debug-only invariant checking**: double evaluation of the stat DAG, research
   config validation, stat-modifier `show` audits — a poor man's test suite that runs
   at boot in debug builds only.

---

## 13. Code Quality & Modularity Assessment

**Strengths**

- The stat-pipeline design is excellent and is the reason the game can offer full
  numeric transparency; it also keeps effect application order globally consistent.
- Formula discipline: `eff/format/formula` triples and inverse functions are maintained
  with impressive consistency across ~200 effect sources.
- Save robustness: tolerant loading, NaN firewalls, in-game error reporting with a
  pre-error save export — the game actively helps players survive its own bugs.
- Constant pooling, event-tagged achievements, and single-tab repainting show real
  attention to hot-path performance despite the naive full-DAG recompute.
- Given one author and no tooling, internal conventions (naming, file roles, comment
  markers like `// hyper-3 cost reductions`) are followed reliably.

**Weaknesses**

- **Global everything.** One mutable `g`, cross-file implicit dependencies resolved by
  script order, stringly-typed field access via `o.add("exoticmatter", …)` and
  `g[type+"Axis"]`. Refactoring safety is near zero; only the boot-time debug checks
  stand in for tests.
- **Presentation braided into logic.** `main.js` mechanics functions build HTML
  strings; stat modifiers own their table-cell rendering; `updateHTML()` contains
  gameplay side effects (input harvesting, forced tab switches in Studies). There is no
  layer that could run headless without a DOM shim.
- **Duplication with sync burden**: 72 cost/inverse pairs, parallel
  normal/dark/anti implementations of near-identical axis logic, `Legacy`/`Modern`
  duplicated UI variants for masteries and stars.
- **Scattered rule checks**: Study restrictions and achievement locks are enforced by
  dozens of independent `if` guards; a missed guard is a silent exploit (several
  comments — "Study X proofing", "no exploits :D" — mark past instances).
- **Inline-string event handlers and popup actions** are effectively `eval` and make
  grepping for behaviour harder.
- Magic numbers abound by design (it's a numbers game), but many balance constants
  appear only inside formulas with no named definition.

**Overall:** a highly disciplined single-author codebase in an undisciplined medium.
The core ideas — declarative stat pipelines with dependency ordering, invertible cost
curves, formula-as-first-class-citizen — are genuinely better than what the original
Antimatter Dimensions does in those areas and are worth studying, while the delivery
(global state, DOM-braided logic, no tests) is exactly what you'd expect from a
hobbyist vanilla-JS game and not a model to imitate.

---

## 14. Good Ideas to Borrow

Ideas from this codebase worth adopting in our engine, independent of whether EMD's
own implementation of them should be imitated.

### 14.1 The NaN firewall (validity checks at the state boundary, with recoverable saves)

Incremental games are formula soup: one `log` of a negative intermediate — or a
division by an effect that some new modifier can now push to zero — produces a NaN that
propagates silently through every downstream multiplier and into the autosave, where it
destroys the run. EMD defends against this at the state-write boundary: all
read-modify-write mutations pass through `Decimal.prototype.fix(identity)` (§4.3),
and on NaN the game

1. **halts both loops immediately**, so the corruption cannot compound or autosave;
2. **opens an error popup containing two exportable saves** — the state just before the
   error and the state at session start — so the player always has a recovery path;
3. replaces the offending field with the operation's identity so the on-disk state
   stays loadable.

The changelog suggests this has caught real regressions repeatedly; per line of code
it may be the most valuable defensive measure in the game.

**How to borrow it in Rust.** Don't port the mechanism (a politely-requested choke
point that half of EMD's own code bypasses); port the guarantee, moved into places the
compiler enforces:

- **Validity by construction:** the `Decimal` type's constructors and arithmetic should
  never yield NaN — normalise, saturate, or return `Result` — so there is no unguarded
  write path to protect. `debug_assert!`s inside the arithmetic catch violations in
  development at zero release cost.
- **Narrow write interfaces where side effects live:** currency mutations go through a
  `Currency` type with private fields (this is also where record tracking and caps
  belong, as in AD's `Currency` class); module privacy makes the choke point mandatory
  rather than conventional.
- **Halt-with-recoverable-save:** a panic hook (or WASM error boundary) that snapshots
  the last-known-good state and surfaces it to the player. This half of the idea is
  independent of how mutations are routed and deserves to be carried over as-is: the
  player should never lose a save to our bug.

The general principle: **corrupted state must be impossible to persist silently.**
EMD achieves it with runtime discipline; we can achieve it with types plus one error
boundary.

### 14.2 Single-source stat pipelines (simulation = explanation = projection)

Every derived stat is one declarative modifier list (§5), and that single definition
drives four consumers: the simulation fold, the Stat Breakdown table (each modifier
renders its own row), "current → after next purchase" projections
(`calcStatWithDifferentBase` re-runs the pipeline from a hypothetical base), and the
global formula-display mode. Because there is one source of truth, the explanation UI
*cannot* drift from the math — a chronic problem in AD, where tooltips, wiki formulas,
and code are maintained separately.

**How to borrow it in Rust.** Our config-driven engine work is already circling this
shape; the insight to keep is that the effect pipeline should be **data with
metadata**, not a fold of closures. Something like:

```rust
struct Modifier {
    source: EffectSource,          // enum: what grants this effect
    stage: Stage,                  // see 14.4
    apply: fn(&GameState, Decimal) -> Decimal,
    is_active: fn(&GameState) -> bool,
    // display metadata — the part EMD proves is worth carrying:
    label: &'static str,
    render: fn(&GameState) -> FormulaFragment,
}
```

The engine folds `apply` over the pipeline; the frontend walks the same list to build
breakdown tables and formula views; what-if projection is the same fold with a
substituted base. One definition, no drift. (EMD's cost — recomputing the whole
142-stat DAG every tick with hand-declared string dependencies — is not part of the
idea; with typed stats we can cache per tick and derive dependencies structurally.)

### 14.3 Closed-form inverses as a design discipline

Every cost curve, softcap, and corruption map in EMD ships with its exact inverse
(§8.1), so buy-max is O(1) arithmetic and "next threshold at X" displays are exact.
AD instead binary-searches (`bulkBuyBinarySearch`) — slower, and approximate at the
boundaries. EMD's weakness is that the ~72 forward/inverse pairs are kept in sync only
by hand, which the author admits has broken ("people are getting negative EM").

**How to borrow it in Rust.** Make the pairing structural: a trait

```rust
trait ScalingCurve {
    fn cost(&self, n: Decimal) -> Decimal;
    fn max_affordable(&self, budget: Decimal) -> Decimal;  // exact inverse
}
```

with each curve implemented once as a value (parameters as fields), and a property test
over every implementation asserting `max_affordable(cost(n)) == n` across the domain.
That converts EMD's one admitted bug source into a test-enforced guarantee while
keeping the O(1) bulk-buy and exact-threshold benefits.

### 14.4 Ordered effect algebra ("hyper levels")

EMD applies stacked effects in a fixed, documented order everywhere: additive terms,
then multipliers, then exponents, then dilations (the comments call the cost-reduction
tiers "hyper-1 … hyper-3.5"). Because the order is global and consistent, the
interaction of 50 stacked sources stays predictable and auditable — you can reason
about "this new upgrade is a hyper-2, so it lands after all additions and before any
exponent" without reading every other effect.

**How to borrow it in Rust.** Don't rely on comment conventions; encode the stage:

```rust
enum Stage { Add, Multiply, Power, Dilate }
```

as a field on every modifier (see 14.2), and have the pipeline constructor sort by
stage (or reject out-of-order definitions). A mis-ordered effect becomes unrepresentable
rather than a balance mystery, and the stage enum doubles as documentation for
designers adding new effects.

### 14.5 Debug-mode invariant checking (the double-evaluation trick)

In debug builds, EMD evaluates the entire stat DAG **twice** per update and raises an
error if any value differs between passes (§5.2) — a three-line check that catches
undeclared dependencies (a stat reading another stat that hadn't been recomputed yet).
Boot-time config validation (`validateResearch`, modifier-shape audits) works the same
way: cheap structural checks, debug-only, zero release cost.

**How to borrow it in Rust.** With typed stats, most dependency errors become compile
errors, but the fixpoint check survives as a property test — evaluate the derived
state twice from the same inputs and assert equality — which doubles as a determinism
test, something we care about anyway for save/simulation reproducibility and any future
offline-progress fast-forward. Config validation belongs at load time behind
`debug_assertions` or in a test that loads every shipped config.

### 14.6 Event-tagged achievement checks

Achievements declare which event can complete them (`event: "axisBuy"`,
`"stardustReset"`, `"gameloop"`, …) and call sites dispatch `addAchievements(event)`,
so only the relevant subset of the 142 predicates runs per action instead of scanning
all of them every tick. Trivial to map to Rust — an `AchievementTrigger` enum and a
per-variant index built at startup — and the right shape for our achievement system as
the count grows past the point where a full per-tick scan is tasteful.

---

*Document generated from source analysis on 2026-07-03.*
