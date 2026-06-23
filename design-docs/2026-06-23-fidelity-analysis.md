# Fidelity Analysis: Rust Implementation vs Original JavaScript

This document compares the current Rust implementation in `ad-core` against the original
Antimatter Dimensions JavaScript source code, identifying discrepancies and their severity.

## Summary

The current Rust implementation covers pre-infinity mechanics (dimensions, tickspeed,
dim boosts, galaxies, sacrifice, autobuyers) but has **6 significant formula
discrepancies** compared to the original game.

| Issue | Severity | Description |
|-------|----------|-------------|
| Cost scaling per-purchase vs per-10 | 🔴 Critical | Completely wrong cost progression |
| Missing buy-10 multiplier in production | 🔴 Critical | Major multiplier source absent |
| Dim boost formula not tier-dependent | 🟠 Significant | Incorrect boost distribution |
| Tickspeed galaxy formula wrong | 🟠 Significant | Different scaling model |
| Sacrifice formula wrong | 🟡 Moderate | Different formula type |
| Galaxy requirement uses `bought` not `amount` | 🟢 Minor | Functionally equivalent for AD8 |

---

## Issue 1: Cost Scaling Per-Purchase vs Per-10 (🔴 Critical)

### JavaScript (correct)

Cost increases happen **per 10 purchases**, not per individual purchase:

```javascript
// antimatter-dimension.js:362-363
get cost() {
  return this.costScale.calculateCost(Math.floor(this.bought / 10) + this.costBumps);
}
```

The `ExponentialCostScaling` uses `floor(bought / 10)` as the purchase index. This means:
- Purchases 1-10 all cost the **same base cost**
- After every 10th purchase, cost jumps by the cost multiplier
- For AD1: purchases 1-10 cost 10 AM each; purchases 11-20 cost 10,000 each; etc.

### Rust (incorrect)

```rust
// dimensions.rs:16-26
pub fn buy_dimension(&mut self, tier: usize) -> bool {
    if self.antimatter >= self.dimensions[tier].cost {
        self.antimatter -= self.dimensions[tier].cost;
        self.dimensions[tier].amount += Decimal::from_float(1.0);
        self.dimensions[tier].bought += 1;
        self.dimensions[tier].cost *= self.dimensions[tier].cost_multiplier;  // ← WRONG
        true
    } else { false }
}
```

Cost multiplies after **every** purchase. This makes dimensions vastly more expensive
than they should be:

| Purchase # | JS Cost (AD1) | Rust Cost (AD1) |
|-----------|---------------|-----------------|
| 1 | 10 | 10 |
| 2 | 10 | 10,000 |
| 3 | 10 | 10,000,000 |
| 10 | 10 | 10^28 |
| 11 | 10,000 | 10^31 |

**Impact:** The game is ~10^27 times more expensive per dimension in Rust. Progression
is fundamentally broken.

### Fix

Replace per-purchase cost multiplication with a formula:
```rust
fn dimension_cost(&self, tier: usize) -> Decimal {
    let purchase_group = self.dimensions[tier].bought / 10;
    base_cost[tier] * cost_multiplier[tier].pow(purchase_group)
}
```

Remove the mutable `cost` field from `DimensionTier`; compute it dynamically.

---

## Issue 2: Missing Buy-10 Multiplier in Production (🔴 Critical)

### JavaScript (correct)

Every 10 purchases of a dimension grants a multiplicative bonus to that dimension's
production. The base multiplier is 2x:

```javascript
// antimatter-dimension.js:94-100
function applyNDMultipliers(mult, tier) {
  let multiplier = mult.times(GameCache.antimatterDimensionCommonMultiplier.value);
  let buy10Value = Math.floor(AntimatterDimension(tier).bought / 10);
  multiplier = multiplier.times(Decimal.pow(AntimatterDimensions.buyTenMultiplier, buy10Value));
  multiplier = multiplier.times(DimBoost.multiplierToNDTier(tier));
  // ...
}
```

The `buyTenMultiplier` starts at `2` and is enhanced by upgrades later.

### Rust (incorrect)

The Rust `dimension_multiplier` function has no buy-10 multiplier:

```rust
// dimensions.rs:42-57
pub fn dimension_multiplier(&self, tier: usize) -> Decimal {
    let mut mult = Decimal::from_float(1.0);
    if self.dim_boosts > 0 {
        let boost_mult = Decimal::from_float(DIM_BOOST_MULTIPLIER.powi(self.dim_boosts as i32));
        mult *= boost_mult;
    }
    if tier == 7 {
        mult *= self.sacrifice_multiplier();
    }
    mult
}
```

**Impact:** A player with 30 bought AD1 should have a `2^3 = 8x` multiplier from buy-10.
With 100 bought, it's `2^10 = 1024x`. This is a major source of exponential growth that's
completely absent.

### Fix

Add the buy-10 multiplier to the dimension multiplier computation:
```rust
let buy10_groups = self.dimensions[tier].bought / 10;
let buy10_mult = Decimal::from_float(2.0).pow_u64(buy10_groups);
mult *= buy10_mult;
```

---

## Issue 3: Dim Boost Formula Not Tier-Dependent (🟠 Significant)

### JavaScript (correct)

The dim boost multiplier is tier-dependent — higher tiers get less benefit:

```javascript
// dimboost.js:41-42
static multiplierToNDTier(tier) {
  const normalBoostMult = DimBoost.power.pow(this.purchasedBoosts + 1 - tier).clampMin(1);
  return normalBoostMult;
}
```

Formula: `power^max(0, boosts + 1 - tier)` where power = 2 (base).

| Boosts | AD1 mult | AD2 mult | AD3 mult | AD4 mult | AD5+ mult |
|--------|----------|----------|----------|----------|-----------|
| 1 | 2^1=2 | 1 | 1 | 1 | 1 |
| 4 | 2^4=16 | 2^3=8 | 2^2=4 | 2^1=2 | 1 |
| 8 | 2^8=256 | 2^7=128 | 2^6=64 | 2^5=32 | 2^4=16 |

### Rust (incorrect)

The Rust implementation applies the same multiplier to all tiers:

```rust
// dimensions.rs:46-49
if self.dim_boosts > 0 {
    let boost_mult = Decimal::from_float(DIM_BOOST_MULTIPLIER.powi(self.dim_boosts as i32));
    mult *= boost_mult;
}
```

Formula: `2^boosts` for all tiers uniformly.

**Impact:** Higher tiers get much more benefit than they should. AD8 gets the same
multiplier as AD1, when it should get significantly less. This distorts the balance between
tiers.

### Fix

```rust
pub fn dimension_multiplier(&self, tier: usize) -> Decimal {
    // tier is 0-indexed, JS is 1-indexed
    let js_tier = tier + 1;
    let exponent = (self.dim_boosts as i64 + 1 - js_tier as i64).max(0);
    let boost_mult = Decimal::from_float(DIM_BOOST_MULTIPLIER).pow_i64(exponent);
    // ...
}
```

---

## Issue 4: Tickspeed Galaxy Formula (🟠 Significant)

### JavaScript (correct)

The tickspeed multiplier per purchase uses two different formulas based on galaxy count:

**Pre-3 galaxies (linear):**
```javascript
// tickspeed.js:44-53
if (galaxies < 3) {
  let baseMultiplier = 1 / 1.1245;  // ≈ 0.8893
  if (player.galaxies === 1) baseMultiplier = 1 / 1.11888888;  // ≈ 0.8937
  if (player.galaxies === 2) baseMultiplier = 1 / 1.11267177;  // ≈ 0.8988
  const perGalaxy = 0.02 * effects;
  return DC.D0_01.clampMin(baseMultiplier - (galaxies * perGalaxy));
}
```

**3+ galaxies (exponential):**
```javascript
// tickspeed.js:55-66
let baseMultiplier = 0.8;
galaxies -= 2;
galaxies *= effects;
const perGalaxy = DC.D0_965;
return perGalaxy.pow(galaxies - 2).times(baseMultiplier);
```

For the simple pre-infinity case (no effects multiplier, effects=1):
```
multiplier = 0.965^(galaxies - 4) * 0.8
```

| Galaxies | JS multiplier | Rust multiplier |
|----------|---------------|-----------------|
| 0 | 0.8893 | 0.88 |
| 1 | 0.8737 | 0.86 |
| 2 | 0.8588 | 0.84 |
| 3 | 0.829 | 0.82 |
| 5 | 0.772 | 0.78 |
| 10 | 0.647 | 0.68 |
| 20 | 0.434 | 0.48 |
| 40 | 0.195 | 0.08 |

