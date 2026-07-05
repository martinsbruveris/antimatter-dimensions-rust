---
status: Partial
feature: "2.2"
---

# Infinity Upgrades — Feature 2.2

Implements the Infinity Upgrades grid: 16 one-shot upgrades bought with Infinity
Points, arranged in four columns of four, each column purchasable top-to-bottom.
Builds directly on Feature 2.1 (Infinity Points / records; see
[`2026-07-02-infinity-points-and-records.md`](./2026-07-02-infinity-points-and-records.md)).

The bottom row (`ipMult` rebuyable ×2 IP + `ipOffline`), which unlocks only after
buying all 16 (Achievement 41) and involves an odd rebuyable cost curve + offline
gain, is a **scoped follow-up** — noted in §6, not built here.

Original source: `secret-formula/infinity/infinity-upgrades.js` (data),
`core/infinity-upgrades.js` (purchase/`totalIPMult`), and the effect *application*
sites in `dimensions/antimatter-dimension.js`, `dimboost.js`, `galaxy.js`,
`tickspeed.js`.

---

## 1. The 16 upgrades

Each column is an independent vertical chain: the top has no prerequisite; each
lower cell requires the cell **above it** (`checkRequirement`). Costs are IP.

| Col | Row 0 | Row 1 | Row 2 | Row 3 |
|-----|-------|-------|-------|-------|
| 0 | `totalTimeMult` (1) | `dim18mult` (1) | `dim36mult` (1) | `resetBoost` (1) |
| 1 | `buy10Mult` (1) | `dim27mult` (1) | `dim45mult` (1) | `galaxyBoost` (2) |
| 2 | `thisInfinityTimeMult` (3) | `unspentIPMult` (5) | `dimboostMult` (7) | `ipGen` (10) |
| 3 | `skipReset1` (20) | `skipReset2` (40) | `skipReset3` (80) | `skipResetGalaxy` (300) |

