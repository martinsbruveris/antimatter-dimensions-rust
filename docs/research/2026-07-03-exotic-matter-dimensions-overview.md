# Exotic Matter Dimensions — Game Overview & Comparison with Antimatter Dimensions

**Source:** `../incrementals/exotic-matter-dimensions`
**Author:** alemaninc (solo developer)
**Version analysed:** 𝕍1ω.16 (2026-05-18); first public release 𝕍1ω on 2024-06-14
**Companion documents:**
[codebase analysis](2026-07-03-exotic-matter-dimensions-codebase.md) ·
[feature decomposition](2026-07-03-exotic-matter-dimensions-features.md)

---

## 1. What the Game Is

Exotic Matter Dimensions (EMD) is a browser incremental game that describes itself as
"loosely inspired by *Antimatter Dimensions*, *Synergism* and *Incremental Mass
Rewritten*". The player starts by passively generating **exotic matter** at 1/second and
spends it on **Axes** (X, Y, Z, W, V, U, T, S, R, Q, P, O — up to twelve spatial
dimensions), each of which improves production in a different way. From there the game
grows through a stack of reset layers (Stardust → Dark Stars → Wormhole), a research
tree, a challenge system ("Studies"), and half a dozen late-game currencies, ending
(currently) in a build-your-own-challenge system called Study XIII. The declared next
layer, the "Matrix", is stubbed throughout the code but unreleased.

The game tracks the player's progress with an **EMD Level** (1–9), which is a useful map
of the content arc:

| EMD Level | Unlocked by | Headline content |
|-----------|-------------|------------------|
| 1 | start | Exotic matter, axes, Masteries, offline time |
| 2 | Stardust | Stardust reset, Stardust Boosts/Upgrades, Stars |
| 3 | Dark Matter | Dark axes, Dark Stars |
| 4 | Energy | Ten exponentially-growing Energy resources |
| 5 | Hawking Radiation | Wormhole reset, Research tree, Studies I–XII, Wormhole Milestones |
| 6 | Light | Chroma / Lumens (9 colours) |
| 7 | Galaxies | Star-cap prestige with boost/penalty pairs |
| 8 | Luck / Prismatic / Antimatter | Study VII/IX rewards, luck runes, prismatic upgrades, anti-axes, Corruption |
| 9 | Study XIII | Binding tree, named rewards, current endgame (200 completions) |

A completed run to the current endgame is in the multi-week range; the game is balanced
around long formula-driven ramps rather than dense unlock cadence.

---

## 2. The Core Production Model

EMD's most important structural difference from Antimatter Dimensions is that **there is
no production cascade**. Antimatter Dimensions builds its growth curve from dimensions
buying/producing each other (AD8 → AD7 → … → antimatter), so growth compounds
mechanically. In EMD the base gain of exotic matter is a flat 1/second, and *all* growth
comes from composing modifier effects on top of it:

- **X axis** multiplies exotic matter gain (each level is one more factor).
- **Y axis** increases the X axis effect additively; **P axis** multiplies the Y effect.
- **Z, W, T axes** give multipliers based on current exotic matter, time, and total axes.
- **S axis** raises exotic matter gain to a power (>1) — the marquee upgrade.
- **R, V axes** reduce axis costs (self-referential economy).
- **Q axis** feeds the Energy system; **O axis** multiplies the *effective levels* of all
  other axes (a meta-axis).

Each axis type has a bespoke cost curve (from `6^n × 5` for X up to power towers and
`layerplus` constructions for R/Q/P/O), and each has a hand-written closed-form inverse
so "buy max" is O(1) rather than a binary search.

The interesting consequence: EMD's balance lives almost entirely in *formula shape*.
Effects routinely apply as exponents (`gain^1.05`), dilations
(`10^(log10(x)^0.9)`), or iterated logarithms rather than plain multipliers, and nearly
every effect passes through an explicit softcap (the codebase has a whole library of
them: linear, logarithmic, convergent, semi-exponential). The developer even formalises
a hierarchy of "hyper levels" for cost reductions — subtract (hyper-1), divide
(hyper-2), root (hyper-3), dilate (hyper-3.5) — and applies them in a fixed order inside
cost functions. Numbers are handled by a fork of `break_eternity.js`, so layered
exponents and tetration are usable as *gameplay* operators, not just display notation.

