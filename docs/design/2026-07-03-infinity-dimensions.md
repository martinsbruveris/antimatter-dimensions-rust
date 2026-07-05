# Infinity Dimensions — Feature 3.1

8 tiers of dimensions bought with Infinity Points. They produce **Infinity Power**,
which gives a large multiplier to *all* Antimatter Dimensions. IDs persist across a
Big Crunch (only their *amount* + Infinity Power reset); a full reset waits for
Eternity.

Original: `core/dimensions/infinity-dimension.js`, and the Infinity-Power term in
`antimatterDimensionCommonMultiplier` (`infinityPower^powerConversionRate`).

---

## 1. Data (per tier, 1-indexed)

| tier | unlock AM | base cost (IP) | cost ×/purchase | power ×/10 |
|------|-----------|----------------|-----------------|------------|
| 1 | 1e1100 | 1e8 | 1e3 | 50 |
| 2 | 1e1900 | 1e9 | 1e6 | 30 |
| 3 | 1e2400 | 1e10 | 1e8 | 10 |
| 4 | 1e10500 | 1e20 | 1e10 | 5 |
| 5 | 1e30000 | 1e140 | 1e15 | 5 |
| 6 | 1e45000 | 1e200 | 1e20 | 5 |
| 7 | 1e54000 | 1e250 | 1e25 | 5 |
| 8 | 1e60000 | 1e280 | 1e30 | 5 |

ID1 additionally needs `IP ≥ 1e8` to unlock (pre-Eternity `hasIPUnlock`). Unlock
gate: `records.max_am_this_eternity ≥ unlockAM` (the same field IC unlocks use).
Purchase hardcap: 2,000,000 purchases (tiers 1–7); tier 8 uncapped.

---

## 2. State

```rust
pub struct InfinityDimension {   // × 8, on GameState
    amount: Decimal,      // grows from higher-tier production; reset to base on crunch
    base_amount: u64,     // 10 × purchases (the bought base); persists across crunch
    cost: Decimal,        // next-purchase IP cost; persists
    is_unlocked: bool,    // persists
}
pub infinity_power: Decimal,     // the produced currency; reset on crunch
```

`purchases = base_amount / 10`. Each purchase gives **10** IDs (`amount += 10`,
`base_amount += 10`). `base_amount`/purchases stay ≤ 2e7, so `u64` is exact.

---

## 3. Purchase & unlock

- **unlock(tier)**: `is_unlocked = true` once `max_am_this_eternity ≥ unlockAM`
  (and, for tier 1, `IP ≥ 1e8`). Buying a locked-but-unlockable ID unlocks it.
- **buy_single(tier)**: if unlocked, affordable (`IP ≥ cost`), and not capped:
  spend `cost` IP; `cost = round(cost × costMultiplier)`; `amount += 10`,
  `base_amount += 10`.
- **buy_max(tier)**: `LinearCostScaling` — the largest affordable run up to the
  hardcap. (Implement as a geometric-series bulk buy or a repeated single-buy loop,
  matching the ADs' `buy_max` approach.)

---

## 4. Production & the Infinity-Power effect

Chain (mirrors `InfinityDimensions.tick`, `diff/10` for dimension→dimension like the
ADs): `for tier in (8..=2): ID(tier).produce(ID(tier-1), diff/10)`, then `ID1`
produces Infinity Power (`diff`). Per-ID production = `amount × multiplier`.

**ID multiplier** = `commonMult × powerMultiplier^purchases`, where `commonMult`
folds the completed **IC1 reward** (`×1.3^(IC completed count)`) and **IC6 reward**
(`tickspeed_per_second^0.0005`) — the two rewards deferred from 2.7 — plus (later)
the Replicanti multiplier (3.2).

**Infinity-Power → AD multiplier**: in the AD common multiplier,
`× infinity_power ^ 7` clamped to ≥ 1 (`powerConversionRate = 7`). Wired into
`dimension_multiplier` alongside the existing common terms.

---

## 5. Reset semantics

- **Big Crunch** (`big_crunch_reset`): `infinity_power = 0`; each ID `amount =
  base_amount`. Purchases/cost/unlock **persist**.
- **Eternity** (later): full reset (`base_amount = 0`, `cost = base`, locked).

---

## 6. Save / load

`player.dimensions.infinity[]` → `{ amount, cost, baseAmount, isUnlocked }` per tier;
`player.infinityPower` → `infinity_power`. All present in the template.

---

## 7. UI

An **Infinity → Infinity Dimensions** subtab (shown once ID1 is unlocked, or
Infinity is broken — practically once you can reach 1e1100 this eternity): 8 rows
(amount, multiplier, cost, unlock/buy button, buy-max), plus the Infinity-Power
readout and its AD multiplier. Vendored `infinity-dimensions` styling. `buy_infinity
_dimension(tier)` / `buy_max_infinity_dimensions` commands + snapshot.

---

## 8. Incremental plan

1. **Engine**: state, data, unlock/buy(single/max), production chain + Infinity
   Power, the AD `^7` effect, IC1/IC6 ID rewards, crunch reset, save/load. Tests.
   Commit.
2. **UI**: the Infinity Dimensions subtab. Commit.

## 9. Open questions

- Autobuyers for IDs (per-tier + toggle-all) are Eternity-milestone gated; deferred.
- `commonMult`'s achievement/time-study/eternity terms are later features (= ×1 now).

*Document generated: 2026-07-03.*
