---
date: 2026-07-10
feature: fidelity
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
---

# Fidelity pass — Eternity-stage fixtures

## Summary
A session working the fidelity suite from the first failing cell forward, one
issue at a time. A third fixture batch (late-game / Eternity-stage saves) landed
the grid at 369×4 = 1476 cells; this log records each fix and its pass-count
delta. Starting point: **1172/1476**.

## Fixes

### 1. Galaxy autobuyer resets its phase at the galaxy cap (`auto.galaxy.lastTick`)

*Fixtures 72, 73, 75, 76, 77 @ 1000 — first failing cells.* The only diverged
field was `auto.galaxy.lastTick`. All five are pre-Eternity saves with the
Galaxy autobuyer configured `limitGalaxies=true, maxGalaxies=1`. For 76/77
(already at 1 galaxy) Rust left `lastTick=0` (phase never reset) while JS reset
it to near-current time; for 72/73/75 Rust fired the galaxy ~700 ticks earlier
than JS.

Root cause: the original `GalaxyAutobuyerState.canTick` is
`Galaxy.canBeBought && requirement.isSatisfied && super.canTick` — it does **not**
test the `maxGalaxies` limit. `tick()` always runs `super.tick()` (which sets
`lastTick = realTimePlayed`, resetting the phase) and only *then* caps the
*purchase* via `requestGalaxyReset(_, limit)`. Rust instead folded a
`galaxy_limit_ok` gate into the autobuyer's `ready`, so at the cap the phase
never reset.

