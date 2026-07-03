# Exotic Matter Dimensions — Feature Decomposition

This document decomposes Exotic Matter Dimensions (𝕍1ω.16) into individual features in
dependency order, in the style of the Antimatter Dimensions feature decomposition. Each
feature lists its prerequisites, scope, and key formulas (taken from the source; `EM` =
exotic matter, `S` = stardust, `DM` = dark matter, `HR` = Hawking radiation, `MP` =
mastery power, `log` = log10 unless noted).

The game progresses through phases matching its in-game "EMD Level" indicator:

1. **Exotic Matter** — axes, masteries, offline time (EMD 1)
2. **Stardust** — first prestige, boosts, upgrades, stars (EMD 2)
3. **Dark Matter** — dark axes, dark stars (EMD 3)
4. **Energy** — exponential resource pairs (EMD 4)
5. **Wormhole** — second prestige, research, Studies (EMD 5)
6. **Light** — chroma and lumens (EMD 6)
7. **Galaxies** — star-cap prestige (EMD 7)
8. **Luck / Prismatic / Antimatter / Corruption** (EMD 8)
9. **Study XIII** — challenge-builder endgame (EMD 9)

A tenth layer ("Matrix", plus stubbed Spacetime/Eternity reset scopes) exists in the
save schema and Study XIII's final reward but is unreleased.

---

## Phase 1: Exotic Matter (EMD Level 1)

### Feature 1.1: Exotic Matter & Normal Axes

**Dependencies:** break_eternity-class Decimal type

**Scope:** Exotic matter accrues passively at a base of 1/second. Twelve axis types
(X, Y, Z, W, V, U, T, S, R, Q, P, O) are purchased with EM; only the first four are
available initially (more via Stardust Upgrade 1). Each axis type has a unique effect
and a unique cost curve with a closed-form inverse for buy-max.

**Axis effects (normal family):**

| Axis | Effect |
|------|--------|
| X | EM gain ×(effect) per level |
| Y | +к to the X axis effect (additive) |
| Z | EM ×, based on current EM |
| W | EM ×, grows with time |
| V | All normal axes cheaper |
| U | Stardust gain ×, based on unspent stardust |
| T | EM ×, based on total normal axes |
| S | **EM gain raised to a power** |
| R | Normal axis costs raised to a power < 1 |
| Q | Energy gain × |
| P | Y axis effect × |
| O | Effective levels of the first 11 axes × |

**Cost curves (base, before exponent/divisor reductions and softcaps):**
```
X: 5 × 6^n            V: 1e20 × 10^n           R: (e7.5e12)^(4/3)^n
Y: 100 × 1.5^T(n)     U: 1e100 × 10^(n^1.5)    Q: (e4e13)^(1.1^n)
Z: 1e6 × 10^(n^1.38)  T: 10^(10n + 180)        P: (e1.3e14)^(1.03^n)
W: 5e7 × 10^T(n)      S: (1.8e308)^(1.25^n)    O: layer-3 construction
   (T(n) = n-th triangular number)
```
Purchased levels pass through **linear scaling** (start `stat.axisScalingStart`) and
**semi-exponential superscaling** before the base curve; reductions apply in "hyper
order": cost `^ exponent` reductions, then `÷ divisor` reductions. Free axes (from
stars, dark axes, research) add to the effective level through a softcap.

**Production formula (stat pipeline):**
```
EM/s = 1 × X^levels × Z^levels × W^levels × T^levels × (achievements, masteries,
       stardust boost 1, stars 11-14, 42, research …)
       ^ S^levels ^ star41 ^ darkEnergy ^ (bindings …)
       → study dilations → × tickspeed
```

---

### Feature 1.2: Masteries

**Dependencies:** Feature 1.1 (first X axis)

**Scope:** ~34 free toggleable upgrades in 11 rows; initially one active per row
(row-mastery unlocks via stars/upgrades allow full rows). Effects scale with **mastery
power**, which grows as `masteryTimer ^ baseExponent` where the timer increases over
time and **resets to zero when a mastery is unassigned** — switching builds costs
momentum, not currency.

**Sample effects:** EM ×(MP+1)^0.1 (M11), axes cheaper (M12), free Z/W axes (M31/32),
stardust ×(M42), dark-axis cost ^ (M62), MP gain × from resources (M81–84), knowledge ×
(M103), star cost ^ <1 (M105).

