---
status: Implemented
feature: "2.6"
---

# Autobuyers — Feature 2.6

The pre-Infinity autobuyers and, crucially, their **interval-upgrade** system: the
mechanism that lets an autobuyer's tick interval be reduced (with Infinity Points)
down to a 100 ms floor. This is the true gate for Break Infinity (§2.3): the "Break
Infinity" button unlocks only once the **Big Crunch autobuyer** reaches its
100 ms floor (`Autobuyer.bigCrunch.hasMaxedInterval`), and that autobuyer can only
be upgraded after **Normal Challenge 12** is completed.

Original source: `core/autobuyers/autobuyer.js` (the `AutobuyerState` /
`IntervaledAutobuyerState` / `UpgradeableAutobuyerState` hierarchy), the per-type
files (`antimatter-dimension-autobuyer.js`, `tickspeed-autobuyer.js`,
`dimboost-autobuyer.js`, `galaxy-autobuyer.js`, `big-crunch-autobuyer.js`), and the
`player.auto` defaults in `player.js`.

---

## 1. What already exists

`autobuyers.rs` models the **slow versions** of the 8 AD autobuyers + the Tickspeed
autobuyer:
- `Autobuyer { is_bought, is_active, mode, interval_ms, timer_ms }`, a fixed
  `interval_ms` per tier, `advance(dt)` timer, and `tick_autobuyers` firing.
- Unlock by antimatter (`is_bought`): AD tier at `1e(40+10·tier)`, Tickspeed at
  `1e140`, gated on `total_antimatter` (our stand-in for `thisEternity.maxAM`).
- `AutobuyerMode::{BuySingle, BuyMax}`, the global `enabled` switch, save/load.
- Normal-challenge completion (2.5) already flips `is_bought = true` for the NC1–9
  autobuyers as a second unlock path.

Missing (this feature): the **interval-upgrade machinery**, the **Dim Boost /
Galaxy / Big Crunch** autobuyers, the `can_be_upgraded` (challenge-gated) flag, and
`has_maxed_interval`.

---

## 2. Interval-upgrade machinery (`UpgradeableAutobuyerState`)

Every upgradeable autobuyer carries a mutable `interval` (ms) and an IP `cost`:

```
baseInterval        // per-autobuyer default (below)
cost = 1            // IP cost of the next interval upgrade
hasMaxedInterval := interval <= 100
upgradeInterval():
    if hasMaxedInterval: return
    if !buy IP(cost): return
    cost     *= 2
    interval  = max(interval * 0.6, 100)
```

- Each upgrade cuts the interval to 60 % (floored at 100 ms) and doubles the IP
  cost. From a 150 000 ms base, reaching the 100 ms floor takes
  `ceil(log(150000/100)/log(1/0.6)) = 15` upgrades (cost `2^15−1 ≈ 32 767` IP total).
- `maxIntervalForFree()` upgrades to the floor with no IP cost (used by later
  perks/milestones; also handy as a debug/first-implementation path).
- Interval is halved again by `BreakInfinityUpgrade.autobuyerSpeed` — a later
  feature; model as an effect reader returning `interval` unchanged for now.
- `reset()` (on Eternity, unless the keep-autobuyers milestone) restores
  `interval = baseInterval, cost = 1`. No Eternity yet, so unmodelled.

**Base intervals** (`player.auto.*.interval`):

| Autobuyer | baseInterval (ms) |
|-----------|-------------------|
| AD 1–8 | `500, 600, 700, 800, 900, 1000, 1100, 1200` |
| Tickspeed | `500` |
| Dim Boost | `4 000` |
| Galaxy | `20 000` |
| Big Crunch | `150 000` |

Our existing `interval_ms` **is** this value; the only change is that it becomes
mutable (reducible) and gains a companion `cost`.

---

## 3. Autobuyer roster: unlock vs. upgrade gates

Two distinct gates per autobuyer:
- **`is_unlocked`** — whether it runs at all.
- **`can_be_upgraded`** — whether its interval can be reduced (always a completed
  Normal Challenge for these).

| Autobuyer | slow unlock (`is_bought`) | `can_be_upgraded` | `is_unlocked` |
|-----------|---------------------------|-------------------|---------------|
| AD tier `n` (1–8) | `maxAM ≥ 1e(40+10(n-1))` | NC`n` completed | `is_bought ‖ can_be_upgraded` |
| Tickspeed | `maxAM ≥ 1e140` | NC9 completed | `is_bought ‖ can_be_upgraded` |
| Dim Boost | — (none) | NC10 completed | `can_be_upgraded` |
| Galaxy | — | NC11 completed | `can_be_upgraded` |
| Big Crunch | — | NC12 completed | `can_be_upgraded` |