A second consequence is transparency: because every stat is computed by an ordered list
of named modifiers, the game can show the player a full breakdown of every number (the
"Stat Breakdown" tab) and a **formula view hotkey** that replaces every displayed value
with its algebraic formula. This "the math is the content" attitude is the game's
signature.

**Aside: the layer system and `layerplus`.** break_eternity stores a number as
`(sign, layer, mag)`, meaning `sign × 10^10^…^mag` with `layer` nested exponentials —
`(1, 0, 5)` is 5, `(1, 1, 5)` is 10^5, `(1, 2, 5)` is 10^(10^5). The fork's
`x.layerplus(k)` simply adds `k` to the layer field: `layerplus(3)` applies `10^x`
three times (shown in-game as `Ξ^[3] x`), `layerplus(-3)` applies `log10` three times —
O(1), exactly self-inverse, no precision loss. The "layerplus constructions" above are
cost curves built directly from this: the O axis is priced as
`cost(n) = ((n+35)/30).layerplus(3)` = `10^10^10^((n+35)/30)`, so each level advances
the cost *linearly in triple-log space* (level 25 costs 10^(10^100)), and the buy-max
inverse is the mirror one-liner `EM.layerplus(-3) × 30 − 35`. This is the clearest
instance of "layered arithmetic as a gameplay operator": once a currency lives at layer
2+, geometric cost curves are either flat or unaffordable, so the designer prices
things linearly in slog-space instead — and the representation makes both the curve and
its exact inverse a single field-addition. A generalised sibling, `layerf(fn)`,
transforms the layer coordinate by an arbitrary function; Study XIII's Binding 25 uses
it to apply a *fractional* iterated logarithm (`x → log^[1.4](x)`) to all core gains.
(The neighbouring R/Q/P axes use the related `decimalPowerTower` construction,
`base^(ratio^n)` — a fixed power tower rather than a layer shift.)

---

## 3. Style of Mechanics

### 3.1 Reset layers

```
Galaxy (destroys wormhole progress, at 60 stars)
  Wormhole  (resets everything below + stardust layer; grants Hawking Radiation)
    Dark Star  (resets stardust layer + dark matter; grants dark stars)
      Stardust  (resets exotic matter + axes; grants stardust)
```

Each layer follows the classic pattern: the reward currency has both a passive role
(stardust drives Stardust Boosts and dark matter gain; Hawking radiation drives
Wormhole Upgrades and milestone effects) and a shop role (Stardust Upgrades, stars,
observations). Higher layers deliberately *don't* award anything innately — Hawking
radiation "has no innate effect" and must be spent on research infrastructure, which is
a nice inversion of the usual power spike.

### 3.2 Choice-based systems

EMD leans harder on *loadout* mechanics than AD does:

- **Masteries** — 11 rows of free toggleable upgrades, initially one active per row;
  their strength scales with "mastery power", which grows from a timer that resets when
  you reassign. Choosing masteries is a continuous respec decision.
- **Stars** — stardust buys stars; stars are allocated into rows of four upgrades with
  per-row capacity, and respec forces a reset. Star builds are exportable strings.
- **Research** — a 47-row × 15-column tree (192 nodes) bought with Discoveries
  (log of Knowledge). Groups of research (Energy, Stardust, Chromatic, Spatial
  Synergism, Luck, Antimatter, Finality…) have intra-group cost or effectiveness
  penalties, so tree-building is a constrained-optimisation puzzle. Respec on Wormhole.

### 3.3 Challenges ("Studies")

Studies are EMD's challenge system and map closely to AD challenges: entering one forces
a Wormhole reset and applies restrictions ("Bindings"); completing it (reach N total
dark axis) grants permanent rewards. Distinctive twists:

- Each Study has **4 completions with escalating restrictions and goals** (like AD's
  Eternity Challenges), and 3 separate rewards whose magnitudes scale with completions.
- **Study X** is a "Triad" — it applies three other Studies simultaneously, in four
  variants (the last lets the player pick any three).