**Boost sources:** knowledge effect (`1 + knowledgeEffect/100`), inter-row masteries
(M41/43/52 boost row 1), achievements, research, Study XIII bindings. Effects are
individually softcapped.

---

### Feature 1.3: Offline Time / Dilated Time

**Dependencies:** none (available from start)

**Scope:** Offline time banks 1:1 into **dilated time** (no offline simulation). Four
time states: normal / **Overclock** (spend DT for a game-speed factor; cost is linear
up to a softcap ~8×, then grows much faster) / **Freeze** (game paused, DT accrues) /
**Equalize** (every frame exactly 50 ms, excess banked — needed for timed achievements
and forced in Studies III/IX).

**Dilation Upgrades (4, unlocked at tickspeed 8× / 128× / 32768× / 2^31×):**
raise the Overclock cap (to `dB(n+18)`, max ~63×), delay its softcap, weaken the
softcap, and +% tickspeed. Costs are paid in dilated time (i.e. real seconds).

**Wormhole Amplification (Wormhole Milestone 20):** spend DT equal to
`timeThisWormhole × (mult − 1)` to multiply a Wormhole reset's HR yield by up to 2^slider.

---

### Feature 1.4: Tickspeed (derived stat)

**Scope:** Not a purchase — a multiplicative stat assembled from stars 71–74,
achievements, research, energies, milestones, Study XIII rewards, minus penalties
(Study VI slowdown, Galaxy Penalty 5). Multiplies all "per second" gains and the
truetime clocks, but never timers that would make it hurt (timed achievements).

---

### Feature 1.5: Achievements (9 tiers + secret)

**Scope:** 142 achievements in tiers 1–9 (visibility gated per tier), each with a
`check` tied to a dispatch event, most with rewards; ~40 secret achievements (rarity
tiers, jokes). **Per-tier aggregate rewards** scale with achievements owned in that
tier (e.g. T1: +0.02 X-effect each; T5: base knowledge × count; T7: first galaxy
penalty base `⌈10^(36/(17+A))⌉`; T9: Study XIII goal `min(999, 1032 − 4A)`).

**Achievement locking:** pinning an incomplete achievement to the progress bar makes
the game *refuse* actions that would fail it, via a declarative
`achievement.maxForLocks` table (axis caps, reset bans, research bans, star caps…).
Constraint achievements thus become enforced challenge runs. Some achievements have
**milestones** (repeatable sub-goals with growing rewards).

---

## Phase 2: Stardust (EMD Level 2)

### Feature 2.1: Stardust Reset

**Dependencies:** Phase 1; requirement 1e25 EM

**Scope:** First prestige. Resets EM, normal axes, mastery power/timer, (later:
first six energies). Keeps masteries, achievements, automation.

**Gain formula:**
```
pending stardust = dilate( EM / 1e24 , Study-IV reward exponent ≈ 0.5–0.55 )
                   × stardust multipliers ^ stardust exponents  (stat pipeline)
```
Stardust is *unspent-matters* currency: boosts key off current stardust; upgrades and
stars consume it. Last-10 runs and fastest/highest records are tracked.

### Feature 2.2: Stardust Boosts (12)

**Scope:** Passive bonuses scaling with unspent stardust; 2 unlocked initially, more
via Stardust Upgrade 3 (cap 12, +4 via Wormhole Milestone 11). Examples:
`#1 EM × (1+S/10)^0.5 × 10^(0.1·log(S+1)^1.5)`; `#4 stardust × MP^k`; `#5 X-axis base
price ratio ÷`; `#7 MP × log(S+10)^(s^0.5 softcapped)`; `#11 row-10 masteries stronger`;
`#12 +HR gain exponent`. Each has a `stardustBoostBoost` hook for meta-scaling.

### Feature 2.3: Stardust Upgrades (5 paths)

**Scope:** Five one-time-per-level upgrade paths with hand-authored cost ladders
(1.5e6 up to `e1.5e6`):

