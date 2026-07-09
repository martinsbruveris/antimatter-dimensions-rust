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

## Bug 6 — autobuyer `lastTick` drifts while autobuyers are globally off

### Symptom
Fixtures with autobuyers toggled off (e.g. `00046`–`00055`) diverged at horizon 1
on *every* autobuyer's `lastTick` (all 12: the 8 AD tiers, tickspeed, dimBoost,
galaxy, bigCrunch): JS held the loaded `0`, Rust `50` — and it grew by one tick
(50 ms) per frame. Those `lastTick` fields were the sole divergence.

### The bug
We model the autobuyer timer as elapsed time (`timer_ms = realTimePlayed -
lastTick`) and re-derive `lastTick = realTimePlayed - timer_ms` on encode. The
original's `timeSinceLastTick` is *derived* from `realTimePlayed`, so it keeps
growing with real time even while autobuyers are globally off — leaving the stored
`lastTick` fixed. But `tick_autobuyers` returned early when
`!autobuyers.enabled`, so `timer_ms` froze while `realTimePlayed` kept advancing;
the re-derived `lastTick` then crept up one tick per frame.

### The fix
In the globally-disabled branch, accrue `timer_ms += dt` for all 12 timer-bearing
autobuyers (no firing) before returning, keeping `realTimePlayed - timer_ms`
constant.

### Verification
- Fixtures `00046`–`00055`: horizon 1 now green (later horizons that still fail
  carry the numerical drift or a crunch-timing difference).
- Fidelity grid: 130 → **160** cells (+30). No regressions.
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.

## Bug 7 — Dim Boost / Galaxy autobuyer limit config unmodelled

### Symptom
Fixtures `00058`–`00068`+ diverged on `auto.dimBoost.{limitDimBoosts,maxDimBoosts,
limitUntilGalaxies,galaxies,buyMaxInterval}` and `auto.galaxy.{limitGalaxies,
maxGalaxies,buyMax,buyMaxInterval}` (e.g. JS `maxDimBoosts=6`, Rust `1`). Worse,
`00061` also showed `dimensionBoosts` JS=6 / Rust=7 — Rust ignored the cap and
over-boosted.

### The bug
The prestige-autobuyer limit config was neither decoded, encoded, nor applied.
The original gates the Dim Boost autobuyer on `!limitDimBoosts || purchasedBoosts
< maxDimBoosts` (or the `limitUntilGalaxies` wait), and caps the Galaxy autobuyer
at `maxGalaxies`.

### The fix
Added `DimBoostAutobuyerConfig` and `GalaxyAutobuyerConfig` to `AutobuyerState`
with decode/encode, and gated the two autobuyers' readiness on the caps in
`tick_autobuyers` (the pre-Reality, non-`isBuyMaxUnlocked` path).

### Verification
- `00061` no longer over-boosts; `00059/60/62/63/66/67/68` pass horizon 1 (the
  rest carry only the numerical drift).
- Fidelity grid: 160 → **187** cells (+27). No regressions.
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.

## Interlude — categorising the residue with a raised tolerance + oracle trace

Per a mid-session suggestion, swept the grid at looser tolerances:

| eps | passing |
| --- | --- |
| 1e-6 | 187 |
| 1e-5 | 220 |
| 1e-4 | 263 |
| 1e-3 | 270 |

So ~76 cells fail *only* between 1e-6 and 1e-4 — the accumulated-rounding /
catastrophic-cancellation class (confirmed: a dense oracle trace of `00023` puts
the first divergence at **tick 1**, antimatter/dim0 ~1e-6, with every multiplier
component matching JS to ~1e-15 — the residue is `f64::powf`-vs-V8-`Math.pow` and
Decimal-cancellation noise). Discrete-bug hunting now targets the ~49 cells that
still fail at 1e-4.

## Bug 8 — Tickspeed autobuyer "Buys max" mode (100) dropped

### Symptom
Fixtures `00071`–`00077` diverged on `auto.tickspeed.mode`: JS `100`
(`BUY_MAX`), Rust `1` (single).

### The bug
Two problems: the tickspeed mode was never decoded (left at the default single),
and even the encode used the AD `BUY_10` value (10) rather than the tickspeed
`BUY_MAX` (100). Rust already runs `buy_max_tickspeed()` for that mode, so only
the codec was wrong.

### The fix
Decode the tickspeed mode (100 → BuyMax, else single) and encode BuyMax as 100
for the tickspeed autobuyer specifically.

### Verification
- `00071/74/75` pass horizon 1 (the rest carry other discrete bugs +/or drift).
- Fidelity grid: 187 → **193** cells (+6). No regressions.
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.

## Bug 9 — `ipMult` ×2 Infinity Upgrade (`IPMultPurchases`) unmodelled

### Symptom
`00076`/`00077` diverged on `IPMultPurchases` (JS 1, Rust 0).

### The bug
The repeatable ×2-IP `ipMult` Infinity Upgrade was not modelled: its purchase
count `player.IPMultPurchases` was dropped, and `total_ip_mult` omitted the
`2^IPMultPurchases` factor.