- Studies are unusually inventive as *rule mutations*: Study VII adds a luck/cosine
  mechanic where production oscillates with accumulated "luck essence"; Study IX resets
  itself every 9 real seconds and awards "experientia" based on how well you did; Study
  XI cycles which single axis type is active every 750 ms ("Lunar Clock").
- **Study XIII** is a freeform challenge builder: the player toggles ~87 "Bindings" in
  a dependency tree, each worth binding levels; completing the run sets completions to
  the total binding level (up to 200/256), which unlocks and upgrades ~24 named rewards
  at breakpoints. It even procedurally names the challenge from the chosen bindings.
  This is the game's current endgame and its most original design.

### 3.4 Achievements as a mechanic

142 achievements across 9 tiers (plus ~40 secret ones). Beyond individual rewards, each
*tier* has an aggregate per-achievement reward (e.g. Tier 5: base knowledge gain ×
achievements owned). The standout mechanic is **achievement locking**: the player can pin
an incomplete achievement to the progress bar, and the game then *blocks* any action
that would fail it (buying the forbidden axis, resetting, etc.) with a confirmation
popup. Constraint-achievements ("complete a Wormhole without buying X") thereby become
self-enforcing challenge runs without any separate challenge infrastructure.

### 3.5 Offline time as a currency

Instead of simulating offline progress, EMD banks it: 1 second offline = 1 second of
**dilated time**. Dilated time is spent on:

- **Overclock** — a game-speed multiplier (efficient up to a softcap, wasteful beyond);
- **Freeze / Equalize** — pause the game or force exact 50 ms frames (for timed
  achievements and certain Studies);
- **Dilation Upgrades** — permanent overclock improvements, unlocked at tickspeed
  thresholds;
- **Wormhole Amplification** — spend banked time to multiply a Wormhole reset's yield.

This makes offline/online time a fungible resource and neatly sidesteps offline
simulation fidelity issues.

### 3.6 Anti-frustration and spectacle

Wormhole Milestones (driven by Tier-5 achievement count, 30 of them) are the direct
analogue of AD's eternity milestones: autobuyers, auto-resets, "dark stars no longer
reset dark matter", culminating in "gain all pending stardust immediately". The game
also carries AD's DNA in its news ticker (720 lines of jokes), secret achievements,
story popups at each feature unlock, and a save-wipe confirmation that demands typing a
random meme phrase.

---

## 4. Comparison with Antimatter Dimensions

### 4.1 Structural parallels

| Antimatter Dimensions | Exotic Matter Dimensions | Notes |
|---|---|---|
| Antimatter | Exotic matter | Base currency, but EMD's base gain is fixed at 1/s |
| 8 cascading dimensions | 12 flat axes × 3 families (normal/dark/anti) | No cascade; axes are effect sources |
| Dimension Boost / Galaxy | Dark Star / Galaxy | Sub-resets inside a bigger layer |
| Infinity (Big Crunch) → IP | Stardust reset → stardust | First real prestige |
| Eternity → EP | Wormhole → Hawking radiation | Second prestige, resets first layer's shop |
| Reality (planned analogue) | Matrix (unreleased) | Both games stub the next layer early |
| Normal/Infinity/Eternity Challenges | Studies I–XII (4 completions each) | Same enter/restrict/complete pattern |
| Time Studies tree | Research tree (192 nodes) | EMD's has group-based cost interference |
| Eternity milestones | Wormhole Milestones (30) | Nearly identical QoL escalation |
| Achievements w/ rewards | Achievement tiers + locking | EMD adds aggregate tier rewards + lock mechanic |
| Tickspeed | Tickspeed (stat) | In EMD it's a derived stat, not a purchase |
| Autobuyers w/ intervals | 5 interval autobuyers + reset automators | Same "upgrade interval down to a cap" model |
| break_infinity.js | break_eternity.js fork | EMD needs layers ≥2 routinely |

### 4.2 Key design differences

1. **Growth engine.** AD's exponential growth is *mechanical* (dimensions produce
   dimensions); EMD's is *analytical* (a fixed base pushed through dozens of
   multiplier/exponent/dilation modifiers). AD's model produces the satisfying
   "cascading purchases" feel; EMD's model makes every unlock legible as a formula
   change and allows much more aggressive use of exponent-space effects.

