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

## Tests
- `cargo test -p ad-core --features serde` — all pass (578 + 22 + 29).
- Fidelity grid re-run after each fix; deltas recorded above.