| # | Path | Per level |
|---|------|-----------|
| 1 | Dimensional | Unlock a new axis type (V, U, T, S, …) — 8 levels |
| 2 | Retention | L0: axis autobuyer; L1+: keep % of first n axes on reset — 13 levels |
| 3 | Boost | Unlock a new Stardust Boost — 10 levels |
| 4 | Mastery | L0: both row-1 masteries; L1+: new mastery rows — 5 levels |
| 5 | Progression | L0: Dark Matter; L1+: a new Energy type — 11 levels |

Costs are reduced by Wormhole Milestone 9 (dilate by `e^(−0.1·slog(HR/10+1))`),
achievements, white lumens, Study XIII effects. On Wormhole reset the paths clamp to
`[0,1,0,5,0]`.

### Feature 2.4: Stars

**Dependencies:** Feature 2.1

**Scope:** Stars cost stardust with a steep bespoke curve
```
cost = 2^( expScaling(superexpScaling(n, start≈25, power), 10, 0.5)^2 + 10 )^(1.5 if n≥10)
       × galaxy penalties, ^ and ÷ reductions (achievements, masteries, lumens, luck …)
```
Bought stars are **allocated** to star upgrades arranged in 10 rows × 4 columns; the
n-th star can only go in a fixed row (`starRow` lookup — rearranged inside Study II).
Upgrades include EM multipliers, free axes, "cube the effect of the star two above"
(31–34), full-row mastery unlocks (51–54, 101–104), +% game speed (71–74), cost-power
reductions (81–84), star-effect amplifiers (91–94). Respec = stardust reset. Hard cap
**60 stars** (→ Galaxies). Builds import/export as comma strings.

---

## Phase 3: Dark Matter (EMD Level 3)

### Feature 3.1: Dark Matter & Dark Axes

**Dependencies:** Stardust Upgrade 5 level 0 (5e11 stardust)

**Scope:** DM accrues passively from stardust: base
`S/1e12`, past 1e12 `dilate(S/1e11, 0.5)/10`, then `(x+1)^darkStar1 − 1`, multipliers
(dark X/Z/U/T axes, research), exponents (dark S), softcaps. Twelve **dark axes**
mirror normal axes with their own effects (dark X: DM ×; dark Y: dark axes cheaper;
dark W: MP ×; dark S: DM ^; dark P: dark-axis costs ^; …) and their own cost curves
(`10^(n^1.2+1)` for X up to layer-3 for O). Each dark axis also grants **free levels of
the corresponding normal axis** (the "normal axis boost", scaled by dark-star effect 3
and achievement tier 3).

### Feature 3.2: Dark Stars

**Dependencies:** Feature 3.1

**Scope:** Sub-prestige within the stardust layer. Requirement on **total dark axes**:
```
req(n) = ceil( (36 + 5.5n + n²/8 with expScaling past start) ^pow ÷ div − sub )
```
Gaining dark stars performs a Stardust reset **plus** resets DM and dark axes (until
Wormhole Milestone 7 removes both). Three effects per dark star:
1. DM base gain `(x+1)^(1 + ★·k/20) − 1` (boosted by Study IV reward 3);
2. Round-robin +10% effect to one specific dark axis per star (cycling X→S, softcapped
   per axis);
3. +% free normal axes from dark axes (`darkStarEffect3`, softcaps at 100/150→200).
Bulk buying supported (`maxAffordableDarkStars` closed form).

---

## Phase 4: Energy (EMD Level 4)

### Feature 4.1: Energies (10 types)

**Dependencies:** Stardust Upgrade 5 level 2+

**Scope:** Each unlocked energy **multiplies itself** every tick:
`E ×= energyPerSec^dt`, with
`energyPerSec = dilate(resource+10, 0.9) ^ (mult / divisor)` — pure exponential growth
keyed to a paired resource (EM, S, DM, X axis, MP, other energies, HR, knowledge, all
axes, tickspeed).

**Effect:** when energy exceeds its paired resource, that resource's production is
raised to a power:
```
eff = 1 + inc × log( log_resource(energy) ) × boost      (per-type inc 0.025–0.5)
```
softcapped per type; the 9th (dimensional) energy *divides* instead. First six energies
reset on Stardust reset (Study III reward keeps a root of them), all reset on Wormhole.
Study III inverts the whole system (energies ×1000–4000 faster but effects become
strong penalties `x^−1 … x^−20`).

---

