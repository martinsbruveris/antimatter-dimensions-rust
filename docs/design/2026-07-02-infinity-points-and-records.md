---
status: Implemented
feature: "2.1"
---

# Infinity Points, Infinities & Records — completing Feature 2.1

This document covers finishing **Feature 2.1 (Big Crunch / Infinity Prestige)** from
[`2026-06-23-feature-decomposition.md`](./2026-06-23-feature-decomposition.md). The
Big Crunch *trigger* and *reset* already work (see `crates/ad-core/src/crunch.rs`),
but the crunch awards no **Infinity Points (IP)**, tracks no **Infinities** count,
and keeps none of the **time records** the rest of Phase 2 depends on. This step adds
those three things end-to-end (engine → save/load → UI) so that a crunch becomes a
real prestige.

It is deliberately scoped *below* Feature 2.2 (Infinity Upgrades): we add the IP
currency and the records that later features read, but nothing that *spends* IP yet.
The Infinity tab created here is the shell that 2.2 fills with the upgrade grid.

---

## 1. What the original does

### 1.1 IP gain (`gainedInfinityPoints`, `game.js`)

```js
const div = Effects.min(308, Achievement(103), TimeStudy(111)); // = 308 pre-those
let ip = player.break
  ? Decimal.pow10(player.records.thisInfinity.maxAM.log10() / div - 0.75)
  : new Decimal(308 / div);                                       // = 1 pre-break
ip = ip.times(GameCache.totalIPMult.value);                       // = 1 pre-upgrades
return ip.floor();
```

Break Infinity is Feature 2.3 (not implemented), so `player.break` is always false
for us and the pre-break branch `308 / div = 308 / 308 = 1` always applies.
`totalIPMult` is 1 until the IP-mult infinity upgrade / achievements 85/93/… exist
(Feature 2.2+). So **every pre-break crunch grants exactly 1 IP** — but we implement
the whole shape (`div`, `total_ip_mult`) so 2.2/2.3 slot in by extension, not rewrite.

### 1.2 Infinities gain (`gainedInfinities`, `game.js`)

`Effects.max(1, Achievement(87)) × …` — all the multiplier sources are post-Reality,
so pre-Reality it is exactly **1** (floored). Awarded as `Currency.infinities.add(…)`.

### 1.3 Records touched by a crunch

The game keeps a large `player.records` tree; the slice Phase 1–2 needs:

- `records.totalTimePlayed` / `realTimePlayed` — monotonic, `+= diff`/`+= realDiff`
  every game loop. Pre-Infinity game speed is 1, so `diff == realDiff == dt`.
- `records.thisInfinity.{time, realTime, maxAM}` — time since the last crunch and the
  max antimatter reached this infinity. `maxAM` is updated in the antimatter setter
  (`currency.js`): `maxAM = maxAM.max(value)`.
- `records.bestInfinity.{time, realTime}` — fastest infinity ever, initialised to
  `Number.MAX_VALUE`, updated at crunch to `min(best, thisInfinity)`.

On crunch (`bigCrunchUpdateStatistics` + `secondSoftReset`): best-infinity time is
lowered to this run's time, then `thisInfinity.time/realTime/maxAM` reset to 0.

### 1.4 What resets vs. persists

- **Reset** (already done): antimatter, dimensions, tickspeed, dim boosts, galaxies,
  sacrifice, plus (new) `thisInfinity` records.
- **Persist**: IP (cumulative), infinities count, `totalTimePlayed`, `bestInfinity`,
  achievements, options, `infinity_unlocked`.

---

## 2. Engine design (`ad-core`)

### 2.1 New `GameState` fields

```rust
pub infinity_points: Decimal,   // cumulative IP; persists across crunch
pub infinities: Decimal,        // count of infinities; persists
pub records: Records,           // time/prestige records (new module records.rs)
```