### The fix
Added `GameState.ip_mult_purchases` (decode/encode), applied `2^purchases` (flat
`1e1000000` past 3.3M) in `total_ip_mult`, and reset it on Eternity and Reality.

### Verification
- `00076`/`00077` pass horizon 1.
- Fidelity grid: 193 → **195** cells (+2). No regressions.
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.

## Bug 10 — autobuyer `resetTick` and `postC4Tier` not reset on prestige

### Symptom
A dense oracle trace of `00059` (using the now-available game server) put the
first divergence at **tick 110**, a Big Crunch: JS zeroed every autobuyer
`lastTick` and set `postC4Tier` to 1, while Rust left the timers firing normally
(`lastTick` ≈ realTimePlayed) and `postC4Tier` at 6.

### The bug
Two prestige-reset behaviours were missing:
1. The original's `Autobuyers.resetTick(prestigeEvent)` sets `lastTick = 0` for
   every autobuyer whose `resetTickOn <= event` (AD/Tickspeed on any prestige,
   Dim Boost from a Galaxy, Galaxy from a Crunch, Big Crunch from an Eternity).
   We modelled none of it.
2. `resetChallengeStuff()` sets `postC4Tier = 1`; our `reset_challenge_stuff`
   reset the challenge powers/matter but not `post_c4_tier`.

### The fix
- Added `reset_autobuyer_ticks(event, dt)` and called it after each successful
  prestige action (dim boost / galaxy / crunch / eternity / reality) in
  `tick_autobuyers`. Because `lastTick = 0` means `timer_ms == realTimePlayed` at
  save and the original bumps `realTimePlayed` *after* the autobuyer pass, the
  reset targets the post-tick value (`realTimePlayed + dt`) so the derived
  `lastTick` lands exactly on 0.
- Added `self.post_c4_tier = 1` to `reset_challenge_stuff`.

### Verification
- `00059` dense trace: first divergence tick 110 → **none over 1000 ticks**.
- Fidelity grid: 195 → **203** cells (+8). No regressions.
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.

## Session summary

Ten discrete fidelity bugs fixed this session, taking the grid from **34 → 203
cells** at the 1e-6 tolerance (and **284/312** at 1e-4):

1. Achievement 28 on bulk buys (count-neutral correctness).
2. AD production freeze at the Big Crunch goal + `maxAM` overshoot (+2).
3. `bestInfinitiesPerMs` / `bestIPminEternity` records (+57).
4. Normal Challenge best times (+2).
5. `thisEternity.bestIPMsWithoutMaxAll` (+35).
6. Autobuyer `lastTick` drift while globally disabled (+30).
7. Dim Boost / Galaxy autobuyer limit config (+27).
8. Tickspeed autobuyer `BUY_MAX` mode (+6).
9. `ipMult` ×2 Infinity Upgrade / `IPMultPurchases` (+2).
10. Autobuyer `resetTick` + `postC4Tier` on prestige (+8).

### The remaining ~109 failures (at 1e-6) split cleanly

