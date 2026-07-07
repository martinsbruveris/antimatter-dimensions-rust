---
date: 2026-07-07
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
  - ../design/2026-07-03-autobuyers.md
---

# Fidelity fixes — AD production rate, "Buys max" grouping, bulk multiplier

## Summary
Investigating fidelity fixture `00003-0003-47-42-timed` surfaced three
divergences from the original in Antimatter Dimension production and buying.
Fixing the first two took the save-replay suite from 12/88 to 27/88 passing
cells; the third (manual max-buy) and the new bulk-multiplier model align code
that the replays don't yet exercise but soon will.

## The bugs

### 1. Dimension→dimension production ran 10× too fast (tick.rs)
The original's `AntimatterDimensions.tick` feeds each lower dimension from the
one above at **one tenth** the rate that the 1st dimension feeds antimatter: it
passes `diff / 10` to `produceDimensions` but the full `diff` to
`produceCurrency`, and `productionForDiff` scales linearly by that interval. Our
tick applied the full `dt` to *every* production, so every dim→dim gain was 10×
too large. This showed up at horizon 1 as a per-tier divergence that grew with
the number of tiers above (AD8 never diverged — nothing produces it; antimatter
never diverged — its per-tick gain is negligible against hours of accumulated
stock, unlike dimensions, which sit near production equilibrium).

The same routine also revealed a second, smaller mismatch: the original applies
the chain **top-down, mutating amounts in place**, so a dimension produces from
its amount *including* this tick's gain from the tier above, and AD1 feeds
antimatter from its just-bumped amount. We computed all productions from a
pre-tick snapshot. Both are now matched (cascade with `dt/10`, then AD1/AD2 →
antimatter at the full `dt`).

### 2. "Buys max" autobuyer bought partial groups (autobuyers.rs / dimensions.rs)
The original's BUY_10 mode (`buyMaxDimension`) bails unless the **entire** next
group of ten is affordable (`isAffordableUntil10`) and then buys it atomically —
it never leaves a partial group. Our autobuyer called `buy_until_10_dimension`,
which buys one-at-a-time greedily, so at a group boundary where the full group
was unaffordable it still bought as many singles as it could. In fixture 3 at
horizon 10 this made Rust buy AD1 up to `bought=198` while JS stayed at `190`;
the ~8 extra purchases (~1e58 each at group 19) drained ~8e58 of antimatter, so
current antimatter diverged by a factor ~11 even though `maxAM` still matched.

### 3. Manual max-buy: same partial-group flaw, wrong price, latent hang
`buy_max_dimension` (the manual "buy max" button and `max_all`) looped
one-at-a-time while affordable. Besides leaving partial groups, that (a) charged
more than the original, which when bulk-buying pays only for the *most expensive*
group obtained (`getMaxBought` returns a single `logPrice`), and (b) would loop
astronomically many times — effectively hang — on large endgame balances. Not
covered by replay fidelity (no manual actions during replay), but fixed
proactively since `max_all` runs in the sim.

## What shipped
- `tick.rs`: dim→dim at `dt/10`, top-down in-place cascade, then AD1 (and AD2
  under NC12) → antimatter at the full `dt`.
- `dimensions.rs`: `buy_max_dimension_bulk(tier, bulk)` — a faithful port of
  `buyMaxDimension(tier, bulk)` used by both the manual path (`bulk = ∞`) and the
  autobuyer. Complete groups only, `isAffordableUntil10` guard, the antimatter-
  challenge goal guard, analytic `getMaxBought` (linear regime) with the "pay for
  the most expensive group" pricing, and the NC9/IC5 group-by-group loop. Added
  `dim_affordable_until_10`.
- Autobuyer **bulk multiplier**: `Autobuyer.bulk` (default 1),
  `ad_autobuyer_effective_bulk` (clamped to the 512 cap; unlimited `1e100` via
  Achievement 61), wired into the BuyMax path, and round-tripped through the save
  (DTO decode + encode overlay). Added `auto.antimatterDims.all[].bulk` to the
  fidelity allowlist.

## Decisions & why
- **Guard-then-reuse for the atomic group.** Because a dimension's cost is
  constant within a group of ten, once `costUntil10` is affordable every single
  buy in the group succeeds. So the all-or-nothing behavior is just
  `buy_until_10_dimension` behind an `isAffordableUntil10` guard — no separate
  atomic path needed, and per-buy side effects (`onBuyDimension`) stay idempotent.
- **Analytic bulk over a loop.** The normal-regime bulk buy uses the closed-form
  `getMaxBought` rather than a group loop, both to match the original's pricing
  and to stay O(1) on huge balances. The loop is kept only for NC9/IC5, where the
  per-group cost bumps are abnormal and the original itself loops.
- **Faithfully replicate the "unclamped top-group" charge.** When bulk clamps the
  quantity, the original still charges `pow10(maxBought.logPrice)` for the
  *unclamped* top group. Mirrored exactly, quirk and all.

## Deviations from the design doc
- `2026-07-03-autobuyers.md` treated the "Buys max" bulk multiplier as a
  deferred, Break-Infinity-era feature (the AD autobuyer DTO comment called
  `bulk` "still ignored"). It is now modelled and round-trips. The design doc's
  body is left as-is (historical); this entry records the delta.

## Surprises & gotchas
- The dim→dim `diff / 10` factor is easy to miss: it lives in the tick loop's
  call site, not in `productionForDiff`, and only dimension production (not
  antimatter) gets it.
- Whether a divergence *shows* depends on stock-vs-flow: dimensions track their
  production each tick (so a rate error appears immediately), while antimatter is
  a deep accumulator (so the same relative error is swamped).
- `getMaxBought` deliberately undercharges bulk buys ("you only pay for the most
  expensive thing you get"), so a naive group-sum loop would not match JS even
  when it buys the same count.

## Follow-ups
- **Antimatter-goal early return.** The original's `tick` returns *before* any
  production when `hasBigCrunchGoal && antimatter >= goal` (freezing dimensions);
  we always produce and cap antimatter at the end. Matters only when sitting at
  the goal.
- **Super-exponential cost scaling.** `dimension_cost` (and hence
  `buy_max_dimension_bulk`) models only the geometric regime; the original's
  `ExponentialCostScaling` adds a quadratic term once a group's cost passes
  `Number.MAX_VALUE`. Needed for deep post-break saves.
- **Per-dimension production cap** (`cappedProductionInNormalChallenges`) and the
  pre-break `cost > Number.MAX_VALUE` affordability guard are still unmodelled.
- Remaining fidelity failures start at fixture 8 (16h+ playtime) — a separate,
  more-advanced divergence class.

## Tests
- `ad-core`: all pass (432 lib + 22 + 29 integration). Added: `tick.rs` cascade
  covered by existing production tests; `buy_max_bulk_one_is_all_or_nothing`,
  `buy_max_bulk_caps_group_count`, `buy_max_unbounded_lands_on_group_boundary`,
  `effective_bulk_clamps_to_cap_and_unlimited_via_ach61`,
  `buy_max_autobuyer_uses_bulk_multiplier`, `ad_autobuyer_bulk_round_trips`.
- Fidelity suite: 12/88 → 22/88 (dim fix) → 27/88 (autobuyer fix); the manual
  max-buy and bulk model leave the count unchanged (not exercised by replays) but
  the `--roundtrip` guard confirms `bulk` persists on every fixture, with no
  `bulk` field divergences.
- Clippy clean on ad-core + ad-fidelity.