## Phase 5: Wormhole (EMD Level 5)

### Feature 5.1: Wormhole Reset & Hawking Radiation

**Dependencies:** 1000 total dark axes

**Scope:** Second prestige; resets everything in Phases 1–4 including stardust,
stardust upgrades (to floor), stars, dark stars, energies. Grants **Hawking radiation**:
```
HR = floor( ( 2 ^ (ΣdarkAxis / 1500) ^ apexExp(2 + SB12) ) ^baseExp ×mults ^exps )
```
HR has **no innate effect**; it is spent on observations and Wormhole Upgrades, and
some milestones scale with it. First reset triggers a 16-second animation + story.

### Feature 5.2: Wormhole Milestones (30)

**Scope:** Driven by **Tier-5 achievement count** (not reset count). QoL ladder:
autobuyers for dark axes/dark stars/stardust upgrades/stars (1–4), auto star
allocation (5), dark stars keep DM (7), auto Stardust resets (8), star & stardust-upgrade
cost dilation from HR (9), +1 S/s flat (10), auto Wormhole resets (12), game speed per
achievement (13), knowledge multipliers (14–29 interleaved), Time Loop (20), instant
full pending stardust (30).

### Feature 5.3: Knowledge, Observations, Discoveries

**Scope:** After first Wormhole, **knowledge** accrues:
`knowledge/s = log(MP+1)/log(1.8e308) × T5-achievement count × milestone/mastery/etc. multipliers`.
Knowledge gives a softcapped % boost to all masteries
(`convergentSoftcap(log(log(K+10)) × 10, 0.75·cap, cap)`, cap 50%→ upgradeable) and
generates **Discoveries** at `⌊log10(K)⌋ (+adds, ×mults)`.
**Observations** (4 types, bought with EM/S/DM/HR at `base^((1+0.1n)^k)` power-tower
costs) multiply the knowledge effect.

### Feature 5.4: Research Tree (192 nodes)

**Dependencies:** Feature 5.3

**Scope:** Discoveries buy research in a 47-row × 15-column tree; a node needs ≥1
purchased neighbour from `adjacent_req`, optional `condition`s (study completions,
lumens, …), and its cost. Three types: **normal** (refunded on Wormhole respec),
**permanent** (white border, priced in *total* discoveries, survives respec), **study**
(red border — unlocks a Study; refunded unless in use).

**Research groups** add intra-group interference: Energy/Stardust research double the
group's costs per owned; Chromatic ×4 per owned in the same row; Spatial Synergism has
a purchase cap (6 + 2·T8 achievements); Luck/Antimatter research lose 7%/9%
effectiveness per owned; Study-V groups (Theoretical/Practical) must stay balanced;
Prismal capped by a prismatic upgrade; Time capped by Study VI completions; 12
"Finality" chains with per-chain cost growth. `researchPower(row,col)` centralises all
effectiveness modifiers. Nine save-able loadouts; "projected respec cost" calculator;
auto-rebuy of a saved build after Studies.

### Feature 5.5: Studies I–XII (challenges)

**Dependencies:** study-type research nodes

**Scope:** Entering a Study wormhole-resets and applies **Bindings** (restrictions).
Completing = reaching the Study's goal in **total dark axes**, then Wormhole resetting.
Up to 4 completions each, with escalating unlock requirements, harsher bindings, higher
goals; each grants 3 scaling rewards (reward 2 boosted by Study X reward 4 and Binding
265; reward 3 by red lumens and per-study achievements).