So the "slow version" and the "upgradeable version" are the **same** autobuyer with
two unlock conditions; completing the challenge additionally enables interval
upgrades. The AD/Tickspeed autobuyers can be unlocked by antimatter *or* by their
challenge; Dim Boost / Galaxy / Big Crunch have **no** antimatter path — the only
way to get them is to complete NC10/11/12.

(`maxAM` is `thisEternity.maxAM`; pre-Eternity we use our all-time `total_antimatter`
stand-in, already used for the AD/Tickspeed unlocks. A dedicated `this_eternity.max_am`
record can replace it when Eternity lands.)

---

## 4. The three new autobuyers

### 4.1 `canTick` gate (per type, on top of the shared `interval elapsed && active
&& autobuyersOn && is_unlocked`):

- **AD**: `dim.isAvailableForPurchase && dim.isAffordable` (we already have
  `dim_available_for_purchase`; affordability is `dim_currency_amount ≥ cost`).
- **Tickspeed**: available & affordable.
- **Dim Boost**: `DimBoost.canBeBought && requirement.isSatisfied` → our
  `can_dim_boost()`.
- **Galaxy**: `Galaxy.canBeBought && requirement.isSatisfied` → our `can_buy_galaxy()`.
- **Big Crunch**: `Player.canCrunch` → our `can_big_crunch()`.

### 4.2 Actions on fire
- Dim Boost: `requestDimensionBoost` → our `buy_dim_boost()`. (`buyMax`/limit config
  is Break-Infinity/Eternity gated; default is single boost.)
- Galaxy: `requestGalaxyReset` → our `buy_galaxy()`. (bulk/limit config later.)
- Big Crunch: `bigCrunchResetRequest` → our `big_crunch()` (only when `willInfinity`).

### 4.3 Config (`player.auto` defaults)
- **Big Crunch** — `mode ∈ {AMOUNT(0), TIME(1), X_HIGHEST(2)}`, `amount = 1` (IP),
  `time = 1` (s), `xHighest = 1`, `increaseWithMult = true`. `willInfinity`:
  - **pre-break** (`!player.break || in-AM-challenge`): **always true** → crunches
    as soon as `can_big_crunch()`. This is the only branch that matters until Break
    Infinity, so the modes are cosmetic for now (model the field; treat as
    always-fire).
  - post-break: AMOUNT `gainedIP ≥ amount`, TIME `thisInfinityRealTime > time`,
    X_HIGHEST `gainedIP ≥ maxIP × xHighest`.
- **Dim Boost** — `limitDimBoosts/maxDimBoosts`, `limitUntilGalaxies/galaxies`,
  `buyMaxInterval`. `isBuyMaxUnlocked = BreakInfinityUpgrade.autobuyMaxDimboosts`
  (later); default fires a single boost when unlimited.
- **Galaxy** — `limitGalaxies/maxGalaxies`, `buyMaxInterval`,
  `isBuyMaxUnlocked = EternityMilestone.autobuyMaxGalaxies` (later); default buys
  one when the requirement holds.

For 2.6 we model the persisted config fields (so saves round-trip) but only the
default behaviour is active; the buy-max/limit paths light up with their
Break-Infinity/Eternity prerequisites.

---

## 5. Rust design

### 5.1 `Autobuyer` struct
Add:
- `cost: Decimal` — IP cost of the next interval upgrade (default 1).
- keep `interval_ms` (now mutable, starts at `baseInterval`).

New methods on `GameState` (or `Autobuyer`):
- `has_maxed_interval(&ab) -> bool` (`interval_ms <= 100.0`).
- `can_be_upgraded(target) -> bool` — the mapped `challenge_completed(n)`.
- `is_autobuyer_unlocked(target) -> bool` — `is_bought || can_be_upgraded`.
- `upgrade_autobuyer_interval(target) -> bool` — IP-gated; `cost *= 2`,
  `interval = max(interval*0.6, 100)`; fires achievements 52/53.
- `autobuyer_interval(target)` — the effective interval (base for the
  `autobuyerSpeed` effect later).

A small `AutobuyerTarget` enum (`AdTier(u8)`, `Tickspeed`, `DimBoost`, `Galaxy`,
`BigCrunch`) keeps the upgrade/toggle API uniform and lets the GUI address any
autobuyer by one handle. The challenge→autobuyer map lives in one place.

### 5.2 New autobuyers on `AutobuyerState`
Add `dim_boost: Autobuyer`, `galaxy: Autobuyer`, `big_crunch: BigCrunchAutobuyer`
(the last carries `mode/amount/time/x_highest/increase_with_mult`). They have no
`is_bought` path — `is_active` defaults on, `is_bought` stays false, and they run
purely off `can_be_upgraded`. Reuse `Autobuyer` for dim_boost/galaxy (their extra
limit config can be added as fields with defaults, inert until 2.3/4.x).

### 5.3 `tick_autobuyers`
Extend to advance and fire dim_boost, galaxy, big_crunch after the AD/Tickspeed
loop, each behind its `canTick`:
```
if unlocked & active & interval-elapsed:
    DimBoost: if can_dim_boost() { buy_dim_boost() }
    Galaxy:   if can_buy_galaxy() { buy_galaxy() }
    BigCrunch: if can_big_crunch() && willInfinity() { big_crunch() }