### Rust (incorrect)

```rust
// tickspeed.rs:37-40
pub fn tickspeed_purchase_multiplier(&self) -> f64 {
    let reduction = self.galaxies as f64 * GALAXY_TICKSPEED_REDUCTION;  // 0.02
    (TICKSPEED_MULTIPLIER - reduction).max(TICKSPEED_MULTIPLIER_MIN)   // 0.88 - gal*0.02
}
```

Linear formula: `max(0.88 - galaxies * 0.02, 0.02)`.

**Impact:** The linear formula hits the floor (0.02) at 43 galaxies, after which more
galaxies have no effect. The JS exponential formula never hits a floor — galaxies always
provide benefit. At high galaxy counts (>40), the Rust formula is dramatically less
powerful. At low counts (< 10), the Rust formula is slightly wrong but close.

### Fix

Implement the two-branch formula:
```rust
pub fn tickspeed_purchase_multiplier(&self) -> Decimal {
    let galaxies = self.galaxies as f64;  // Later: + replicanti + tachyon
    if galaxies < 3.0 {
        let base = match self.galaxies {
            0 => 1.0 / 1.1245,
            1 => 1.0 / 1.11888888,
            2 => 1.0 / 1.11267177,
            _ => unreachable!(),
        };
        Decimal::from_float((base - galaxies * 0.02).max(0.01))
    } else {
        let adjusted = galaxies - 2.0;  // no effects yet
        Decimal::from_float(0.8) * Decimal::from_float(0.965).pow_f64(adjusted - 2.0)
    }
}
```

---

## Issue 5: Sacrifice Formula (🟡 Moderate)

### JavaScript (correct)

Pre-infinity formula (before IC2 is completed):
```javascript
// sacrifice.js:74-76
prePowerSacrificeMult = new Decimal(
  (nd1Amount.log10() / 10) / Math.max(sacrificed.log10() / 10, 1)
);
return prePowerSacrificeMult.clampMin(1).pow(this.sacrificeExponent);
```

Where `sacrificeExponent = 2` (base, modified by achievements).

This means: `sacrifice_mult = max(1, (log10(AD1_amount) / 10) / max(log10(total_sacrificed) / 10, 1))^2`

The key insight: the formula uses `log10` of the CURRENT 1st dimension amount divided by
`log10` of total sacrificed. Sacrifice only improves when your current AD1 amount
significantly exceeds what you've previously sacrificed.

### Rust (incorrect)

```rust
// sacrifice.rs / dimensions.rs:63-70
pub fn sacrifice_multiplier(&self) -> Decimal {
    if self.sacrificed <= Decimal::from_float(SACRIFICE_MIN_AMOUNT) {
        return Decimal::from_float(1.0);
    }
    let ratio = self.sacrificed / Decimal::from_float(SACRIFICE_MIN_AMOUNT);  // sacrificed/10
    let exponent = Decimal::from_float(SACRIFICE_EXPONENT);  // 2.0
    ratio.pow(&exponent)
}
```

Formula: `(total_sacrificed / 10)^2`

**Differences:**
1. JS uses a **ratio** of current AD1 to previous sacrifice (comparative). Rust uses the
   **absolute** total sacrificed.
2. JS uses **log-space** comparison. Rust uses linear division.
3. JS formula means sacrifice is only worthwhile when AD1 >> previous sacrifice. Rust
   formula means more sacrifice always = better.

**Impact:** The gameplay loop is different. In JS, you sacrifice when AD1 has grown enough
to make the ratio favorable. In Rust, the multiplier always grows. The Rust formula is
much simpler but doesn't capture the strategic decision of "when to sacrifice."

Also note: the JS sacrifice multiplier (`Sacrifice.totalBoost`) is calculated as the
**next** boost (comparing new sacrifice to previous), not the **current** total. This is
a running product of successive sacrifice ratios:
```javascript
// sacrifice.js - Sacrifice.totalBoost uses `nextBoost` applied multiplicatively
```

### Fix