`total_antimatter` stays a top-level field (it is wired through many call sites); the
new `Records` holds only the *time / infinity* records, keeping this diff focused.
The naming mismatch (`game.total_antimatter` vs `game.records.this_infinity`) is
documented and can be unified later if a broader records refactor is warranted.

### 2.2 `Records` (new `records.rs`)

```rust
pub struct Records {
    pub total_time_played_ms: f64,
    pub real_time_played_ms: f64,
    pub this_infinity: ThisInfinity,
    pub best_infinity: BestInfinity,
}
pub struct ThisInfinity { pub time_ms: f64, pub real_time_ms: f64, pub max_am: Decimal }
pub struct BestInfinity { pub time_ms: f64, pub real_time_ms: f64 }
```

`best_infinity` initial time is `f64::MAX` (== JS `Number.MAX_VALUE`), i.e. "no
infinity yet".

### 2.3 Time tracking in `tick(dt_ms)`

Pre-Infinity there is no game-speed multiplier, so game time == real time == `dt_ms`.
After production and the Big-Crunch cap, `tick` accumulates:

```rust
self.records.total_time_played_ms += dt_ms;
self.records.real_time_played_ms  += dt_ms;
self.records.this_infinity.time_ms      += dt_ms;
self.records.this_infinity.real_time_ms += dt_ms;
self.records.this_infinity.max_am = self.records.this_infinity.max_am.max(self.antimatter);
```

This also runs during offline replay (`simulate_offline` loops `tick`), which is
correct — away-progress advances the clock too.

### 2.4 IP / infinities formulas

```rust
fn ip_gain_divisor(&self) -> f64 { 308.0 }          // Effects.min(308, Ach103, TS111)
fn total_ip_mult(&self) -> Decimal { Decimal::ONE } // grows with 2.2/2.3

pub fn gained_infinity_points(&self) -> Decimal {
    // Pre-break branch only (Break Infinity = Feature 2.3).
    let base = Decimal::from_float(308.0 / self.ip_gain_divisor()); // = 1
    (base * self.total_ip_mult()).floor()
}
pub fn gained_infinities(&self) -> Decimal { Decimal::ONE }         // Effects.max(1, …)
```

### 2.5 `big_crunch` changes

Award **before** the reset (rewards read pre-reset records), then reset records:

```rust
self.unlock_achievement(21);
self.infinity_points += self.gained_infinity_points();
self.infinities      += self.gained_infinities();
self.records.best_infinity.time_ms =
    self.records.best_infinity.time_ms.min(self.records.this_infinity.time_ms);
self.records.best_infinity.real_time_ms =
    self.records.best_infinity.real_time_ms.min(self.records.this_infinity.real_time_ms);
// … existing pre-Infinity reset …
self.records.this_infinity = ThisInfinity::new(); // time 0, maxAM 0
self.infinity_unlocked = true;
```

IP and infinities are **not** reset (they persist). Everything the crunch already
reset is unchanged.

### 2.6 Snapshot (`ObservedState`)

Add `infinity_points`, `infinities`, and `gained_infinity_points` (what a crunch
would grant right now) so the sim/trace and Python see the new currency.

---

## 3. Save / load

The save's `player.infinities` / `player.infinityPoints` are `Decimal` strings; the
records times are plain numbers and `maxAM` is a `Decimal` string — all present in
the vendored template.

- **DTO / `from_save_dto`**: `infinities` and `infinity_points` are already read (for
  the `infinity_unlocked` derivation); now also *store* them in `GameState`. Extend
  `RecordsDTO` with `total_time_played`, `this_infinity.{time,realTime,maxAM}`, and
  `best_infinity.{time,realTime}`, and map them into `Records`. These are within our
  frontier now, so we load the real values (a late-game save still collapses its
  *mechanics* to fresh early-game, but its IP/infinities/records carry over — they
  are just numbers our `Decimal`/`f64` hold fine).