Two changes:
- `tick_autobuyers`: dropped `galaxy_limit_ok` from the Galaxy `ready`; the phase
  now resets on every ready tick even at the cap. The limit is applied only to
  the purchase — the non-buy-max branch now buys only while `galaxies < limit`
  (matching `requestGalaxyReset`'s early return), buy-max already clamps inside.
- `can_buy_galaxy` was also missing the `Galaxy.canBeBought` "past the Infinity
  goal" gate (`thisInfinity.maxAM > infinityGoal && (!break || inAntimatterChallenge)`)
  that the Dim Boost path (`can_dim_boost`) already models. Added it so the
  readiness (and the guarded purchase) match JS.

**1172 → 1186 (+14).**

### 2. Dimensional Sacrifice ignored the antimatter ceiling inside IC2

*Fixture 144 @ 1000.* AD 1–7 amounts were 0 in Rust but huge in JS, and
`sacrificed`/`chall8TotalSacrifice` diverged by ~295 orders. The save is running
Infinity Challenge 2 ("Dimensional Sacrifice happens automatically every 400 ms";
goal `1e10500`). At horizon 1000 antimatter has reached the goal exactly, so
production is frozen — but Rust kept sacrificing, zeroing the frozen dimensions,
while JS left them.

Root cause: `Sacrifice.canSacrifice` requires `Currency.antimatter.lt(Player
.infinityLimit)`, which `can_sacrifice` omitted. Inside an antimatter challenge
`infinityLimit` is the challenge goal (= `infinity_goal` there), so once
antimatter hits the goal both production *and* sacrifice must freeze. Added
`antimatter >= infinity_limit() ⇒ can't sacrifice`.

The ~295-order gap is one tick of production: with production frozen at the goal,
the last pre-freeze sacrifice set `sacrificed` and the dimensions never regrew, so
a single extra sacrifice on the final tick fully accounts for the divergence.

**1186 → 1187 (+1).**

### 3. Replicanti interval timer drifted a hair below the integer (f64 round-trip)

*Fixture 222 @ 1000* (dense-trace scan pinned the first divergence to tick 218,
field `replicanti.amount`). A tiny ~1e-3 log drift compounded across the whole AD
chain: the Replicanti amount feeds Replicanti Galaxies → `effectiveBaseGalaxies`
→ the tickspeed multiplier, which multiplies every dimension's production once per
chain step (hence the clean constant per-tier error increment, ~2e-4 log10 in
this fixture).

The Replicanti sub-interval timer was recomputed each game tick as
`(ticks - whole)·interval` with `ticks = (dt + timer)/interval`. That
`(total/interval)·interval` round-trip loses a little in f64 every non-growth
tick, so the timer drifts a hair below the exact integer and eventually crosses an
interval boundary one game tick late (here JS grows Replicanti at tick 218, Rust
at 219). The original computes the rollover in `Decimal`, staying on the clean
value.

Fixed by subtracting the consumed whole intervals directly:
`timer = total - whole·interval` (algebraically identical, no division
round-trip). The dense trace is now clean over all 1000 ticks.

**1187 → 1188 (+1).** (Fixtures 244/284 share the signature but diverge by more —
a separate/larger cause remains.)

### 4. Free Tickspeed upgrade applied to AD production one tick early

*Fixture 244 @ 100/1000* (trace scan: first divergence tick 11). Same constant
per-tier signature. The Time Dimensions produce Time Shards, which convert into
free Tickspeed upgrades (`totalTickGained`); a fresh Eternity fixture with 0
galaxies, so tickspeed is otherwise constant. `time_shards` and the grant tick
matched JS exactly — the divergence was *when* within the tick the grant took
effect.

The original's loop order (game.js) is `TimeDimensions.tick` → `AntimatterDimensions
.tick` → `totalTickGained += gain`: the free-tickspeed grant runs **after** AD
production, so an upgrade earned from this tick's shards only speeds up AD from the
*next* tick. Rust called `update_free_tickspeed()` inside `tick_time_dimensions`
(before AD production), so the extra ×1.1245 tickspeed hit AD production one tick
early — a ~0.05%/step drift that compounds through the whole AD chain.

Moved `update_free_tickspeed()` out of `tick_time_dimensions` to right after the
AD production block in `tick`. Time Dimension production already ran before it
(so TD still reads the old `totalTickGained`), matching JS.

**1188 → 1198 (+10).** (Fixture 284 still fails — a further cause remains.)

### 5. Replicanti timer rollover — do it in Decimal, mirroring JS (refines #3)

*Fixture 284 @ 100/1000* (trace: first divergence tick 16, `replicanti.amount`).
Another slow-Replicanti timing mismatch, but the *opposite* of #3: here JS grows a
tick *later* than Rust. Fix #3's exact `total − whole·interval` form matched #222
(where JS's timer stays on clean integers) but not #284 (where JS's timer drifts a
hair below the integer). The two aren't reconcilable with a single f64 formula.

The faithful port is to mirror the original's rollover *in Decimal*:
`tickCount = Decimal.divide(diff + timer, interval)`, then
`timer = (tickCount − floor(tickCount)) · interval`. Doing this with the `Decimal`
type reproduces JS's rounding, so the timer drifts (or stays clean) as JS's does.
This fixes the #284 class.

Residual: `00222` regresses. Rust's `break_infinity` `Decimal` divide/multiply
accumulates a rounding drift that JS's `Decimal` does not for interval 729, so its
timer lands a tick late there. That's a `break_infinity`-vs-JS `Decimal` precision
gap, not a Replicanti-logic bug — out of scope here.

**1198 → 1199 (+1 net; fixes the 284 class, `00222` regresses to a Decimal-port
precision residual).**

### Not fixed: pre-break "wall" unfreeze timing (`00258`–`00261`)

At the pre-break antimatter ceiling (`NUMBER_MAX_VALUE`, `1.7976931348623157e308`)
with the Big-Crunch autobuyer off, production freezes while the Tickspeed autobuyer
slowly drains antimatter (~`1e293`/purchase) until it dips back below the ceiling
and unfreezes for one explosive tick. The exact unfreeze tick depends on sub-ULP
`Decimal` arithmetic right at `1e308`, which differs between Rust's (i64-exponent)
and JS's (f64-exponent) `Decimal`. Rust unfreezes ~2 ticks early. Not cleanly
fixable without bit-matching `Decimal` at the boundary; left as a known residual.

### 6. `requirementChecks.reality.maxStudies` dropped on decode (Eternity block)

*Fixtures 301–368 (the new Eternity batch) @ all horizons.* Almost every new
fixture diverged on exactly one field: `requirementChecks.reality.maxStudies`
(JS=1, Rust=0, constant across horizons — a decode passthrough miss, not a tick
bug). The DTO had no `maxStudies` field and the decoder hardcoded
`reality_max_studies: 0`, so the saved peak-study count was lost on load (and the
re-encoded save read back 0).

Added `maxStudies` to `RealityChecksDTO`, wired it through the decoder, and emitted
it in the encoder (alongside `maxGlyphs`). The engine already maintained
`reality_max_studies` (bumped in `buy_time_study`); it simply wasn't persisted.

**1199 → 1371 (+172).** The single most impactful fix of the session — it cleared
the bulk of the new Eternity-batch failures.

### 7. Passive-IP `partInfinityPoint` frozen after an Eternity

*Fixtures 344+ @ all horizons.* `partInfinityPoint` was constant in Rust but grew
~5e-12/tick in JS. With the `ipGen` Infinity Upgrade bought,
`preProductionGenerateIP` does `partInfinityPoint += diff / (bestInfinity.time·10)`
**unconditionally**; the "too slow / never happened" cutoff
(`bestInfinity.time >= 999999999999`) only zeroes `gainedPerGen` (the IP granted
per whole gen), not the fractional accumulation. Rust gated the *whole* block on
`best < IP_GEN_TOO_SLOW_MS`, so a fresh Eternity — whose `bestInfinity.time` is the
`999999999999` reset sentinel — froze the fraction.

Reordered to always accumulate `part_infinity_point` (guarded only by
`gen_period > 0`) and to grant IP on a whole gen only when
`best < IP_GEN_TOO_SLOW_MS`.

**1371 → 1395 (+24).**

## Tests
- `cargo test -p ad-core --features serde` — all pass (578 + 22 + 29).
- Fidelity grid re-run after each fix; deltas recorded above.
- Final grid: **1395 / 1476**.
