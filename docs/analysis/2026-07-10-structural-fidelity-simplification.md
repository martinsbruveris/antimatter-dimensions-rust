---
status: Reference
---

# How much simpler would the code be under structural-only fidelity?

A speculative exercise (2026-07-10): if the requirement were relaxed from
**exact fidelity** (bit-compatible progression against the JS oracle, verified
by the save-replay harness) to **structural fidelity** (same features, same
progression curve, same gameplay — but free to diverge on floating-point
detail, edge-case timing, multiplication order, and per-caller buy semantics),
how much code and complexity would fall away?

This is analysis only; nothing here is planned work. Line counts are from the
codebase as of this date (ad-core 42.7k lines, ad-gui backend 6.0k,
ad-fidelity 3.4k).

---

## 1. The headline: what is exact-fidelity code, really

The essential observation up front: **most of ad-core is irreducible game
content.** The multiplier formulas, upgrade tables, achievement conditions,
study tree, challenge modifiers, celestial mechanics — ~30k of the 42.7k
lines — *are* the gameplay, and structural fidelity keeps every one of them.
Many things that look like "quirks" are load-bearing balance:

- The unclamped ≥3-galaxy tickspeed branch is worth ~e1888 of tickspeed on
  deep-galaxy saves (fidelity fix, 2026-07-09).
- IC3 neutralising only the *per-purchase* tickspeed multiplier (not the base
  achievement effects) changes IC3 strategy.
- `chall8TotalSacrifice` accumulating on every sacrifice, even outside NC8,
  feeds later NC8 runs.

None of that can be simplified away without changing the game. The genuine
exact-only surface is concentrated in four places:

| Subsystem | Today | Structural-fidelity version | Δ |
|---|---|---|---|
| Save codec (`save/`: dto+encode+codec+template) | 7,734 lines | `serde_json::to_string(&state)` — the serde derives **already exist** (70 `cfg_attr`s in state.rs alone) | **−7.6k** |
| ad-fidelity harness (+oracle/capture JS) | 3,364 lines | deleted or archived; replaced by invariant/milestone tests | **−3k** (replaced by ~1k of new tests) |
| Per-caller buy/prestige paths + autobuyer timer machinery | ~1.2k lines | unified parametrized paths, plain periodic timers | **−0.6k** |
| Precision mirroring (JS RNG, Decimal-rollover timers, sentinels, JS int32 semantics) | ~0.3k lines + scattered sites | idiomatic Rust (`Option`, any RNG, f64 timers) | **−0.2k** |

Plus the second-order effect in ad-gui (§4): with unified buy semantics the
Action IR *can* become the single seam, collapsing ~90 game commands and their
~90 store dispatchers into one generic `apply(Action)` channel (**−2k** across
commands.rs + stores/game.js).

**Total: roughly 12–14k lines (~25% of the workspace's hand-written code) and
— more importantly — the removal of the two most demanding invariants
maintainers currently carry: exact tick-order and exact save-format.**

## 2. Subsystem detail

### 2.1 The save codec is the single biggest item (−7.6k)

`save/dto.rs` (3,281) + `encode.rs` (1,815) + `codec.rs`/`bundle.rs`/`mod.rs`
(779) + the 1,859-line vendored `default_player.json` template exist *only* to
read and write the original game's exact save format (AAB container, the
`player` tree, mixed-type tuples, capitalized keys, sentinel encodings).
`GameState` already derives Serialize/Deserialize throughout — a native format
is one line. The codec is also where a steady stream of passthrough bugs lives
(three of this week's fidelity fixes — `maxStudies`, `gameCreatedTime`,
`statTabResources` — were codec drops).

**The catch:** wire compatibility is itself a feature (import your real save;
it also powers the fidelity harness). This axis is separable from float
exactness — one could keep the codec for *import only* (~3.3k lines of dto.rs)
and save natively, halving the cost while keeping migration.

### 2.2 Unified action semantics (−0.6k in ad-core, unlocks §4)

Today each mechanic has up to four faithful buy paths with distinct gates:

- Dimensions: `buy_dimension` (15) / `buy_until_10_dimension` (29, atomic
  group-buy — itself a fidelity fix for NC4+NC6) / `buy_max_dimension` (6) /
  `buy_max_dimension_bulk` (83, with NC9/IC5 purchase-by-purchase branches)
- Tickspeed: `buy_tickspeed` (23) / `buy_max_tickspeed` (46, the closed-form
  `getMaxBought` that deliberately charges only the top purchase) +
  `cost_scaling.rs` (115, the super-exponential regime port)
- Galaxy: `buy_galaxy` (22) / `max_buy_galaxies` (47) — plus the subtlety that
  the autobuyer's `maxGalaxies` cap gates the *purchase* but not the *timer
  reset* (fidelity fix 2026-07-10)
- Dim Boost: single vs `max_buy_dim_boosts` with a different gate set