```
Order mirrors the original's `Autobuyers.tick()` priority (dimensions → tickspeed →
dim boost → galaxy → big crunch). The existing `advance()` timer stands in for
`timeSinceLastTick >= interval` (elapsed-accumulator form).

### 5.4 The Break Infinity hook (exposed, consumed in 2.3)
`break_infinity_unlockable() := big_crunch.is_unlocked && has_maxed_interval(big_crunch)`
— i.e. NC12 completed **and** the Big Crunch autobuyer upgraded to 100 ms. Feature
2.3 reads this to reveal the Break Infinity button.

---

## 6. Save / load

`player.auto.{bigCrunch,galaxy,dimBoost}` and the per-tier / tickspeed `cost` +
`interval` join the DTO. New fields:
- Each AD tier + tickspeed: `cost` (number), `interval` (number). We currently
  reconstruct intervals from constants and ignore the saved value; now the saved
  (possibly-upgraded) interval + cost must round-trip.
- `bigCrunch`: `cost, interval, mode, amount (Decimal), time, xHighest (Decimal),
  increaseWithMult, isActive`.
- `galaxy`/`dimBoost`: `cost, interval, isActive` + their limit config.

All present in the template `default_player.json`, so making them required DTO
fields is fine. `lastTick` stays ignored (our timer is an elapsed accumulator reset
to 0 on load).

---

## 7. UI

The Autobuyers subtab already lists the AD + Tickspeed autobuyers. Add:
- The three new autobuyer boxes (Dim Boost / Galaxy / Big Crunch), shown once
  `is_unlocked`.
- An **interval-upgrade button** per box (cost in IP, disabled at the 100 ms floor,
  hidden until `can_be_upgraded`), plus the interval readout.
- Big Crunch mode selector (AMOUNT/TIME/X_HIGHEST) — cosmetic pre-break, so it can
  be deferred with a note.

Vendor the `AutobuyerBox` / interval-upgrade styles from the original.

---

## 8. Incremental plan

1. ✅ **Interval machinery + `AutobuyerTarget`**: `cost` field, `can_be_upgraded`,
   `is_unlocked`, `upgrade_autobuyer_interval`, `has_maxed_interval`; wired into
   the AD/Tickspeed autobuyers; per-autobuyer `interval`/`cost` round-trip. *(part 1.)*
2. ✅ **Dim Boost + Galaxy + Big Crunch autobuyers**: state + `tick_autobuyers`
   firing behind `can_dim_boost`/`can_buy_galaxy`/`can_big_crunch`, NC10/11/12
   unlock, `break_infinity_unlockable()` hook, save/load. *(part 2; the three ship
   together since they share the prestige shape and firing loop. Big Crunch's
   mode/amount/time config is inert pre-break, so it stays at the template default
   rather than being modelled.)*
3. ✅ **UI**: `IntervalUpgradeButton.vue` wired into the AD/Tickspeed boxes (the old
   disabled placeholder), plus `PrestigeAutobuyerBox.vue` for the three new
   autobuyers (row when `is_unlocked`, locked hint before their NC is completed).
   Snapshot gains the per-autobuyer upgrade fields (`can_be_upgraded`,
   `has_maxed_interval`, `upgrade_cost`, `can_afford_upgrade`, `is_unlocked`) and
   the three prestige entries; two string-keyed Tauri commands
   (`upgrade_autobuyer_interval`, `toggle_autobuyer`). *(part 3.)*

Records/`thisEternity.maxAM`, the buy-max/limit config behaviours, and the
`autobuyerSpeed` halving stay stubbed until their prerequisite features (Eternity,
Break Infinity) land.

**Feature 2.6 is complete.** The remaining timing nuance (our `advance` resets the
timer per interval, so the Big Crunch autobuyer at its 150 s base can lag the goal
by up to one interval vs. the original's continuously-accumulating
`timeSinceLastTick`) matches how the existing AD autobuyers already behave and
vanishes once the interval is upgraded toward the 100 ms floor.

---

## 9. Open questions (best-guess defaults)

- `thisEternity.maxAM` stand-in: keep using `total_antimatter` for the slow-version
  unlock (as the existing AD autobuyers do) until Eternity introduces a real
  per-eternity max-AM record.
- Big Crunch modes pre-break: modelled but always-fire; revisit when Break Infinity
  makes `willInfinity` meaningful.
- Whether to expose interval upgrades in a "buy max (free)" debug affordance — the
  existing dev/game-speed controls suggest yes, but not required for fidelity.

*Document generated: 2026-07-03.*