2. **Softcaps as vocabulary.** AD mostly gates progress with cost scaling breakpoints
   (distant/remote galaxies, super-exponential costs). EMD softcaps *effects*
   everywhere, with the softcap parameters themselves being upgrade targets ("Overclock
   softcap starts 20 later", "Masterful limit raised to …"). Progression often means
   buying back headroom on a curve rather than buying a bigger number.

3. **Transparency.** EMD exposes its own math: per-stat breakdown tables generated from
   the same modifier pipeline the simulation uses, plus a global formula-display mode.
   AD (base game) hides nearly all formulas; players rely on the wiki. This is arguably
   EMD's biggest UX innovation and comes essentially for free from its architecture.

4. **Choice density.** AD pre-Reality is mostly monotone purchasing plus a few
   path choices (time study paths). EMD front-loads loadout decisions (masteries, star
   allocation, research groups with interference penalties) from the first hour.

5. **Challenge generation.** AD's challenges are all hand-authored. EMD ends with a
   player-composed challenge (Study XIII) whose difficulty is priced in binding levels —
   a genuinely novel structure closer to roguelike ascension modifiers than to AD.

6. **Offline model.** AD simulates missed time; EMD converts it to a spendable currency.

7. **Scale and pacing.** AD is a vastly larger game (~119k lines, 7 celestials of
   endgame); EMD is ~25k lines with one developer, and its pacing is slower and more
   grind-tolerant, leaning on very long formula ramps between feature unlocks.

### 4.3 Mechanics worth considering for an AD expansion/mod

- **Achievement locking** — turns constraint achievements into self-enforcing challenge
  runs with almost no new UI. Would slot naturally into AD's achievement system.
- **Stat breakdown / formula view** — the modifier-pipeline pattern that powers it is
  very close to what a config-driven engine produces anyway; exposing it to players is
  cheap and beloved (community mods for AD already attempt this).
- **Banked offline time (dilated time + overclock)** — an alternative to `simulateTime()`
  that is both simpler to implement and more interesting to play; note AD already has a
  cosmetically similar "stored time" mechanic on Enslaved, but EMD makes it the *only*
  offline mechanism and prices it with a softcap.
- **Build-your-own challenge (Study XIII bindings)** — a strong endgame archetype: a
  tree of composable nerfs, reward scaling with total difficulty taken, procedural
  naming for flavour. Could be an alternative or supplement to a new Celestial.
- **Triad challenges (Study X)** — "run three old challenges at once" is a cheap way to
  recycle authored content into new difficulty tiers (AD's IC1 "all normal challenges at
  once" is the degenerate version of this).
- **Wormhole amplification** — spending banked time to multiply a prestige's yield
  creates a nice active/idle tradeoff at reset boundaries.
- **Effect-space upgrades** (raise X to a power, dilate a cost curve, move a softcap) as
  a late-game upgrade vocabulary, enabled by break_eternity-class numerics.

### 4.4 What EMD does worse

- **Onboarding/pacing:** the first hours are slow (the news ticker jokes about it), and
  several systems (energies, luck) are opaque even with the how-to-play entries.
- **No production cascade** means fewer "numbers explode on their own" moments; the
  moment-to-moment feel is more spreadsheet, less firework.
- **UI ergonomics:** hand-rolled DOM with per-tick innerHTML rewrites; functional but
  visually rough compared to AD's Vue components, and modals/config live behind many
  small buttons.
- **Single-developer bus factor** shows: debug hooks in production, "please tell
  alemaninc" error popups, known memory-leak workarounds in the tick loop.

---

## 5. Summary

EMD is best understood as "Antimatter Dimensions rebuilt around formula composition
instead of production cascades", by a designer who treats softcaps, dilations, and
layered arithmetic as the primary content. Its prestige skeleton, challenge system, and
QoL ladder are straight from the AD playbook; its original contributions are the
transparency layer (stat breakdowns + formula view), achievement locking, banked offline
time, and the Study XIII challenge-builder endgame. Those four are the mechanics most
worth mining for an AD expansion, and none of them depend on EMD's flat-axis production
model.

---

*Document generated from source analysis on 2026-07-03.*
