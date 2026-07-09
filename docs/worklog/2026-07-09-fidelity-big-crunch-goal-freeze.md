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

## Bug 15 (partial) — dimension production order; fixture 119 deep-dive

Investigating the deep post-break fixtures (e.g. `00119`, 261 min), whose AD/ID
amounts diverge by growing factors down the tier chain (~25×/tier, dim6 1.4
orders → dim0 9.7 orders), all with clean round-trips and no structural
count/flag divergence.

**Fixed:** the original produces `TimeDimensions → InfinityDimensions →
AntimatterDimensions` each tick (game.js), so AD production reads *this* tick's
Infinity Power. We produced ADs first, then TD/ID, so the AD multiplier's
`infinityPower^7` term was stale. Reordered TD/ID ahead of AD production
(inert pre-break, where `infinityPower = 0`).

**Still open:** that reorder only recovered ~0.2 of the ~11 orders on `00119`.
Tracing showed `tickspeed` (both 8 bought), the `infinityPower^7` multiplier,
`Sacrifice` (both 1), `achievement_power`, the dim-boost/buy-10 factors, and every
`bought` count all match JS; `infinityPower` itself matches at horizon 1. Yet AD
production stays 25×/tier low. The save's AD5–AD8 are empty at load and the
autobuyer buys them (same counts as JS) within the tick, so the divergence is in
the buy+chain-production interaction of freshly-bought top dimensions — it needs
per-tick *intermediate* state (post-autobuyer / pre-production) that the oracle's
persisted-player snapshots don't expose. Left as the next target.

## Bug 16 — the `slowestChallengeMult` Break Infinity Upgrade (resolves Bug 15)

The Bug-15 "25×/tier" divergence on the deep post-break fixtures turned out to be
a single missing multiplier, not a production-chain subtlety.

### Diagnosis
Added a temporary within-tick dump to the oracle (monkey-patching
`AntimatterDimensions.tick` to snapshot the post-autobuyer / post-TD-ID /
pre-AD-production state, plus `AD8.multiplier`, `AD8.productionPerSecond`, and the
game-speed diff) and a matching Rust snapshot hook. This confirmed the pre-AD
state is bit-identical (antimatter, infinityPower, every AD amount/bought), the
game-speed diff matches (50 ms, 1×), yet JS's `AD8.multiplier = 3.082e46` vs
Rust's `dimension_multiplier(7) = 1.264e45` — a clean **24.39×**.

Breaking the multiplier into components isolated the gap to the Break Infinity
common multiplier. JS's `antimatterDimensionCommonMultiplier` applies
`BreakInfinityUpgrade.slowestChallengeMult`
(`clampMin(50 / Time.worstChallenge.totalMinutes, 1)`, capped at 3e4), which Rust
had stubbed as "deferred — no challenge best-times". For `00119` the slowest
Normal Challenge best time is 122 999 ms → `50 / (122999/60000) = 24.390` —
exactly the missing factor.

### Fix
Implemented `slowest_challenge_mult()` from the already-decoded `nc_best_times_ms`
(worst = max over the 11 Normal Challenge best times; any uncompleted challenge is
`f64::MAX`, so the effect stays ×1 until every challenge is complete) and folded
it into `break_infinity_upgrade_common_mult`. Rust's `dimension_multiplier(7)` now
matches JS to ~15 significant figures.

### Verification
- Fidelity grid: 424 → **574** cells (+150). No early-game regression.
- `cargo test -p ad-core --features serde`: 566 pass; fmt + clippy clean.
- All temporary diagnostics removed (oracle dump, `tick.rs` stop-flag, the
  breakdown tests, the `/tmp` scratch saves).

`00119` itself still fails: at horizons 1/10 a small residual (~2.7e-4/tier,
compounding linearly down the chain — i.e. the shared per-tier `tickspeed`
factor) sits just over the 1e-4 epsilon, and at 100/1000 JS performs a Big Crunch
that Rust does not. Both are separate follow-ups; the dominant structural gap on
this whole cohort of post-break fixtures is closed.