| # | Name | Binding (level 1) | Goal 1 | Sample rewards |
|---|------|-------------------|-------:|----------------|
| I | Autonomy | Stardust/Automation tabs + hotkeys locked | 1,000 | empower Y axes; boost SB2; HR × |
| II | Big Bang Theory | Star costs ↑↑, forced buy order, unspent stars act as dark stars | 800 | Star Scaling −%; row-9 stars ^; free dark stars |
| III | Analgesia | Energies ×1000 faster but effects become x^−1…−20 penalties | 2,000 | +SD-upgrade-5 cap; keep energies^k on reset; meta energy faster |
| IV | Vacuum Decay | Each Stardust reset → stardust gain ^0.5 for rest of Study | 3,200 | base stardust exponent ↑; M42 ×; dark-star eff 1 × |
| V | Scientific Illiteracy | Research respec + all research costs ×32…×10,000 | 4,000 | Theoretical/Practical efficiency; observation costs ^; −research cost |
| VI | Event Horizon | Game 10^27…10^999 × slower, easing toward goal | 4,500 | tickspeed→chroma ^; tickspeed^k→knowledge; R8-2 stronger |
| VII | Luck Be In The Air Tonight | Gains ^0–1 oscillating with luck essence (cosine, per-reset gain) | 6,227 | empower dark W; Crystal research eff; **luck shards/s (unlocks Luck)** |
| VIII | Masterful | One mastery per row enforced; dark-axis cost ceiling `(MP+1)^88` | 5,888 | knowledge cap ↑; M85 per T8 ach; +real time to mastery timer |
| IX | Scientia est Experientia | Gains → `10^(log(gain)^0.5−)`; auto-resets every 9 s, banks experientia from dark stars | 999 | **antimatter gain (unlocks Antimatter)**; Galaxy Penalty 3 −; Spatial Synergism + |
| X | Study of Studies (Triads) | Applies 3 Studies at once — Stellar (I+IV+VII), Decisive (II+V+VIII), Temporal (III+VI+IX), Ontological (pick any 3) | varies | U-axis × per achievement; anti-T +; tickspeed→energy; **all Studies' reward 2 ×** |
| XI | Lunar Clock | Only one axis type active, rotating every 750 ms; row-1 masteries/stars & SB1/4 off | 11,800 | axis superscaling later ×3 |
| XII | Titanium Will | Non-permanent research off; **Stardust resets disabled**; DM gain capped at 1 (softened by Titanium Empowerments bought with EM) | 40 | boosts tied to Iron-Will achievements |

While inside a Study, its effects are enforced by ~70 scattered `StudyE(n)` checks.
Studies completed unlock further research (their reward research nodes).

### Feature 5.6: Wormhole Upgrades (12)

**Scope:** Late-phase HR sinks (costs `e3000`–`e45678`): HR gain ×/^ scaling with
upgrades owned, better galaxy formulas, higher mastery-101 softcap, anti-Y multiplier
from study completions, gray-lumen base +, stardust-upgrade cost relief, halved
autobuyer interval caps, and three repeatable ones (all-axis costs ^0.99/level, star
costs ^0.9/level, passive stardust `log(pending+10)^(n/20)`).

---

## Phase 6: Light (EMD Level 6)

### Feature 6.1: Chroma & Lumens

**Dependencies:** research r8_8 (tiers extend at r10_5, r11_8, r19_8)

**Scope:** Nine colours: red/green/blue generate freely; cyan/magenta/yellow convert
two primaries; white/black convert from {cyan,magenta,yellow}; gray from {white,black}.
Base generation `1/s at 60 stars, ÷3 per star below 60` (galaxy boost 2 counteracts),
one colour generated at a time (achievement 815 later parallelises).
**Lumens**: chroma converts at exponential thresholds (`baseReq × baseScale^owned`,
scale 1.1–1e10 by colour, reducible via prismatic "Illumenati" upgrades). Lumen
effects: red — Study third-rewards stronger; green — S axis→T axis multiplier; blue —
HR base gain ^; cyan — R7-5 affects knowledge; magenta — +MP base-gain exponent;
yellow — upgrades specific achievement rewards at breakpoints; white — star and
stardust-upgrade costs ^ <1; black — chroma cheaper; gray — chroma gain ×1e5^L
(softcapped). Chromatic/Photonic research rows deepen the system.

---

## Phase 7: Galaxies (EMD Level 7)

### Feature 7.1: Galaxies

**Dependencies:** research r12_8; 60 stars (the cap)

**Scope:** At the 60-star cap, gaining a Galaxy **forces a Wormhole reset** and
permanently raises star costs, in exchange for boost/penalty pairs (both scale with
galaxies past per-pair unlock counts 1/2/4/6/10):

| # | Boost | Penalty |
|---|-------|---------|
| 1 | Row-1 stars stronger | Star base cost ^(T7-achievement base)^G |
| 2 | +chroma gain per star | Stardust gain ^0.99^G per star below 40 |
| 3 | U-axis effect + per star | Each star × star cost by 1.8e308^(G^3) |
| 4 | Prismatic base gain ↑ | Star Scaling stronger per star above 40 |
| 5 | Assigned stars help Study II reward 3 | Game slower per unassigned star below 20 |