Structurally, each mechanic needs *one* parametrized purchase (`buy(n)` with
`n ∈ {1, to-group, max-within-budget}`) and affordability = cumulative cost —
no "charge only the most expensive" quirk, no per-caller `canTick` variants.
`tick_autobuyers` (350 lines of per-autobuyer ready-gate transliteration)
becomes a table of `{timer, gate, action}` entries at maybe half the size, and
the `lastTick`/`timeSinceLastTick` elapsed-phase conversion + prestige
`resetTick` machinery (~100 lines + codec support) becomes a plain periodic
timer.

### 2.3 Tick ordering: freedom, not fewer lines

`tick.rs` carries 32 ordering-constraint comments ("the original runs X before
Y"); `autobuyers.rs` 39 more. Structural fidelity wouldn't shrink the tick loop
much, but it would remove the *invariant*: producers could be reorganized into
clean phases (inputs → automation → production → records) without one-tick
offsets being bugs. Note the flip side in §3 — those one-tick offsets are also
what makes the current test harness so sensitive.

### 2.4 Precision mirroring (−0.2k, plus ongoing sanity)

- The glyph RNG mirrors JS bit-for-bit (`js_to_int32`, `xorshift32`, ~80
  lines): any seeded RNG would do structurally.
- The Replicanti interval timer must roll over in `Decimal` to reproduce JS
  rounding drift (fidelity fix 2026-07-10) — an f64 timer is fine
  structurally.
- 52 sentinel sites (`999999999999`, `f64::MAX` best-times) become `Option`.
- The 7 permanently-diverged fidelity cells (sub-ULP `Decimal` behavior at the
  `1e308` wall, i64- vs f64-exponent rounding) stop being anyone's problem.

## 3. What it costs — the part that matters

**Exact fidelity is the project's test oracle, and it is far more valuable
than it looks.** The save-replay harness (369 fixtures × 4 horizons, 1e-4
tolerance) works *because* the engines are bit-compatible: any divergence,
however small, is a bug signal. Of the 45 count-changing fixes in the fidelity
tracker, the large majority were **real behavioral bugs** — missing gates,
dropped save fields, wrong formulas, misordered updates — that surfaced as
tiny numeric drifts long before a player would notice. Under structural
fidelity:

1. **Save-replay diffing dies.** One-tick offsets compound exponentially (a
   single tick at late-game growth rates is orders of magnitude), so replaying
   a save through both engines produces uncomparable states within seconds of
   game time. There is no tolerance setting that distinguishes "structurally
   fine" from "subtly wrong".
2. **The replacement is much weaker.** Invariant tests (monotonicity,
   conservation, gate consistency) plus progression-milestone tests ("a fixed
   strategy reaches Eternity in X game-hours ± 20%") catch gross breakage but
   not balance drift — a mistranscribed exponent that makes a layer 3× faster
   passes both.
3. **It is a one-way door.** Once behavior diverges, re-verifying any future
   port work against the JS game requires re-establishing exactness first.
   The endgame-mod goal (porting a *second* JS codebase on top) would inherit
   the weaker safety net for all shared mechanics.

There is also a smaller, concrete feature loss if the codec goes: importing
real saves (and the capture-based workflow used to build the fixture library).

## 4. The Action-surface revisit

The 2026-07-10 architecture review concluded the Action IR should *not* become
the universal seam, and the decisive argument was per-caller semantics — an
exact-fidelity constraint. Under structural fidelity that argument dissolves,
and the design the original simulation doc sketched becomes natural:

- One `Action` enum (~60–80 variants after unification — options setters and
  save/editor operations stay out) with `apply_action` as the **only**
  mutation path for GUI, autobuyers, Automator, and sim alike.
- ad-gui: ~90 game commands + ~90 store dispatchers collapse into one
  serializable `apply(action)` command (+ the view/save/meta commands).
  `commands.rs` ≈ 2.1k → ~0.6k; `stores/game.js` ≈ 0.9k → ~0.3k.
- ad-sim/ad-python get the full vocabulary for free, and an action log becomes
  a natural replay/debug format — partially compensating for the lost
  save-replay harness.
- `ObservedState` and `GameView` could merge into one snapshot type with two
  serializations.

## 5. Verdict

Structural fidelity would buy roughly a quarter less code, the collapse of the
two most delicate invariants (tick order, save format), a genuinely unified
action architecture, and an end to sub-ULP chase work. It would cost the
project its extraordinarily sensitive test oracle, real-save import, and easy
comparability for all future porting (including the endgame mod) — and the
majority of the codebase (the content) would not get any smaller.

Given that the fidelity harness is what made this week's bug-hunting tractable
and the endgame-mod port is still ahead, the current prioritization looks
right *for the porting phase*. The interesting future option is **sequencing**:
finish the port under exact fidelity (keeping the oracle sharp), then — if the
project pivots from "port" to "platform" (numerical experiments, mods,
balance variants) — take the structural-fidelity simplifications as a
deliberate v2, using the still-exact engine as its own oracle for the
transition (replay both, compare at milestone granularity rather than
per-tick). That ordering banks nearly all of the simplification value without
ever being blind during a rewrite.