- **`encode.rs`**: overlay `infinities`, `infinityPoints` (as `Decimal` strings) and
  the records fields onto the template. `break` still carries `infinity_unlocked`.

Round-trip test: decode → mutate IP/infinities/records → encode → decode reproduces
them; plus the existing sample-save decode asserts the loaded records.

---

## 4. UI (`ad-gui`)

- **`GameView`**: add `infinity_points` only. The engine snapshot (`ObservedState`)
  also carries `infinities` and `gained_infinity_points` for the sim/Python, but the
  webview has no consumer for them yet — `infinities` surfaces with the Statistics
  tab, and the per-crunch IP gain with the post-break crunch modal (Feature 2.3) — so
  they are not put on `GameView` prematurely.
- **Infinity tab** (`config/tabs.js`): a new top-level `infinity` tab, shown only
  when `snapshot.infinity_unlocked` (mirrors `condition: infinityUnlocked()`), with
  one subtab **Infinity Upgrades** → new `InfinityUpgradesTab.vue`. It carries the
  infinity accent via `uiClass: "o-tab-btn--infinity"` (Sidebar now applies a
  per-tab `uiClass`). For this feature the tab renders **only** the
  `InfinityPointsHeader` — "You have _X_ Infinity Points." (vendored
  `c-infinity-tab__header` / `c-infinity-tab__infinity-points`). Feature 2.2 adds
  the upgrade grid inside the same `.l-infinity-upgrades-tab` container.
- **Big Crunch confirm modal**: **unchanged** — the original's first-infinity modal
  (`BigCrunchModal.vue` with `alternate-text`) is purely explanatory, showing *no*
  IP number and *no* disable checkbox. The number-showing "You will gain _N_
  Infinity and _M_ Infinity Points" body + the disable checkbox is the *post-break*
  path (`confirm-option: 'bigCrunch'`), which arrives with Feature 2.3.
- **Crunch gating** (`requestBigCrunch`): mirror `manualBigCrunchResetRequest` — show
  the modal only when the bigCrunch confirmation is on **and** it is the first
  infinity (`|| player.break` once 2.3 lands). So pre-break the first crunch pops the
  explanatory modal and every later crunch goes through directly.
- **Auto-navigate**: on the *first* crunch (snapshot `infinity_unlocked` was false),
  the `bigCrunch` store action switches to the Infinity/upgrades subtab, matching the
  original's `Tab.infinity.upgrades.show()` in `bigCrunchTabChange`.

The IP total is intentionally shown in the Infinity-tab header (as the original
does) rather than the game header; the game header stays antimatter-only pre-Infinity.

---

## 5. Deferred / open questions

- **`bestIPmin` / best-IP-per-minute records** are omitted until a consumer exists
  (the `ipGen` infinity upgrade in 2.2, or the Statistics tab). Adding them now would
  be untested gold-plating.
- **Break-Infinity IP formula** (`maxAM`-based) lands with Feature 2.3; the code path
  is structured (`div`, pre/post-break) so it is a branch add, not a rewrite.
- **`totalIPMult`** returns 1 until Feature 2.2 wires the IP-mult upgrade and the
  IP-boosting achievements; kept as a method so those are additive.
- **Records naming**: `total_antimatter` remains top-level rather than moving under
  `Records`, to keep the diff contained; revisit if a full records/statistics system
  is built.

---

## 6. Testing

- Engine: crunch awards exactly 1 IP and 1 infinity; IP/infinities persist across a
  second crunch; `thisInfinity` time/maxAM reset while `bestInfinity` lowers; tick
  advances `totalTimePlayed` and `thisInfinity.time`; `maxAM` tracks the peak.
- Save: IP/infinities/records round-trip; sample-save decode reads the records.
- Build: `cargo test` + `clippy` + `fmt`; frontend `npm run build`; manual smoke of a
  crunch showing the Infinity tab + IP header.

*Document generated: 2026-07-02.*