Galaxies can be voluntarily destroyed (also a Wormhole reset). `highestGalaxies` is
tracked for effects that shouldn't punish destruction.

---

## Phase 8: Luck, Prismatic, Antimatter, Corruption (EMD Level 8)

### Feature 8.1: Luck

**Dependencies:** Study VII completion

**Scope:** **Luck shards** accrue passively (`studies[7].reward(3)` per second, HR-based),
with two innate effects (a % boost and a `log log`-shrinking penalty reducer). Shards
buy **runes** in five folium types (trifolium/quatrefolium/cinquefolium via research;
uni/duo via Study XIII), geometric costs. Unspent runes buy **luck upgrades** —
cost-curve benders (axis/star/dark-star/observation costs ^<1 or ÷, Spatial-Synergism
efficiency, chroma/radiation/prismatic boosts) — with a **cascade** graph: levels of
one upgrade add levels to those "below" it. All refundable (full respec = Wormhole reset).
Percentage-based spend throttles for automation.

### Feature 8.2: Prismatic

**Dependencies:** research r20_8

**Scope:** **Prismatic** generates from all lumen counts (galaxy boost 4 sets the base
`(x+1)^0.5−1`). Upgrades come in *unlimited* (geometric cost; e.g. Prismatic/Chromatic
Amplifiers) and *limited* (bespoke cost formula + max; e.g. Illumenati I–III lumen
threshold reducers, Prism Rune, Prism Lab enabling Prismal research, Prism Condenser
granting free anti-axes, Lab Amplifier, Master Spark) variants; upgrades with drawbacks
(Chromatic Overdrive, Master Spark) are **refundable**. Bulk-buy uses geometric-series
math or bounded binary search.

### Feature 8.3: Antimatter & Anti-Axes

**Dependencies:** Study IX completion

**Scope:** Antimatter accrues passively (its own stat pipeline; Study IX's reward
raises it; anti-S raises it to a power; antimatter galaxies ^ it). Twelve **anti-axes**
(first 4 free; V/U/T/S via research; R/Q/P/O via Study XIII "slabdrill") mirror the
other families with effects like: anti-X antimatter ×; anti-Y luck/prismatic/AM ×;
anti-T adds to observation effect; anti-Q anti-axis costs ^; anti-O meta-level ×.
Every anti-axis also gives a **dimension boost** multiplying the *effective levels* of
the corresponding normal and dark axes:
`(ln(n·pow/1000 + 1) + 1)^0.9`.

**Antimatter Galaxies** (Study XIII reward at 24 completions): cost `(1e5000)^(1.2^n)`
AM, bulk-buyable, never reset; give free dark stars (= count), AM gain
`^(1+G/1000)^10`, and +0.2% game speed per normal galaxy each.

### Feature 8.4: Corruption

**Dependencies:** reaching cost thresholds (auto-unlocks, permanent)

**Scope:** Cost walls on the three axis families and dark stars: past
`ee15 / ee12 / ee9 / e9` respectively, base costs map by
`start^((log(x)/log(start))^p)` with `p`-driven effective powers 256/64/64/16.
Study XIII's "Evil-Sealing Circle" reward pushes the normal/dark starts later. Framed
in-story as the world resisting further expansion.

---

## Phase 9: Study XIII (EMD Level 9 — current endgame)

### Feature 9.1: Binding Tree

**Dependencies:** research r44_8

**Scope:** A freeform challenge builder. ~87 **Bindings** in a dependency tree (all
parents required, unlike research); each toggled binding applies a nerf (dark-axis
costs ^, star costs compounding, specific research weakened, mastery rows weakened,
timers, meta-bindings amplifying other bindings, the extreme Binding 25:
`x → log^[1.4](x)` on all core gains, worth 56 levels) and contributes **binding
levels**. Entering Study XIII applies every active binding; completing it (goal =
`min(999, 1032 − 4·T9-achievements)` total dark axes; Binding 236 adds a 9-second-ish
time limit variant) sets `completions[13] = max(current, total binding level)` — bulk
completion in one run. Cap 200 (256 in beta). The Study's display name is procedurally
generated from the strongest active bindings.