(The grid *rendering* order groups the dim-infinity upgrades pairwise, but the
purchase prerequisite is strictly the cell above in the same column, exactly as the
original's `checkRequirement` chains resolve.)

### Effects (verified against the original)

- `totalTimeMult` — **all** ADs × `(totalTimePlayed_minutes / 2) ^ 0.15`.
- `dim18mult` / `dim27mult` / `dim36mult` / `dim45mult` — ADs {1,8}/{2,7}/{3,6}/{4,5}
  × `dimInfinityMult = infinities × 0.2 + 1`. (Original `Currency.infinitiesTotal`;
  pre-Eternity `infinitiesBanked = 0`, so it is our `infinities`.)
- `buy10Mult` — the buy-10 base multiplier `2 → 2 × 1.1 = 2.2`.
- `resetBoost` — Dimension-Boost **and** Antimatter-Galaxy requirements each `− 9`.
- `galaxyBoost` — galaxies are ×2 as strong (the `effects` product in the tickspeed
  formula; doubles the per-galaxy reduction pre-3-galaxies and the scaled galaxy
  count 3+).
- `thisInfinityTimeMult` — **all** ADs × `max((thisInfinity_minutes / 4) ^ 0.25, 1)`.
- `unspentIPMult` — 1st AD × `(infinityPoints / 2) ^ 1.5 + 1`.
- `dimboostMult` — Dimension-Boost power `2 → 2.5` (`Effects.max(2, 2.5)`).
- `ipGen` — passively generate `totalIPMult` (= 1 pre-upgrades) IP every
  `bestInfinity.time × 10` ms; disabled while `bestInfinity.time ≥ 999999999999` ms.
- `skipReset1/2/3` — start each reset with 1/2/3 Dimension Boosts.
- `skipResetGalaxy` — start each reset with 4 Dimension Boosts **and** 1 Galaxy.

---

## 2. Engine design (`ad-core`)

### 2.1 State

```rust
pub infinity_upgrades: u32,     // bitmask, one bit per grid upgrade
pub part_infinity_point: f64,   // ipGen fractional accumulator (player.partInfinityPoint)
```

The original stores a `Set` of string ids; a `u32` bitmask is the idiomatic Rust
equivalent (16 bits used). Persists across a Big Crunch (reset only on Eternity /
Doomed, later). New `infinity_upgrades.rs` module holds the enum, data, purchase,
and effect readers.

### 2.2 Data + purchase

```rust
enum InfinityUpgrade { TotalTimeMult, Dim18Mult, /* … */ SkipResetGalaxy }  // 16
struct InfinityUpgradeConfig { save_id: &'static str, cost: Decimal, requires: Option<InfinityUpgrade> }
```

`save_id` is the original's string id (`"timeMult"`, `"18Mult"`, …) for save
round-trip. `buy_infinity_upgrade(u)` = not already owned, prerequisite owned,
`infinity_points ≥ cost` → subtract cost, set bit. `skipReset*` additionally run
`skip_resets_if_possible()` immediately (original applies the 4th column
retroactively on purchase).

### 2.3 Effect wiring (fidelity-critical: order & sites)

- **`dimension_multiplier(tier)`** gains: the common infinity-upgrade multiplier
  (`totalTimeMult`, `thisInfinityTimeMult`) applied to every tier; the per-pair
  `dim{18,27,36,45}mult`; and `unspentIPMult` on tier 0. Slotted to match the
  original's `antimatterDimensionCommonMultiplier` + `applyNDMultipliers` order.
- **buy-10 base** — replace the `BUY_TEN_MULTIPLIER` constant use with
  `self.buy_ten_multiplier()` (`2`, ×1.1 if `buy10Mult`).
- **dim-boost power** — replace the `DIM_BOOST_MULTIPLIER` constant use with
  `self.dim_boost_power()` (`2`, or `2.5` if `dimboostMult`).
- **boost/galaxy requirements** — subtract 9 when `resetBoost` owned, in
  `dim_boost_requirement` and `galaxy_requirement`.
- **tickspeed** — `tickspeed_purchase_multiplier` threads a `galaxy_strength_effect`
  (`2.0` if `galaxyBoost`, else `1.0`) exactly where the original's `effects`
  product enters (per-galaxy reduction pre-3; scaled count 3+).
- **skip resets** — `skip_resets_if_possible()` sets `dim_boosts` up to the highest
  owned skip level (and `galaxies` to ≥1 for `skipResetGalaxy`), called from the
  Big-Crunch, galaxy, and dim-boost reset paths (mirrors `softReset`).
- **ipGen** — in `tick`, when owned and `bestInfinity.time_ms < 999999999999`:
  `part_infinity_point += dt_ms / (bestInfinity.time_ms × 10)`, award
  `floor(part) × totalIPMult` IP, keep the remainder.

The `GameView` `buy_ten_multiplier` / `dim_boost_power` fields switch from the raw
constants to these methods so the header numbers reflect the upgrades.

---

## 3. Save / load

- **DTO**: `player.infinityUpgrades` (array of string ids) → set the matching bits
  via `save_id`; unknown ids are ignored (forward-compat). `player.partInfinityPoint`
  (number) → `part_infinity_point`.
- **encode**: write the owned upgrades back as their `save_id` strings and
  `partInfinityPoint`. Both fields exist in the template.

Round-trip test + a load test asserting a known id set maps to the right bits.

---

## 4. UI (`ad-gui`)

`InfinityUpgradesTab.vue` gains the 4×4 grid below the IP header, faithfully
reusing the vendored `o-infinity-upgrade-btn` classes (`--bought` / `--available` /
`--unavailable`, `--color-2/3/4` per column) and the per-column background gradient
(`c-infinity-upgrade-grid__column--background`, lit segments for owned cells). Each
button shows description, effect value, and IP cost (hidden once bought), mirroring
`InfinityUpgradeButton.vue`. Descriptions/effect text live frontend-side (like the
achievements data), formatted from snapshot values; the engine owns owned-state,
affordability, and the effect magnitudes.

Snapshot: `GameView.infinity_upgrades[]` — per upgrade `{ id, is_bought,
can_be_bought, cost, effect_value, description }` (or the raw inputs the frontend
needs to format). New command `buy_infinity_upgrade(id)` + store action.

---

## 5. Testing

- Purchase respects cost + column prerequisite; bit set; IP subtracted.
- Each effect: `buy10Mult` → buy-10 base 2.2; `dimboostMult` → boost power 2.5;
  `resetBoost` → both requirements −9; `galaxyBoost` → tickspeed matches the
  ×2-`effects` formula; dim mults hit the right tiers; `unspentIPMult` tier 0 only;
  time mults scale with the records; `skipReset*` restore boosts/galaxy on crunch.
- ipGen accrues IP over ticks and respects the too-slow cutoff.
- Save round-trip of the upgrade set + `part_infinity_point`.

---

## 6. Deferred (scoped follow-up)

- **Bottom row**: `ipMult` (rebuyable ×2 IP, `player.IPMultPurchases`, softcap `e3M`
  / cap `e6M` cost curve) and `ipOffline` (offline-only IP), both gated behind
  **Achievement 41** ("buy 16 Infinity Upgrades"). Needs the `IpMultiplierButton`
  UI and `bestIPMsWithoutMaxAll` for offline. Best done alongside Break Infinity
  (Feature 2.3), where `totalIPMult` and the post-break crunch modal also grow.
- **Charged** infinity upgrades (Ra) — far-future celestial content, not modelled.

*Document generated: 2026-07-03.*
