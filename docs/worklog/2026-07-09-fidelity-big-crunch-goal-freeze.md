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

## The numerical-drift class (fixtures 15, 23 — investigated, not a discrete bug)

After the records fixes, the first remaining failures were tiny relative drifts
right at the tolerance boundary, not logic errors:
- `00015-…` diverges only at horizon 1000: antimatter Δlog10 ≈ 1.6e-5 while the
  dimension amounts drift only ~1e-8. JS freezes at the Infinity wall (identical
  at 100 and 1000); the AD amounts match, but antimatter (fed by AD1) drifts.
- `00023-…` diverges from horizon 1: antimatter Δlog10 ≈ 1.3e-6, dim0 ≈ 1.1e-6,
  stable across horizons; round-trip is clean.

Traced both by decoding the save and dumping every production-multiplier
component. Confirmed tickspeed (`multiplier^upgrades`), the per-purchase
tickspeed multiplier, `achievement_power`, `buy_ten_multiplier`, the dim-boost
power, and sacrifice all match JS to ~1e-15; the tier-1 antimatter
`pow10(log10(p)^effarigantimatter)` round-trip (with the glyph effect defaulting
to 1) is a ~6e-14 near-identity. The residual seed is the accumulation of
ULP-level differences between Rust's `f64`/`libm` and V8's across the ~15-factor
multiplier product, amplified two ways: by `multiplier^totalUpgrades` (112 for
`00023`) and by catastrophic cancellation when a large dimension/tickspeed
purchase subtracts a near-equal cost from antimatter. Both are exactly what the
1e-6 log-space tolerance was meant to absorb — but cancellation pushes a handful
of cells just past it.

Explored (and reverted) three faithful-but-ineffective candidates: the
`break_infinity.js` middle `pow` branch, `achievement_power` `powi`→`powf`, and
`Math.round` in `Decimal::add`. None moved the count; all are real JS behaviours
but below the level that flips a cell here. Closing these boundary cells would
need a dedicated bit-alignment pass over `break_infinity` (matching V8's `pow`,
the `1eN` normalize table, add rounding, and every multiplication order) — a
large, high-risk effort tracked separately. Moved on to the discrete bugs behind
the *other* horizon-1 failures.

## Bug 4 — Normal Challenge best times (`challenge.normal.bestTimes`) dropped

### Symptom
Many later fixtures (`00030`, `00040`, `00050`, `00060`, `00070`, …) diverged on
`challenge.normal.bestTimes[i]`: JS held real completion times (~1.2e7 ms) while
Rust held the `f64::MAX` "never" sentinel — a huge, discrete mismatch.

### The bug
`challenge.normal.bestTimes` (11 entries, NC2–12, indexed `id - 2`) was neither
decoded, encoded, nor updated, so the loaded personal-best times were lost to the
encode template's sentinel. The original maintains them in
`NormalChallenge.updateChallengeTime()` — called on the active challenge from the
crunch's `handleChallengeCompletion` — as `bestTimes[id-2] =
min(bestTimes[id-2], thisInfinity.time)`. They persist forever (no Eternity /
Reality reset).

### The fix
Added `GameState.nc_best_times_ms: [f64; 11]` (mirroring `ic_best_times_ms`) with
decode (`dto.rs`) and encode (`encode.rs`), and the `updateChallengeTime` step in
`big_crunch_reset`'s `at_goal` branch next to the existing IC one.

### Verification
- Fidelity grid: 93 → **95** cells (+2) — the fixtures where NC best-times were
  the *sole* remaining divergence; the rest also carry the numerical drift above.
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.

## Bug 5 — `thisEternity.bestIPMsWithoutMaxAll` unmodelled

### Symptom
After the NC best-times fix, the next remaining discrete divergence (e.g.
`00030 @ 1`) was `records.thisEternity.bestIPMsWithoutMaxAll`: JS held a real
rate, Rust `0`.

### The bug
The last field of `bigCrunchUpdateStatistics` was unmodelled. The original keeps
it only over crunches that did *not* use "Max All" this infinity:
`if (!requirementChecks.infinity.maxAll) bestIPMsWithoutMaxAll =
max(bestIPMsWithoutMaxAll, gainedIP / max(33, thisInfinity.realTime))`. It resets
to 0 on Eternity and Reality.

### The fix
Added `ThisEternity.best_ip_ms_without_max_all` with decode/encode and the guarded
update in the crunch `at_goal` branch (Rust already models the
`requirement_checks.infinity_max_all` latch). The Eternity/Reality resets come
for free via the `ThisEternity::new()` those paths assign.

### Verification
- Fixture 30 @ 1: now passes (remaining higher-horizon fails are the numerical
  drift only).
- Fidelity grid: 95 → **130** cells (+35) — this field gated a large swath of
  mid/late-game fixtures. No regressions.
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.