### Feature 9.2: Named Rewards

**Scope:** ~24 rewards unlock/upgrade at completion **breakpoints**, in three shapes —
`scaling` (each level supersedes: Exo/Dark/Anti Reactors boosting axis effects,
knowledge base ×, binding weakeners "Mailbreaker", white-lumen→stardust-cost effect,
gray-lumen softcap delay, corruption delays), `composite` (each level adds a distinct
effect: mastery boosts, game-speed "Pocket Watch", ×100-per-completion "Centennial"),
and `single` (unlocks: 3 new luck upgrades, 2 prismatic upgrades, antimatter galaxies,
extra research rows, and at 256 — **"Unlock the ability to create a new Matrix"**, the
next, unreleased layer).

---

## Cross-Cutting Systems

### Automation
Five interval autobuyers (axis / dark axis / stardust upgrade / star / research), each
with geometric-cost interval upgrades (−5%/level to a softcap, then −1% to a 0.1 s
floor) paid in a thematically-matched resource; per-target cap inputs. Threshold
automators for Stardust and Wormhole resets with 6 trigger modes (amount, time,
× / ^ of current or previous gain). Star allocator replaying saved builds. All
automation survives every reset.

### Stat pipeline & formula transparency
Every derived quantity is an ordered modifier pipeline (base → multipliers → exponents
→ dilations → tickspeed) that simultaneously drives the simulation, the Stat Breakdown
tab, "current → next" projections, and the global formula-display hotkey. Any port of
this game should treat the pipeline as the core engine abstraction (see the codebase
report §5).

### Softcap vocabulary
Reused parametric curves: linear, logarithmic, convergent (asymptotic), semi-exp /
semi-log scaling pairs, `dilate`, and layer shifts — each with an exact inverse. Balance
knobs are usually the *parameters* of these curves.

### Records & prestige history — nested-reset-scope parameterisation
Last-10 runs and fastest/highest/most-efficient records per layer, with stored **builds**
(masteries/stars/research/luck at the time) viewable per record.

The underlying pattern deserves attention: counters, clocks, and records are all
**indexed by reset scope**, including scopes for layers that don't exist yet
(Stardust ⊂ Wormhole ⊂ Spacetime ⊂ Eternity — the last two belong to the unreleased
Matrix content). Three pieces:

1. **Currency accumulators.** Every major currency is a family of variables, one per
   enclosing scope plus an all-time total, e.g. `exoticmatter`,
   `exoticmatterThisStardustReset`, `exoticmatterThisWormholeReset`,
   `exoticmatterThisSpacetimeReset`, `totalexoticmatter`. Gains go through a list of
   variable names (`for (i of exoticmatterVariables) o.add(i,x)`) so all scopes update
   uniformly, and each reset function zeroes exactly the accumulators at or below its
   scope — `wormholeReset()` clears the `ThisStardust` and `ThisWormhole` families and
   leaves `ThisSpacetime` accumulating.

2. **Clocks.** `timePlayed / timeThisStardustReset / timeThisWormholeReset /
   timeThisSpacetimeReset`, each with a tickspeed-adjusted `truetime…` twin, plus
   `fastest{Stardust,Wormhole,Spacetime}Reset` and even `bestTickspeedThisMatrix`.

3. **Run records keyed by (run layer, enclosing scope).**
   `previousStardustRuns = {last10, wormhole:{fastest,highest},
   spacetime:{…}, eternity:{…}}` — the best stardust run *within* each enclosing
   scope. `previousPrestige.generate(resLayer, inLayer, base)` snapshots a run with
   detail proportional to the enclosing scope's depth (masteries → +stars →
   +research/luck): records that survive longer store richer builds.

**Why the scoped values diverge (they are odometers, not wallet copies).** The scoped
variables are not per-layer copies of the currency's value; they are statistics over
the same income stream with different reset lifetimes, and they diverge from the wallet
for two reasons:

- **Spending.** The wallet (`g.exoticmatter`) decreases on purchases; the scoped
  accumulators only ever increase (`incrementExoticMatter` adds gains to all of them,
  while `o.sub("exoticmatter", cost)` touches only the wallet). This makes them the
  right operand for thresholds that must be robust to spending: the Stardust reset
  button is revealed by `exoticmatterThisSpacetimeReset ≥ 1e25`
  (`masteryData[42].req()`), so spending EM on axes at the threshold cannot make the
  unlock flicker away.
- **Inner resets.** The scopes diverge from *each other* as soon as an inner layer
  resets: after three Stardust resets inside one Wormhole, `…ThisStardustReset` covers
  only the current run while `…ThisWormholeReset` sums all four. Each scope has its own
  consumers — per-run effects and achievements (Stardust Boost 7 scales with
  `truetimeThisStardustReset`; "produce X in a single reset" achievements), per-Wormhole
  formulas and records (Study III reward 3 uses `truetimeThisWormholeReset`; Wormhole
  efficiency = HR ÷ `timeThisWormholeReset`), and monotone all-time totals that must
  never decrease (EMD Score and story text read `totalexoticmatter`).

Antimatter Dimensions needs and has the same variables, just under different names and
shapes: `records.totalAntimatter`, `records.thisInfinity.maxAM`,
`records.thisEternity.maxAM`, `records.thisReality.maxAM`, consumed by the post-break
IP formula (peak antimatter this infinity, deliberately immune to spending/crunch
timing), achievements, and the statistics screens. AD tracks a *max* where EMD tracks a
*sum* — a different statistic chosen for the same spending-robustness motive.

The difference is uniformity, and that is the payoff: in EMD every currency gets the
same scope family, maintained by one shared increment helper and cleared by scope on
reset — so when the Matrix layer ships, its reset function only needs to zero the
`ThisSpacetime` family and roll the `spacetime` record slots; every statistic has been
accumulating correctly since day one and old saves need no migration. AD's equivalents
are scattered across differently-shaped, hand-updated `records.this*` objects, and
forgetting to update or reset one of them is a recurring AD bug class (several
changelog entries concern record stats surviving resets they shouldn't have).

**Suggestion for the Rust engine:** model a currency as a wallet plus a
`ResetScope`-indexed array of statistics (sum and/or max as the consumers require),
updated in one place on gain and cleared by a generic `on_reset(scope)` for everything
at or below that scope — so a hypothetical fourth prestige layer becomes one new enum
variant rather than a schema change and a hunt through bespoke record structs.

### EMD Level & Score
A spoiler-free progression indicator (level 1–9 plus a 0–1,000,000 score computed from
weighted per-level factors), used by the community save bank; explicitly excluded from
gameplay effects.

---

## Implementation Order Summary

```
Phase 1: 1.1 Axes → 1.2 Masteries → 1.3 Offline Time → 1.4 Tickspeed → 1.5 Achievements
Phase 2: 2.1 Stardust Reset → 2.2 Boosts → 2.3 Upgrades → 2.4 Stars
Phase 3: 3.1 Dark Matter/Axes → 3.2 Dark Stars
Phase 4: 4.1 Energies
Phase 5: 5.1 Wormhole/HR → 5.2 Milestones → 5.3 Knowledge/Observations/Discoveries
         → 5.4 Research Tree → 5.5 Studies I–XII → 5.6 Wormhole Upgrades
Phase 6: 6.1 Chroma & Lumens        (parallel with late Phase 5)
Phase 7: 7.1 Galaxies
Phase 8: 8.1 Luck → 8.2 Prismatic → 8.3 Antimatter → 8.4 Corruption
Phase 9: 9.1 Binding Tree → 9.2 Named Rewards → (Matrix, unreleased)
```

### Estimated scope

| Phase | Features | JS reference lines (approx.) |
|-------|----------|------------------------------|
| 1 | 5 | ~3,500 (axes/masteries/offline/achievement core) |
| 2 | 4 | ~1,800 |
| 3 | 2 | ~1,000 |
| 4 | 1 | ~400 |
| 5 | 6 | ~6,500 (research tree + studies dominate) |
| 6 | 1 | ~900 |
| 7 | 1 | ~400 |
| 8 | 4 | ~2,500 |
| 9 | 2 | ~1,600 |
| cross-cutting | — | ~4,000 (stat pipeline, automation, records, UI glue) |

---

*Document generated from source analysis on 2026-07-03.*