## Bug 17 — the Dimension Boost autobuyer's "buy max" branch

### Symptom
Many deep post-break fixtures (e.g. `00204`, `00206`, in Infinity Challenge 7 with
antimatter ≈ 1e7810) diverged catastrophically at horizon 1: Rust bought a
Dimension Boost that JS did not, and the boost's soft reset collapsed antimatter
from ~1e7810 to the ~5e25 starting value (JS kept growing). The shared tell was
`dimensionBoosts JS=4 Rust=5`.

### Diagnosis
Every `DimBoost` gate said "yes" (`canBeBought`, `requirement.isSatisfied`, the
autobuyer active with its interval elapsed), yet JS still didn't boost. Driving
the real game confirmed `Autobuyer.dimboost.isBuyMaxUnlocked` was **true**: the
`autobuyMaxDimboosts` Break Infinity Upgrade switches the autobuyer to a distinct
`tick()` branch that boosts only when a boost would unlock a new dimension **or**
the wait-for-galaxies condition is met — and returns otherwise. With all 8
dimensions already unlocked (`canUnlockNewDimension` false) and 0 galaxies (< the
configured 5), JS's branch returned without boosting. Rust had stubbed this
upgrade as "deferred behaviour" and always ran the default (boost-under-cap)
branch, so it boosted.

### Fix
Implemented the buy-max branch: `is_buy_max_dimboosts_unlocked`,
`can_unlock_new_dimension`, and `max_buy_dim_boosts` (JS `maxBuyDimBoosts` — buy
one to unlock a new dimension, else linearly extrapolate the tier-8 requirement
with an EC5 binary-search fallback), plus `soft_reset_bulk` (JS `softReset(bulk)`
with the `antimatter > Player.infinityLimit` guard and the boost-cap clamp). The
autobuyer now branches on the upgrade: the buy-max path gates on
`canUnlockNewDimension || galaxyCondition`, fires on the `buyMaxInterval` (seconds)
cadence, and resets on the Infinity prestige event.

