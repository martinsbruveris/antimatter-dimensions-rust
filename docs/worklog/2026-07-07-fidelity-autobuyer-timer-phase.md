---
date: 2026-07-07
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
  - ../design/2026-07-03-autobuyers.md
---

# Fidelity fix — autobuyer timer phase (`lastTick`) on load + `advance` ordering

## Summary
Tracing fixture `00003-0003-47-42-timed` (dense per-tick trace `t.json`) showed a
first divergence at tick 140: the 1st Antimatter Dimension autobuyer fired a
buy-until-10 (bought 190→200) three ticks earlier than the JS oracle (tick 143).
That single mistimed purchase cascaded into all five reported field diffs. Root
cause was **not** floating point — it was the autobuyer timer phase. Two fixes
took the trace to a clean 1000 ticks and the grid from 35/312 to 38/312.

## The investigation
The `bought` field (a discrete counter) diverging was the tell: 190 vs 200. The
other four diffs were downstream of one early purchase:

- `antimatter` 100× lower — the buy drained ~1e59 of current AM at tick 140.
- `records.thisInfinity/thisEternity.maxAM` −0.1% — Rust's peak froze one tick
  lower (it bought just before the next rise).
- `records.totalAntimatter` +0.1% — buying 190→200 crosses a buy-10 boundary,
  **doubling** AD1's `2^(bought/10)` multiplier, so Rust's cumulative production
  jumped one tick ahead (Rust total@140 ≈ JS total@141). The ~4.5e-4 log-deltas
  are exactly one tick of growth, not drift.

Decoding the save: AD1 autobuyer `interval=500`, `lastTick=13662147.999`,
`realTimePlayed=13662576.999` → 429 ms of phase already elapsed at load. The JS
oracle preserves that (`timeSinceLastTick = realTimePlayed − lastTick`), firing
at ticks 3, 13, …, 143. Rust's decoder reset the timer to 0, firing at 10, 20,
…, 140. An instrumented replay pinned it exactly: `timer_ms` init 0 → buys @140,
`realTimePlayed − lastTick` (429) → @142, `… − dt` (379) → @143 = JS.

## What shipped

### 1. Save codec converts `lastTick ↔ timer_ms` (dto.rs / encode.rs)
The JS interval autobuyers store `lastTick` as an absolute `realTimePlayed`
timestamp; we model the timer as elapsed time. Decode now sets
`timer_ms = (realTimePlayed − lastTick).max(0)` for the 8 AD, tickspeed, dim
boost, galaxy, and big-crunch autobuyers (added `last_tick` to `AutobuyerDTO`,
`BigCrunchAutobuyerDTO`, `PrestigeAutobuyerDTO`, all `#[serde(default)]` so a
save omitting it defaults to 0 = the JS default). Encode writes the inverse,
`lastTick = realTimePlayed − timer_ms`. This alone moved the trace's first
divergence 140 → 142. The old code deliberately reset `lastTick` to 0 — a
mistaken simplification that desynced every autobuyer's firing phase on replay.

### 2. `Autobuyer::advance` matches `IntervaledAutobuyerState` ordering
JS `canTick` compares `realTimePlayed − lastTick >= interval` using the
`realTimePlayed` *before* the game loop advances it, and `tick()` sets
`lastTick = realTimePlayed` (phase → 0, overshoot discarded). `advance` now
tests the carried-over phase *before* adding this tick's `dt`, and resets to 0
on a fire instead of carrying the remainder (`timer -= interval`). This closed
the last tick: the trace now matches JS over all 1000 ticks.

A consequence of the check-before-add ordering: a fresh timer (phase 0) in a
fresh game (`realTimePlayed = 0`) no longer fires on its first tick — it must
accumulate a full interval first, which is exactly what JS does. When an
autobuyer is unlocked mid-game (`realTimePlayed` already large, `lastTick = 0`),
the huge `timeSinceLastTick` still fires it immediately.

## Test fallout (expected, per the plan)
Twelve autobuyer tests assumed the old add-before-check semantics (a fresh timer
firing on the first `tick(interval)`). Updated to the faithful model:
- The two raw-timer tests (`test_autobuyer_fires_after_interval`,
  `..._timer_resets_after_firing`) now assert the warm-up + reset cadence.
- Behaviour/counting tests pre-arm the timer to a full interval (a new `arm`
  helper; the autobuyer is "exactly due"), isolating the *buy* behaviour from the
  warm-up. Pre-arming restores the intended fire counts (e.g. 10 fires over
  5000 ms at a 500 ms interval).

## Verification
- `trace t.json`: first divergence 140 → 142 (fix 1) → **none over 1000 ticks**
  (fix 2).
- Fidelity grid: 35/312 → **38/312** cells, no regressions (stash A/B compared).
- `cargo test -p ad-core --features serde`: 507 pass. `-p ad-fidelity`: pass.
- fmt + clippy clean on the four touched files.

## Follow-up (same session) — readiness-gated timer reset
The latent difference noted below turned out to matter for the broader grid, so
it was fixed in the same session. Every interval autobuyer's JS `canTick`
overrides gate on action-readiness (AD/Tickspeed `isAvailableForPurchase &&
isAffordable`, Dim Boost/Galaxy `canBeBought && requirement.isSatisfied`, Big
Crunch `Player.canCrunch`), and `tick()` (which resets `lastTick`) only runs when
`canTick` holds. Our `advance` reset the timer whenever the interval elapsed,
regardless of readiness — so an autobuyer waiting to afford a purchase would
restart its phase each interval instead of carrying it, then fire an interval
late once ready.

`advance` now takes a `ready` flag and only fires (resets) when `ready`, while
the phase still accrues every tick. `tick_autobuyers` builds `ready` per
autobuyer (active + unlocked + the action-specific condition), reusing existing
predicates (`can_dim_boost`, `can_buy_galaxy`, `can_big_crunch`) and two new
ones (`dim_single_affordable`, `tickspeed_available` / `tickspeed_affordable`
mirroring the JS getters, including the Continuum / `NUMBER_MAX_VALUE` guards).
Big Crunch keeps the split the original has: the reset is gated on `canCrunch`,
the crunch itself additionally on `willInfinity` (`will_auto_crunch`).

Grid 38 → **39**, trace still clean over 1000 ticks. Two tickspeed integration
tests now set up a 2nd dimension, since the faithful readiness gate (unlike our
lenient `buy_tickspeed`) requires `Tickspeed.isUnlocked` (a 2nd AD owned).

Still out of scope: our `buy_tickspeed` itself doesn't check
`isAvailableForPurchase` (only EC9 + affordability), so the manual path stays
more lenient than the original — a separate, pre-existing faithfulness gap.
