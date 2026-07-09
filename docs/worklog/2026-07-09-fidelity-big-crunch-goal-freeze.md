---
date: 2026-07-09
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
---

# Fidelity fixes — achievement 28 on bulk buys, and the Big Crunch goal freeze

Working the fidelity suite from the first failing cell,
`00008-0016-42-49-manual @ 100`. That fixture is an early-game (~16 min) save
that reaches the Infinity wall between horizons 10 and 100. Two distinct bugs
surfaced from its divergences; this file covers both.

## Bug 1 — achievement 28 unlocked on the bulk "buy max" path

### Symptom
`achievementBits` row 2 diverged: Rust had bit 8 (achievement 28) set, JS did
not. Achievement 28 ("There's no point in doing that…") gives 1st ADs ×1.1 *and*
bumps the global achievement power (×1.03 per unlock), so the wrong bit also
inflated every dimension's multiplier.

### The bug
The original only calls `Achievement(28).tryUnlock()` inside `buyOneDimension`
(the genuine single-buy) — never in `buyManyDimension`, `buyAsManyAsYouCanBuy`,
or `buyUntilTen`. Its `checkEvent` (`ACHIEVEMENT_EVENT_OTHER`) is a dead event
that is never dispatched, so the only other route is the post-Reality
auto-achiever.

Our engine unlocked 28 inside `on_buy_dimension`, which fires from *every* buy
path. In particular the AD1 autobuyer's "Buys max" mode runs
`buy_max_dimension_bulk` → `buy_until_10_dimension` → `buy_dimension` →
`on_buy_dimension`, so a bulk AD1 purchase over 1e150 wrongly awarded 28.

### The fix
Split `buy_dimension` into the public single-buy (which keeps the tier-0,
≥1e150 achievement-28 check, mirroring `buyOneDimension`) and a private
`buy_one_dimension` core that performs the purchase + `on_buy_dimension` *without*
the 28 check. `buy_until_10_dimension` now loops `buy_one_dimension`, so the bulk
paths no longer touch achievement 28 — matching the original's split exactly.

### Verification
`achievementBits` no longer diverges. On its own this fix leaves the grid at 34
cells (the fixture still fails on the production divergence below), but it is a
genuine correctness fix. `cargo test -p ad-core --features serde` stays green.

## Bug 2 — production must freeze once the Big Crunch goal is reached

### Symptom
Every `dimensions.antimatter[*].amount` diverged, and the gap *grew* with the
horizon (Δlog10 ≈ 0.4 at 100 ticks, ≈ 3.0 at 1000). Tell-tale: the JS amounts
were **identical at horizons 100 and 1000** — JS had frozen — while Rust kept
producing. Separately, `records.{thisInfinity,thisEternity,thisReality}.maxAM`
diverged by a hair: JS ≈ 1.805e308, Rust exactly `NUMBER_MAX_VALUE`
(1.7977e308).

### The bug
The original's `AntimatterDimensions.tick(diff)` opens with

```js
const hasBigCrunchGoal = !player.break || Player.isInAntimatterChallenge;
if (hasBigCrunchGoal && Currency.antimatter.gte(Player.infinityGoal)) return;
```

Pre-break (or in an antimatter challenge), once antimatter reaches the goal the
whole AD tick is skipped — the dimensions are hidden behind the Big Crunch
button, so **all** production and antimatter gain freeze until the player
crunches. Our tick always ran the production chain and only capped antimatter at
the end, so the dimension amounts kept climbing past the wall.

The `maxAM` discrepancy is the same mechanic seen from the antimatter setter:
`Currency.antimatter`'s setter records `maxAM = maxAM.max(value)` on *every*
assignment, so the overshoot on the reaching tick (produced value, e.g.
1.805e308) is captured *before* `dropTo(infinityGoal)` caps it. We recorded
`maxAM` from the already-capped value, so it stuck at the goal.

### The fix
In `tick.rs`, compute `has_big_crunch_goal` and the `goal`, then wrap the
dimension chain + antimatter gain in `if !(has_big_crunch_goal && antimatter >=
goal)`. Inside, capture `peak_am` (the post-gain, pre-cap antimatter) and use it
for the three `maxAM` record updates, then apply the `dropTo` cap. When
production is frozen, `peak_am` is just the current (already ≥ goal) antimatter,
so `maxAM` doesn't move.

### Verification
- Fixture 8 (`00008-…-manual`): was 2/4, now **4/4**.
- Fidelity grid: 34 → **36** cells, no regressions.
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.

## Bug 3 — best-rate records `bestInfinitiesPerMs` / `bestIPminEternity` unmodelled

### Symptom
The next failing cell, `00009-…-timed @ 1`, diverged on
`records.thisEternity.bestInfinitiesPerMs` (JS 1.66e-8, Rust 0) and
`records.bestInfinity.bestIPminEternity` (JS 9.97e-4, Rust 0). `--roundtrip`
diverged identically, so the values were already present in the input save and
we simply dropped them on decode.

### The bug
Neither field was modelled: not decoded, not encoded (so the encode template's 0
won), and never updated. Both are maintained by `bigCrunchUpdateStatistics()` at
each Big Crunch:
- `bestInfinity.bestIPminEternity = clampMin(thisInfinity.bestIPmin)` (then
  `thisInfinity.bestIPmin` is reset), and
- `thisEternity.bestInfinitiesPerMs = clampMin(gainedInfinities().round() /
  max(33, thisInfinity.realTime))`.

They reset to 0 on Eternity, Reality, and at the start of an Eternity Challenge.
(`bestInfinity.bestIPminReality` is never written by the core game — always 0 —
so it needs no modelling.)

### The fix
- Added `ThisEternity.best_infinities_per_ms` and
  `BestInfinity.best_ip_min_eternity` to the records structs, with decode
  (`dto.rs`) and encode (`encode.rs`) so real-save values round-trip.
- Applied the `bigCrunchUpdateStatistics` update in the `at_goal` branch of
  `big_crunch_reset`, before `this_infinity` is reset.
- Reset `best_ip_min_eternity` in `eternity_reset_core` (shared by Eternity and
  EC-start) and `reality_reset_internal`; `best_infinities_per_ms` already zeroes
  via the `ThisEternity::new()` those paths assign.

### Verification
- Fixture 9: was 0/4 (+rt), now **5/5**.
- Fidelity grid: 36 → **93** cells (+57) — these fields gated the record diff on
  many fixtures. No regressions.
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.