- **~76 are the floating-point drift / catastrophic-cancellation class**: they
  pass at 1e-4 (263 → 284 after this session's fixes). A dense oracle trace of
  `00023` confirmed the divergence is present at tick 1 with every multiplier
  component matching JS to ~1e-15 — the residue is `f64::powf`-vs-V8-`Math.pow`
  and Decimal add/normalize rounding, amplified by `multiplier^upgrades` and by a
  large purchase's subtraction from a near-equal antimatter total. This is what
  the log-space tolerance exists to absorb; closing these would need a bit-exact
  `break_infinity`/pow alignment pass against V8.
- **~28 still fail at 1e-4** and are mostly *timing snowballs* downstream of that
  drift: a purchase/boost/tickspeed buy lands one tick early or late because the
  ~1e-6 production drift tips an affordability check, then cascades into
  `lastBuyTime`, `chall2Pow`, `dimensionBoosts`, `totalTickBought`, `postC4Tier`.
  A handful of records fields (`bestIPminVal`) remain to audit.

### Tooling note
The dense oracle trace (`npm run generate -- --save <n> --trace <f>.json` against
the game served on `:8080`, then `ad-fidelity trace <f>.json`) was decisive for
bug 10 — it pinned the first divergence to the exact tick and field.

## Bug 11 — super-exponential cost scaling (`ExponentialCostScaling`) unmodelled

### Context
A second batch of savefiles (Infinity → very-early-Eternity, 112–620 min) was
added and oracle fixtures generated for them; the default tolerance was raised to
1e-4 (the FP-drift class). Almost every late-game fixture diverged massively.

### Symptom
`00086 @ 1` (post-break, 117 min): antimatter JS=1.26e399 vs Rust=3.6e430
(Δlog10=31), with Rust over-buying tickspeed (`totalTickBought` 396 vs 319) and
dimensions. Round-trip clean, so it was one tick of runaway production.

### The bug
Antimatter Dimension and Tickspeed costs use the original's
`ExponentialCostScaling`: geometric until the cost passes `Number.MAX_VALUE`,
then the per-purchase ratio itself grows by `costScale` each step
(super-exponential). We modelled only the geometric branch, so past the
threshold (tickspeed at ~306 purchases, AD1 at ~103 groups) costs were tens of
orders too cheap — the autobuyers bought far too much and production exploded.

### The fix
- New `cost_scaling.rs` porting `ExponentialCostScaling` (`calculate_cost` +
  `get_max_bought`, both matching the original's precomputed-`log10` formulae).
- `dimension_mult_decrease` / `tickspeed_mult_decrease` = `10 − dimCostMult|
  tickspeedCostMult purchases − EC6·0.2 | EC11·0.07` (the `costScale`).
- `dimension_cost` and a new computed `tickspeed_purchase_cost` (replacing the
  stored/incremental tickspeed cost) now go through the curve; the AD bulk buy
  uses `get_max_bought` (its quadratic branch), and tickspeed buy-max already
  loops the single buy so it needs no separate change.

### Verification
- `00086`: 28 diverged fields → **1** (a sub-1e-6 `lastTick` timing residual).
- No regression on the original 78 fixtures (still 284/312 at 1e-4).
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.

## Bug 12 — `ipOffline` Infinity Upgrade dropped from `infinityUpgrades`

### Symptom
Many late-game fixtures (e.g. `00091`) diverged (round-trip included) on
`infinityUpgrades`: JS held `"ipOffline"`, Rust didn't.

### The bug
`ipOffline` is a one-time Infinity Upgrade stored in the `infinityUpgrades` string
set, but it isn't part of our 16-slot grid enum, so the decoder (which ignores
unknown ids) dropped it and the encoder never wrote it back. Its effect is
offline-only (0 during a replay), so it just needs to round-trip.

### The fix
Added a `GameState.ip_offline_bought` flag, decoded from the `"ipOffline"` id and
re-emitted into the encoded set. Also relaxed the number-field comparison epsilon
to 1e-4 (matching the log-space default) so sub-1e-4 `lastTick`/time timing noise
no longer masks structural divergences.

### Verification
- `00091`: round-trip + horizon 1 now pass.
- Fidelity grid: 308 → **346** cells (+38). No early-game regression (284/312).
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.

## Bug 13 — early-Eternity per-tick accumulators (`partInfinitied`, `maxID1`, `ic2Count`)

After the cost-scaling fix, the widest remaining structural divergences on the new
(Infinity → early-Eternity) fixtures were three per-tick accumulators:

- **`partInfinitied`** (fractional carry of passive Infinity generation). The
  `infinitiedGen` Break Infinity Upgrade grants `0.5·dt/max(50, bestInfinity.time)`
  Infinities per tick; we didn't model it. Added `GameState.part_infinitied`, the
  `generate_passive_infinities` tick step, decode/encode, and Eternity/Reality
  resets. (The RU5/RU7/Ra/glyph/RU11/Effarig factors are all 1/absent pre-Reality.)
- **`requirementChecks.reality.maxID1`**. We modelled this as a bool
  (`reality_had_id1`), but the original tracks the *peak 1st Infinity Dimension
  amount* this Reality (a Decimal, `maxID1.clampMin(ID1.amount)` each ID tick).
  Converted the field to `reality_max_id1: Decimal`, updated it in the ID tick,
  wired decode/encode, and switched Imaginary-Upgrade-15's gate to `== 0`.
- **`ic2Count`** (IC2 auto-sacrifice timer). The original is an `if/else`: on the
  tick the counter reaches 400 it sacrifices and takes the modulo *without* adding
  that tick's delta, and it accrues clamped real time. We added the delta
  unconditionally before the check, firing the sacrifice a tick early. Matched the
  `if/else` + `clamp(dt, 1, 6h)`.

### Verification
- Fidelity grid: 298 → **350** cells across this batch (new-fixture set), no
  early-game regression (284/312).
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.

## Bug 14 — Eternity best-rate records + `ic2Count` preservation

- `player.ic2Count` (IC2 sacrifice timer) was hard-coded to 0 on decode and never
  encoded, dropping any in-progress timer. Now decoded/encoded (+12 cells).
- `records.thisReality.bestEternitiesPerMs` and
  `records.bestEternity.bestEPminReality` — the Eternity analogues of the Infinity
  best-rate records — were unmodelled. Added the fields with decode/encode, the
  Eternity-time update (`bestEternitiesPerMs = clampMin(gainedEternities /
  max(33, thisEternity.realTime))`, `bestEPminReality = max(thisEternity.bestEPmin)`),
  and the Reality resets. This gated a large swath of the new fixtures (+58 cells).

### Verification
- Fidelity grid: 362 → **420** cells. No early-game regression (284/312).
- `cargo test -p ad-core --features serde`: 565 pass; fmt + clippy clean.