Also ported the two Dimension-Boost purchase guards Rust lacked (`can_dim_boost`):
`requestDimensionBoost`'s `antimatter > Player.infinityLimit` and `canBeBought`'s
`maxAM > Player.infinityGoal && (!broke || inAntimatterChallenge)`, adding the
`infinity_limit()` helper (challenge goal, else the full `Decimal::MAX_VALUE` —
distinct from `infinity_goal`'s `1e308`, so post-break boosting is unaffected).

### Verification
- Fidelity grid: 574 → **580** cells (+6); `00204`/`00206` @1 now pass.
- `cargo test -p ad-core --features serde`: 568 pass; fmt + clippy clean.

## Bug 18 — the Antimatter Dimension autobuyer group toggle

### Symptom
Deep fixtures with the AD autobuyers turned off at the *group* level (e.g. `00184`,
`00185`, in Infinity Challenge 5) diverged from horizon 1: JS bought nothing
(`dim0.bought` stayed 10, dimensions barely grew from production alone), while Rust
ran the tier autobuyers and bought aggressively (`dim0.bought` → 2500, antimatter
1e6053 → 1e7120). The tell was `auto.antimatterDims.isActive JS=false Rust=true`
plus the tier autobuyers' `lastTick` reset to 0 in JS.

### Diagnosis
The AD autobuyer's `canTick` uses `thisSetting = individualSetting &&
(collapseDisplay ? groupSetting : true)`, where `collapseDisplay =
allMaxedInterval && allUnlocked && allUnlimitedBulk` (the UI collapses the eight
tier controls into one once they're all maxed, unlocked, and have unlimited bulk —
Achievement 61). On these fully-upgraded saves the display is collapsed, so the
group toggle `auto.antimatterDims.isActive` (false) gates every tier autobuyer.
Rust modelled only the per-tier `is_active` flags and never read the group flag.

### Fix
Modelled the group toggle (`AutobuyerState.ad_group_active` ↔
`auto.antimatterDims.isActive`, decode/encode) and `ad_autobuyer_collapse_display`
(all tiers maxed-interval + unlocked, and Achievement 61), then gated the tier
autobuyers on `individual && (collapseDisplay ? group : true)`.

### Verification
- Fidelity grid: 580 → **584** cells (+4); `00184`/`00185` now pass @1 and @10.
- Round-trip identity holds for all fixtures (the new group flag encodes cleanly).
- `cargo test -p ad-core --features serde`: 569 pass; fmt + clippy clean.

## Bug 19 — the Dimensional Sacrifice autobuyer (+ the `chall8TotalSacrifice` product)

### Symptom
`00145` (438 min, post-break, not in a challenge) diverged from horizon 1 with
every Antimatter Dimension a flat Δlog10 ≈ 17.80 low, plus `sacrificed JS=3.9e1779
Rust=0` and `chall8TotalSacrifice JS=6.25e17 Rust=1`.

### Diagnosis
Driving the game showed an active, unlocked **Sacrifice autobuyer**
(`multiplier`/threshold 2) that Rust didn't model — JS auto-sacrificed on tick 1,
Rust didn't. With Achievement 118 the sacrifice keeps the dimensions (no reset),
so the sacrifice's 8th-dimension boost (`nextBoost` = 6.25e17) propagates down the
production chain and lifts every dimension by log10(6.25e17) ≈ 17.80. The
`chall8TotalSacrifice` gap was a second bug: `sacrificeReset` multiplies the
running product by `nextBoost` for **every** sacrifice, but Rust only did so
inside the NC8 branch.

### Fix
Rewrote `sacrifice()` to mirror `sacrificeReset()` (the always-on
`chall8TotalSacrifice *= nextBoost` and `sacrificed += AD1`, the pre-crunch
antimatter guard, the NC8 cap, and the Achievement-118-gated resets), then modelled
the Sacrifice autobuyer: `AutobuyerState.sacrifice_active`/`sacrifice_multiplier`
(↔ `player.auto.sacrifice`, decode/encode), `sacrifice_autobuyer_unlocked` (the
`autoIC` milestone or a completed IC2), and the trigger in `tick_autobuyers`
(sacrifice when `Achievement(118)` applies or `nextBoost >= max(threshold, 1.01)`),
placed after the prestige autobuyers to match the original's order.

### Verification
- `00145`'s structural divergence is gone (17.80 orders → ~1.5e-4 FP noise); it now
  fails only on floating-point drift, like the rest of the deep cohort. Grid holds
  at 584/1148 (no fixture flips, none regress; round-trip clean).
- `cargo test -p ad-core --features serde`: 570 pass; fmt + clippy clean.

## Bug 20 — Galaxy/Dim-Boost autobuyer order (and the buy-max `resetTickOn`)

### Symptom
`00129`, `00154` (post-break, buy-max Dim Boost active) diverged at horizon 1 with
`dimensionBoosts JS=4 Rust=30` and `galaxies JS=6 Rust=5` — JS bought an Antimatter
Galaxy (which resets boosts to the starting 4) while Rust bought 18 Dimension
Boosts instead.

### Diagnosis
On the input both `DimBoost.requirement` (131) and `Galaxy.requirement` (371)
exceed the 8th dimension (120), so nothing can fire yet. But the AD autobuyers run
first and grow the 8th dimension past both thresholds, and then **whichever of the
Galaxy / Dim Boost autobuyers runs next consumes it via its reset**. The original
ticks them in `singleComplex` order — Galaxy *before* Dim Boost — so a galaxy
pre-empts the boost; Rust ran Dim Boost first. A follow-on mismatch: the buy-max
Dim Boost autobuyer's `resetTickOn` is `INFINITY`, not `ANTIMATTER_GALAXY`, so a
galaxy must not reset its timer — Rust's `reset_autobuyer_ticks` reset it
unconditionally, leaving `auto.dimBoost.lastTick` wrong.

### Fix
Reordered `tick_autobuyers` to run the Galaxy autobuyer before the Dim Boost one,
and made `reset_autobuyer_ticks` use the buy-max-aware Dim Boost `resetTickOn`
(`INFINITY` when `autobuyMaxDimboosts` is bought, else `ANTIMATTER_GALAXY`).

### Verification
- Fidelity grid: 584 → **602** cells (+18). `00129` @1 passes; `00154`'s structural
  divergence is gone (now FP-only). No regressions; round-trip clean.
- `cargo test -p ad-core --features serde`: 571 pass; fmt + clippy clean.

## Bug 21 — the buy-max Dim Boost interval must ignore `autobuyerSpeed`

### Symptom
`00241` (post-break, buy-max Dim Boost, `autobuyerSpeed` bought) boosted on tick 1
where JS did not (`dimensionBoosts JS=4 Rust=5`), collapsing a mid-rebuild state
(all dimensions 0, antimatter 5e25) back to zero instead of letting production
rebuild it.

### Diagnosis
The buy-max interval was 1000 ms and the autobuyer's phase was 545 ms, so JS didn't
fire. Rust applied the `autobuyerSpeed` Break Infinity Upgrade's ×0.5 to the
interval (→ 500 ms), so 545 ms cleared it and Rust boosted. In the original the
buy-max `interval` getter *overrides* `UpgradeableAutobuyerState.interval` and
returns `buyMaxInterval` seconds directly — the `autobuyerSpeed` halving lives only
in the overridden `super.interval`, so it does not apply in buy-max mode.

### Fix
Dropped the `speedup` factor from the buy-max Dim Boost branch's effective interval
in `tick_autobuyers` (`buy_max_interval * 1000`, not `* speedup`).

### Verification
- `00241`'s spurious boost is gone: its tick-1 divergence drops from 43 fields to a
  743-order production drift down the rebuilt AD chain (a separate issue — the
  from-zero rebuild with astronomical multipliers still diverges ~124 orders/tier).
  Grid holds at 602/1148 (no regressions; round-trip clean).
- `cargo test -p ad-core --features serde`: 572 pass; fmt + clippy clean.

## Bug 22 — the Big Crunch over-reset `thisInfinity` and skipped the requirement latches

### Symptom
`00124` (both engines crunch on tick 1, identical dimension reset so no production
divergence) diverged on two fields: `requirementChecks.infinity.noAD8 JS=true
Rust=false` and `records.thisInfinity.bestIPminVal JS=3010765 Rust=0`.

### Diagnosis
Rust reset the whole `thisInfinity` record with `ThisInfinity::new()`, zeroing
`bestIPminVal` — but the original's `secondSoftReset` resets only `time` /
`realTime` / `maxAM` / `lastBuyTime`, and `bigCrunchUpdateStatistics` zeroes
`bestIPmin` alone; `bestIPminVal` is kept across the crunch. Rust also never ran
`Player.resetRequirements("infinity")`, so the infinity latches (`maxAll`,
`noSacrifice`, `noAD8`) weren't cleared for the new infinity.

### Fix
Replaced the blanket `thisInfinity = ThisInfinity::new()` with the selective
`secondSoftReset` fields (keeping `best_ip_min_val`), moved the `best_ip_min = 0`
reset into the at-goal `bigCrunchUpdateStatistics` block, and cleared
`infinity_max_all` / `infinity_no_sacrifice` / `infinity_no_ad8` on every crunch.

### Verification
- Fidelity grid: 602 → **606** cells (+4); `00124` @1 now passes.
- `cargo test -p ad-core --features serde`: 573 pass; fmt + clippy clean; round-trip
  clean.

## Bug 23 — the missing Infinity Challenge 8 completion reward (AD 2–7)

### Symptom
`00241`'s from-zero AD rebuild (after its crunch) diverged ~124 orders/tier down
the chain — a 743-order gap at the 1st dimension — even though bought counts and the
top dimensions matched.

### Diagnosis
Capturing JS's per-tier AD multipliers at production time and recomputing Rust's on
the identical intermediate state showed AD1 and AD8 matched exactly but **AD2–AD7
were ~124 orders low**. Those are precisely the tiers (`tier > 1 && tier < 8`) that
`applyNDMultipliers` gives `InfinityChallenge(8).reward`:
`(AD1.multiplier × AD8.multiplier)^0.02`. Rust modelled IC8's *production decay*
(while running) but not its *completion reward*; for `00241`
`(2.345e3195 × 5.4e2700)^0.02 ≈ e118`, the missing per-tier factor.

### Fix
Applied the IC8 completion reward to AD 2–7 in `dimension_multiplier` using the 1st
and 8th dimensions' final multipliers (they never include the reward, so there is no
recursion). All eight per-tier multipliers now match JS to ~15 significant figures.

### Verification
- `00241`'s tick-1 divergence collapses from **743 orders to 0.34** (the residual is
  the infinity-power drift — `infinityPower` Δ≈5.6e-3 → `IP^7` ≈0.04/tier compounding —
  a separate, much smaller issue). Grid holds at 606/1148 (no regressions;
  round-trip clean; the structural @1 count/flag list stays empty).
- `cargo test -p ad-core --features serde`: 574 pass; fmt + clippy clean.

## Bug 24 — Infinity Dimension production must compound within a tick

### Symptom
After the IC8 fix, `00241`'s AD chain still drifted ~0.04 orders/tier, tracking a
`infinityPower` divergence (Δ≈5.6e-3) that fed the `IP^7` AD multiplier.

### Diagnosis
On JS's exact pre-Infinity-Dimension state, Rust's per-tier ID multipliers and
Infinity Power all matched — yet running Rust's `tick_infinity_dimensions` on that
state produced `1.4051e364` vs JS's `1.4233e364`. `tick_infinity_dimensions`
snapshotted *every* tier's production-per-second up front (`std::array::from_fn`)
and then applied them, so the chain did **not** compound within the tick. The
original produces sequentially top-down (`ID(tier).produceDimensions(ID(tier-1),
diff/10)`), so each tier reads the amount the tier above just added, and the final
Infinity Power uses the *compounded* 1st Infinity Dimension — exactly like the AD
production loop, which already recomputed its rate inside the loop.

### Fix
Recompute `id_production_per_second` inside the chain loop (top-down), and produce
Infinity Power from the just-updated 1st Infinity Dimension. Rust's post-ID
Infinity Power now matches JS to ~15 significant figures.

### Verification
- Fidelity grid: 606 → **607** cells (+1). `00241`'s AD chain drift drops from
  ~0.04/tier to ~1e-4/tier (a `replicanti` divergence, Δ≈4.9e-2, is the next
  prominent issue). The single-tick effect is small for steady-state saves but is a
  correctness fix that matters wherever Infinity Dimensions change fast within a tick.
- `cargo test -p ad-core --features serde`: 575 pass; fmt + clippy clean; round-trip
  clean.

## Bug 25 — the Replicanti sub-interval `timer` was dropped on load

### Symptom
`00232`, `00233`, `00241` diverged on `replicanti.amount` at horizon 1 (~8–12%).
For `00241`, Rust's post-tick Replicanti equalled JS's *pre*-tick value — Rust grew
nothing where JS completed one interval.

### Diagnosis
Replicanti replicate in discrete intervals; the leftover sub-interval time is stored
in `player.replicanti.timer` and carried across ticks (and saves). Rust decoded this
as 0 (deliberately, on the assumption it was transient). For `00241`
`getReplicantiInterval = 166.77 ms` and the saved `timer = 140.94 ms`, so JS's
`tickCount = (50 + 140.94) / 166.77 = 1` (one interval, ×1.12), while Rust's
`(50 + 0) / 166.77 = 0` — no growth.

### Fix
Decode and encode `player.replicanti.timer` into `ReplicantiState.timer_ms` (with a
serde default of 0 for older saves).

### Verification
- The Replicanti divergence is gone for all three fixtures (they now fail only on the
  general ~2e-4/tier AD-chain floating-point drift). Grid holds at 607/1148;
  round-trip clean (the new `timer` field round-trips).
- `cargo test -p ad-core --features serde`: 576 pass; fmt + clippy clean.

## Bug 26 — records time was advanced *after* production, not before (the ~2e-4/tier drift)

### Symptom
Most surviving fixtures failed by a small, per-tier-compounding AD-chain drift
(~2e-4/tier). On `00241` every per-tier multiplier, tickspeed, and infinity power
matched JS to ~13–15 figures, yet the AD chain still drifted.

### Diagnosis
Isolating the AD production proved it was *correct* (running it on JS's exact pre-AD
state reproduced JS's result to ~1e-13), so the divergence was upstream. Diffing
Rust's own pre-AD state against JS's intermediate, everything matched **except the
records time**: Rust's `thisInfinity.time_ms` was 50 ms (one tick) behind JS's.
Rust advanced the time records at the *end* of `tick`, but the original advances
them (`records.thisInfinity.time += diff`, etc.) *before* the Dimension ticks. The
time-based Antimatter Dimension achievement multipliers (56/76/91/92) and Infinity
Challenge 8's decay therefore read last tick's time — for `00241`, Achievement 56's
`6/(min+3)` boost came out 2.71e-4 high, an all-tier factor that compounded down the
AD chain.

### Fix
Moved the records time advance (`total`/`realTimePlayed`, `thisInfinity`/
`thisEternity`/`thisReality` game + real time) to run right after the autobuyers and
before Time/Infinity/Antimatter Dimension production, matching the game loop.

### Verification
- Fidelity grid: 607 → **706** cells (+99). `00241` now passes @1 and @10.
- `cargo test -p ad-core --features serde`: 578 pass; fmt + clippy clean; round-trip
  clean (the autobuyer `lastTick` derivation is unaffected — the final
  `realTimePlayed` is the same, and the autobuyers still run before the increment).

## Bug 27 — the pre-break 1e315 Antimatter Dimension production cap

### Symptom
A cohort of pre-break, near-the-wall fixtures (`00079`, `00081`–`00085`) failed at
horizon 100/1000 only on the *peak/total* records: `records.totalAntimatter`,
`records.thisEternity.maxAM`, `records.thisReality.maxAM`. JS's `maxAM` stayed
frozen at ~5e313 across every Big Crunch, while Rust's grew unbounded (e315 → e327);
the current antimatter and dimensions matched.

### Diagnosis
These saves repeatedly hit the Infinity wall and auto-crunch. On the tick antimatter
crosses the goal it overshoots (recorded as `maxAM`) before being capped to the goal.
JS's `productionPerSecond` ends with `.min(cappedProductionInNormalChallenges)`,
which is `1e315` unless post-break (Infinity broken and outside a Normal Challenge,
or inside an Infinity Challenge, or Enslaved). So pre-break each dimension produces
at most 1e315/s, bounding the per-tick antimatter gain to 1e315·0.05 = 5e313 — exactly
JS's frozen `maxAM`. Rust never applied the cap, so its overshoot (and thus the peak
records) ran away near the super-exponential wall.

### Fix
Cap `dimension_production_per_second` at 1e315 unless post-break, mirroring
`cappedProductionInNormalChallenges`.

### Verification
- Fidelity grid: 706 → **715** cells (+9). `00081` now passes all four horizons.
- `cargo test -p ad-core --features serde`: 579 pass; fmt + clippy clean; round-trip
  clean.