Implement the log-ratio formula:
```rust
pub fn sacrifice_boost_if_sacrificed(&self) -> Decimal {
    let ad1_amount = self.dimensions[0].amount;
    if ad1_amount <= Decimal::ONE {
        return Decimal::ONE;
    }

    let log_ad1 = ad1_amount.log10() / 10.0;
    let log_sacrificed = (self.sacrificed.log10() / 10.0).max(1.0);
    let ratio = Decimal::from_float(log_ad1 / log_sacrificed);
    ratio.max(Decimal::ONE).pow_f64(SACRIFICE_EXPONENT)
}
```

**Note:** The full sacrifice system uses a running product (`totalBoost`), where each
sacrifice multiplies by `nextBoost`. This requires tracking the cumulative multiplier
rather than recomputing from total sacrificed.

---

## Issue 6: Galaxy Requirement Uses `bought` (🟢 Minor)

### JavaScript

```javascript
// galaxy.js:11-14
get isSatisfied() {
  const dimension = AntimatterDimension(this.tier);
  return dimension.totalAmount.gte(this.amount);
}
```

Uses `totalAmount` (which for AD8 = amount from purchases, since nothing produces AD8).

### Rust

```rust
// galaxy.rs:24
self.dimensions[7].bought >= self.galaxy_requirement()
```

Uses `bought` count.

**Impact:** Functionally equivalent for AD8 because nothing produces the 8th dimension
(it's the highest tier), so `amount == bought` in practice. Similarly, dim boost
requirements for the first 4 boosts check tiers that aren't yet being produced.

However, for semantic correctness and future-proofing, the requirement should check
`amount` (or `floor(amount)` to be precise).

---

## Additional Observations

### What's Correct

1. **Production chain order** (AD8→AD7→...→AD1→AM) — ✅ correct
2. **Initial game state** (10 AM, 4 unlocked dims) — ✅ correct
3. **Dim boost unlock progression** (boost 1-4 unlock dims 5-8) — ✅ correct
4. **Galaxy requirement base formula** (80 + 60*N) — ✅ correct for pre-infinity
5. **Sacrifice resets dims 1-7, preserves dim 8** — ✅ correct
6. **Galaxy resets dim boosts** — ✅ correct
7. **Autobuyer interval/timer system** — ✅ reasonable abstraction
8. **Tickspeed cost formula** (1000 × 10^bought) — ✅ correct

### What's Missing (Not Yet Implemented)

These are not bugs — they're features from later phases that haven't been built yet:

1. **Common multiplier** (achievements, infinity power, break upgrades, time studies)
2. **Challenge modifiers** (normal/infinity/eternity challenges)
3. **Infinity prestige** (Big Crunch, IP, infinity upgrades)
4. **ExponentialCostScaling class** (for proper bulk-buy math)
5. **Distant/remote galaxy scaling** (quadratic/exponential past galaxy 100/800)
6. **Free tickspeed** (from time shards)
7. **`costBumps` mechanism** (challenge-related cost increases)
8. **NC5 tickspeed start** (starts with higher base in Normal Challenge 5)
9. **NC10 galaxy changes** (tier 6 requirement instead of 8, base cost 99)

---

## Recommended Fix Priority

1. **Cost model** — Fix immediately. Everything depends on correct cost scaling.
2. **Buy-10 multiplier** — Fix immediately. Core progression mechanic.
3. **Dim boost tier formula** — Fix next. Affects balance between dimensions.
4. **Tickspeed galaxy formula** — Fix next. Important for multi-galaxy runs.
5. **Sacrifice formula** — Fix when implementing the full sacrifice system.

---

## Tickspeed Integration Note

The JS production formula is:
```javascript
production = amount * multiplier * Tickspeed.perSecond
```

Where:
```javascript
Tickspeed.perSecond = 1000 / Tickspeed.current
Tickspeed.current = 1000 * getTickSpeedMultiplier()^totalUpgrades  // simplified
```

So:
```
Tickspeed.perSecond = 1000 / (1000 * mult^upgrades) = 1 / mult^upgrades = (1/mult)^upgrades
```

Since `mult < 1`, `1/mult > 1`, so `perSecond` grows exponentially with upgrades.

The Rust implementation computes the same thing via:
```rust
tickspeed_effect = INITIAL_TICKSPEED_MS / current_tickspeed_ms
             = 1000 / (1000 * multiplier^bought)
             = 1 / multiplier^bought
```

This structure is correct — the issue is only with how `multiplier` is computed (Issue 4).

---

*Document generated: 2026-06-23*
