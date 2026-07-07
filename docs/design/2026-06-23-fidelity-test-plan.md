---
status: Superseeded
---

# Fidelity Test Plan: Pre-Infinity & Infinity Stages

This document defines test scenarios for the `ad-fidelity` crate to verify that the Rust
implementation matches the original JavaScript game. Tests cover pre-infinity and
infinity stages only (Phases 1-3 from the feature decomposition).

## Testing Methodology

### Approach

Each test scenario specifies:
1. **Initial state** — Starting conditions (fresh game or specific game state)
2. **Actions** — Sequence of player actions or simulation steps
3. **Assertions** — Expected outcomes with tolerance

### Tolerance

Floating-point arithmetic differs between JS and Rust. Use log-space relative tolerance:
```
|log10(rust_value) - log10(js_value)| < epsilon
```
Default epsilon: `1e-10` for exact formula matches. For multi-step simulations
(accumulated error): `1e-6`.

### Test Data Source

Expected values should be extracted from the JS game by running equivalent scenarios. The
`ad-fidelity` crate should include a JS runner (via embedded V8 or pre-computed JSON
fixtures) to generate reference data.

---

## Section 1: Dimension Cost Model

### 1.1 Basic Cost Calculation

Verify that dimension cost is computed from `floor(bought / 10)`, not per-purchase.

| Tier | bought | Expected Cost |
|------|--------|---------------|
| 1 | 0 | 10 |
| 1 | 1 | 10 |
| 1 | 9 | 10 |
| 1 | 10 | 10,000 (= 10 × 1e3) |
| 1 | 11 | 10,000 |
| 1 | 19 | 10,000 |
| 1 | 20 | 10,000,000 (= 10 × 1e3^2) |
| 1 | 30 | 10^10 |
| 2 | 0 | 100 |
| 2 | 9 | 100 |
| 2 | 10 | 1,000,000 (= 100 × 1e4) |
| 2 | 20 | 10^10 |
| 8 | 0 | 1e24 |
| 8 | 10 | 1e39 (= 1e24 × 1e15) |
| 8 | 80 | 1e24 × (1e15)^8 = 1e144 |

**Formula:** `cost(tier, bought) = baseCost[tier] ×
costMultiplier[tier]^floor(bought/10)`

**Static data:**
```
baseCost       = [10, 100, 1e4, 1e6, 1e9, 1e13, 1e18, 1e24]
costMultiplier = [1e3, 1e4, 1e5, 1e6, 1e8, 1e10, 1e12, 1e15]
```

### 1.2 Cost After Purchasing

Verify that buying dimensions 1-9 of a tier doesn't change the cost, but buying the 10th
does. Test sequence:

```
state: fresh game, antimatter = 1e30
action: buy AD1 ten times
assert: cost after 1st buy == 10
assert: cost after 9th buy == 10
assert: cost after 10th buy == 10,000
assert: bought == 10
assert: amount == 10
```

### 1.3 ExponentialCostScaling (Pre-Scaling Region)

Before the `scalingCostThreshold` (set to `Number.MAX_VALUE` for dimensions in pre-
infinity), cost is purely exponential:

```
log10(cost) = floor(bought/10) × log10(costMultiplier) + log10(baseCost)
```

For tickspeed (scales every purchase, no per-10 grouping):
```
log10(cost) = bought × log10(10) + log10(1000)
            = bought + 3
```

| Tickspeed bought | Expected Cost |
|-----------------|---------------|
| 0 | 1,000 |
| 1 | 10,000 |
| 2 | 100,000 |
| 10 | 10^13 |
| 50 | 10^53 |

### 1.4 Bulk-Buy (Buy Until 10)

In the original game, the standard purchase mode is "buy until next set of 10":
```
state: AD1 bought = 7, antimatter = 100
action: buy_until_10(AD1)
assert: buys 3 more AD1 (bought becomes 10)
assert: cost paid = 3 × 10 = 30
assert: new cost = 10,000
```

---

## Section 2: Buy-10 Multiplier

### 2.1 Base Buy-10 Multiplier

Every complete set of 10 purchases grants a 2x production multiplier.

```
state: AD1 bought = 0, dim_boosts = 0, galaxies = 0
assert: buy10_multiplier(AD1) = 2^0 = 1

state: AD1 bought = 10
assert: buy10_multiplier(AD1) = 2^1 = 2

state: AD1 bought = 30
assert: buy10_multiplier(AD1) = 2^3 = 8

state: AD1 bought = 100
assert: buy10_multiplier(AD1) = 2^10 = 1024
```

### 2.2 Buy-10 in Production Formula

Production per second for a dimension:
```
production = amount × multiplier × tickspeed_per_second
```
where `multiplier` includes the buy-10 multiplier:
```
multiplier = buy10_mult × dimboost_mult × sacrifice_mult(if tier 8) × common_mult
```

Test:
```
state: AD1 amount = 10, bought = 10, no boosts/galaxies/tickspeed
assert: AD1 multiplier includes factor of 2 (from buy10)
assert: production_per_second(AD1) = 10 × 2 × 1 = 20 AM/s
```

### 2.3 Buy-10 Multiplier Upgrade (Infinity Upgrade)

After purchasing the `buy10Mult` infinity upgrade, the buy-10 multiplier becomes `2 ×
InfinityUpgrade.buy10Mult.effect` where the effect is based on dim boosts + galaxies.

```
state: buy10Mult infinity upgrade purchased, dim_boosts = 5, galaxies = 2
assert: buy10_multiplier = function(dim_boosts, galaxies)  [compute from JS]
```

---

## Section 3: Dimension Boost Multiplier

### 3.1 Tier-Dependent Multiplier

Formula: `DimBoost.power^max(0, purchasedBoosts + 1 - tier)` where power = 2 (base).

```
state: dim_boosts = 1
assert: multiplier(tier=1) = 2^max(0, 1+1-1) = 2^1 = 2
assert: multiplier(tier=2) = 2^max(0, 1+1-2) = 2^0 = 1
assert: multiplier(tier=3) = 2^max(0, 1+1-3) = 2^0 = 1 (clamped)

state: dim_boosts = 4
assert: multiplier(tier=1) = 2^4 = 16
assert: multiplier(tier=2) = 2^3 = 8
assert: multiplier(tier=3) = 2^2 = 4
assert: multiplier(tier=4) = 2^1 = 2
assert: multiplier(tier=5) = 2^0 = 1

state: dim_boosts = 10
assert: multiplier(tier=1) = 2^10 = 1024
assert: multiplier(tier=5) = 2^6 = 64
assert: multiplier(tier=8) = 2^3 = 8
```

### 3.2 Boost Requirements

```
// First 4 boosts require progressively higher tiers
state: dim_boosts = 0
assert: requirement = (tier=4, amount=20)

state: dim_boosts = 1
assert: requirement = (tier=5, amount=20)

state: dim_boosts = 2
assert: requirement = (tier=6, amount=20)

state: dim_boosts = 3
assert: requirement = (tier=7, amount=20)

// After unlocking all 8 dims, boost 5+ requires 8th dimension
// amount = 20 + (targetResets - 5) × 15
// targetResets = currentBoosts + 1
state: dim_boosts = 4
assert: requirement = (tier=8, amount=20)  // 20 + (5-5)*15 = 20

state: dim_boosts = 5
assert: requirement = (tier=8, amount=35)  // 20 + (6-5)*15 = 35

state: dim_boosts = 10
assert: requirement = (tier=8, amount=95)  // 20 + (11-5)*15 = 110? 
// Actually: targetResets=11, amount = 20 + (11-5)*15 = 20 + 90 = 110
// Wait, let me recalculate:
// targetResets = purchasedBoosts + 1 = 10 + 1 = 11
// tier = min(11 + 3, 8) = 8
// tier === 8: amount += round((11 - 5) * 15) = 90
// total amount = 20 + 90 = 110
assert: requirement = (tier=8, amount=110)
```

### 3.3 Dim Boost Requirement Uses totalAmount

The requirement checks dimension `totalAmount` (not `bought`). For pre-infinity with no
continuum, `totalAmount ≈ amount` (includes production from higher tiers).

```
state: dim_boosts = 0, AD4 bought = 15, AD4 amount = 22.5 (produced by AD5)
assert: can_dim_boost() = true  // amount >= 20, even though bought < 20
```

---

## Section 4: Tickspeed

### 4.1 Galaxy Effect on Tickspeed Multiplier

**Pre-3 galaxies (special hardcoded values):**

| Player Galaxies | Base Multiplier | Effective Multiplier (per purchase) |
|----------------|-----------------|--------------------------------------|
| 0 | 1/1.1245 ≈ 0.88936 | 0.88936 |
| 1 | 1/1.11889 ≈ 0.89376 | 0.89376 - 1×0.02 = 0.87376 |
| 2 | 1/1.11267 ≈ 0.89876 | 0.89876 - 2×0.02 = 0.85876 |

Wait — re-reading the JS code: for `galaxies < 3`, `galaxies` = `effectiveBaseGalaxies()`
which equals `player.galaxies` in pre-infinity (no replicanti/tachyon). The formula is:
```
return max(0.01, baseMultiplier - galaxies × perGalaxy)
```
where `perGalaxy = 0.02 × effects` (effects = 1 in pre-infinity).

So with the special `baseMultiplier` values per galaxy count:
| Player Galaxies | Formula | Result |
|----------------|---------|--------|
| 0 | 0.88936 - 0×0.02 | 0.88936 |
| 1 | 0.89376 - 1×0.02 | 0.87376 |
| 2 | 0.89876 - 2×0.02 | 0.85876 |

**3+ galaxies (exponential formula):**
```
multiplier = 0.965^(adjustedGalaxies - 2) × 0.8
adjustedGalaxies = (effectiveGalaxies - 2) × effects
```
In pre-infinity (effects = 1, effective = player.galaxies):
```
multiplier = 0.965^(galaxies - 4) × 0.8
```

| Galaxies | Formula | Result |
|----------|---------|--------|
| 3 | 0.965^(-1) × 0.8 | 0.82902 |
| 4 | 0.965^0 × 0.8 | 0.80000 |
| 5 | 0.965^1 × 0.8 | 0.77200 |
| 10 | 0.965^6 × 0.8 | 0.64729 |
| 20 | 0.965^16 × 0.8 | 0.44771 |
| 50 | 0.965^46 × 0.8 | 0.15413 |

### 4.2 Tickspeed Value

```
Tickspeed.current = baseValue = 1000 × multiplier^totalUpgrades
Tickspeed.perSecond = 1000 / current
```

Test cases:
```
state: galaxies=0, tickspeed_bought=0
assert: current_tickspeed_ms ≈ 1000
assert: ticks_per_second ≈ 1

state: galaxies=0, tickspeed_bought=1
assert: current_tickspeed_ms ≈ 1000 × 0.88936 ≈ 889.36
assert: ticks_per_second ≈ 1000/889.36 ≈ 1.1244

state: galaxies=0, tickspeed_bought=10
assert: current_tickspeed_ms ≈ 1000 × 0.88936^10 ≈ 307.88
assert: ticks_per_second ≈ 3.248

state: galaxies=5, tickspeed_bought=10
assert: current_tickspeed_ms ≈ 1000 × 0.772^10 ≈ 71.89
assert: ticks_per_second ≈ 13.91

state: galaxies=10, tickspeed_bought=50
assert: current_tickspeed_ms ≈ 1000 × 0.64729^50 ≈ very small
assert: ticks_per_second ≈ very large  [compute exact]
```

### 4.3 Tickspeed Cost

Tickspeed cost scales per-purchase (not per-10):
```
cost = 1000 × 10^bought
```

| bought | cost |
|--------|------|
| 0 | 1,000 |
| 1 | 10,000 |
| 5 | 10^8 |
| 10 | 10^13 |

### 4.4 Tickspeed Integration with Production

```
production_per_second(tier) = amount × multiplier × tickspeed_per_second
```

Test:
```
state: AD1 amount=10, bought=10, dim_boosts=0, galaxies=0, tickspeed_bought=5
multiplier = buy10_mult × dimboost_mult = 2 × 1 = 2
ticks_per_second = 1000 / (1000 × 0.88936^5) = 1000/556.0 ≈ 1.799
production = 10 × 2 × 1.799 = 35.97 AM/s
```

---

## Section 5: Antimatter Galaxies

### 5.1 Requirement Formula (Normal Scaling)

```
requirement = 80 + galaxies × 60
```

| Current Galaxies | Requirement (8th dim totalAmount) |
|-----------------|-----------------------------------|
| 0 | 80 |
| 1 | 140 |
| 2 | 200 |
| 5 | 380 |
| 10 | 680 |

### 5.2 Galaxy Requirement Uses totalAmount

```
state: galaxies=0, AD8 bought=70, AD8 amount=80.0 (some from production... 
       actually nothing produces AD8 so amount = bought = 70)
```

Note: Nothing produces AD8 (it's the highest tier), so `totalAmount = amount = bought`.
This means for galaxy requirements, `bought` and `totalAmount` are interchangeable.

### 5.3 Distant Scaling (Galaxy ≥ 100)

The `costScalingStart` is 100 in pre-infinity (no time studies or EC5 reward).

```
// For galaxy count ≥ costScalingStart:
galaxiesBeforeDistant = galaxies - costScalingStart + 1
amount += galaxiesBeforeDistant^2 + galaxiesBeforeDistant
```

| Galaxy # | Base (80+60×g) | Distant Extra | Total Requirement |
|----------|----------------|---------------|-------------------|
| 100 | 6080 | 1^2+1=2 | 6082 |
| 101 | 6140 | 2^2+2=6 | 6146 |
| 110 | 6680 | 11^2+11=132 | 6812 |
| 150 | 9080 | 51^2+51=2652 | 11732 |
| 200 | 12080 | 101^2+101=10302 | 22382 |

### 5.4 Remote Scaling (Galaxy ≥ 800)

```
// Remote start = 800 (default, modified by Reality Upgrade)
amount *= 1.002^(galaxies - 799)
```

| Galaxy # | Pre-remote | Remote Multiplier | Total |
|----------|-----------|-------------------|-------|
| 800 | base+distant | 1.002^1 | × 1.002 |
| 850 | base+distant | 1.002^51 | × 1.107 |
| 1000 | base+distant | 1.002^201 | × 1.493 |

### 5.5 Galaxy Reset

Verify that galaxy reset:
1. Increments galaxy count
2. Resets dim_boosts to 0
3. Calls softReset(0) which resets:
   - All dimension amounts to 0, bought to 0, costBumps to 0
   - Tickspeed purchases to 0
   - Antimatter to starting value (10)
   - Sacrificed amount to 0

```
state: galaxies=0, dim_boosts=5, AD8.bought=80, tickspeed.bought=20, 
       antimatter=1e200, sacrificed=1e50
action: buy_galaxy()
assert: galaxies = 1
assert: dim_boosts = 0
assert: AD8.bought = 0, AD8.amount = 0
assert: tickspeed.bought = 0
assert: antimatter = 10
assert: sacrificed = 0
```

---

## Section 6: Sacrifice

### 6.1 Unlock Condition

In the JS, sacrifice requires `DimBoost.purchasedBoosts > 4` (i.e., 5+ boosts). The Rust
currently unlocks it at 1 boost. This is a discrepancy.

```
state: dim_boosts = 4
assert: can_sacrifice() = false

state: dim_boosts = 5
assert: can_sacrifice() = true (if AD8 totalAmount > 0 and nextBoost > 1)
```

### 6.2 Pre-Infinity totalBoost Formula

The `Sacrifice.totalBoost` (the actual multiplier applied to AD8 production) uses:
```
prePowerBoost = max(1, log10(total_sacrificed) / 10)
totalBoost = prePowerBoost^sacrificeExponent
sacrificeExponent = 2 (base, no achievements in pre-infinity)
```

| total_sacrificed | log10/10 | totalBoost (^2) |
|-----------------|----------|-----------------|
| 1 | 0 → clamped to 1 | 1 |
| 10 | 0.1 → clamped to 1 | 1 |
| 1e10 | 1 | 1 |
| 1e20 | 2 | 4 |
| 1e50 | 5 | 25 |
| 1e100 | 10 | 100 |
| 1e200 | 20 | 400 |
| 1e1000 | 100 | 10,000 |

### 6.3 nextBoost Formula (Should You Sacrifice?)

The `nextBoost` determines the gain from a new sacrifice:
```
prePowerMult = max(1, (log10(AD1_amount) / 10) / max(log10(total_sacrificed) / 10, 1))
nextBoost = prePowerMult^2
```

Test cases:
```
state: AD1_amount = 1e100, total_sacrificed = 1e50
prePowerMult = (100/10) / max(50/10, 1) = 10 / 5 = 2
nextBoost = 2^2 = 4

state: AD1_amount = 1e200, total_sacrificed = 1e100
prePowerMult = (200/10) / (100/10) = 20/10 = 2
nextBoost = 4

state: AD1_amount = 1e50, total_sacrificed = 1e100
prePowerMult = (50/10) / (100/10) = 5/10 = 0.5 → clamped to 1
nextBoost = 1  (sacrifice not worth doing)
```

### 6.4 Sacrifice Reset Behavior

```
state: AD1.amount = 1e100, AD2.amount = 1e80, ..., AD7.amount = 1e20, AD8.amount = 100
       sacrificed = 1e50, dim_boosts = 5
action: sacrifice()
assert: sacrificed = 1e50 + 1e100 ≈ 1e100  (AD1 amount added)
assert: AD1.amount = 0 (reset)
assert: AD2.amount = 0 (reset)
assert: AD7.amount = 0 (reset)
assert: AD8.amount = 100 (preserved!)
assert: AD1.bought unchanged
assert: AD1.cost unchanged
assert: dim_boosts unchanged
assert: antimatter unchanged
```

Note: In JS, `AntimatterDimensions.resetAmountUpToTier(7)` resets amounts for dims 1-7.
Dim 8 amount is preserved. Bought counts and costs are preserved for all dims.

---

## Section 7: Production Simulation

### 7.1 Single Dimension Production

```
state: fresh game, AD1.amount=1 (bought 1), no tickspeed/boosts/galaxies
       antimatter = 0 (spent on purchase)
action: tick(1000ms)
assert: antimatter += 1 × 1 × 1 = 1.0  (amount × mult × ticks_per_sec × seconds)
```

### 7.2 Multi-Dimension Chain

```
state: AD1.amount=1, AD2.amount=1, AD3.amount=1, AD4.amount=1
       no tickspeed/boosts/galaxies, antimatter=0
action: tick(1000ms)
assert: antimatter += AD1 production = 1 × 1 × 1 = 1
assert: AD1.amount += AD2 production = 1 × 1 × 1 = 1 → AD1.amount = 2
assert: AD2.amount += AD3 production = 1 × 1 × 1 = 1 → AD2.amount = 2
assert: AD3.amount += AD4 production = 1 × 1 × 1 = 1 → AD3.amount = 2
```

### 7.3 Production with Tickspeed

```
state: AD1.amount=10, bought=10, tickspeed_bought=5, galaxies=0
       buy10_mult = 2^1 = 2
       dimboost_mult = 1 (no boosts... wait, need boosts for tier 5)
       Actually: dim_boosts=0, so dimboost_mult(tier=1) = 2^max(0, 0+1-1) = 2^0 = 1
       tickspeed_multiplier = 0.88936
       ticks_per_second = 1000 / (1000 × 0.88936^5) = 1/0.55607 ≈ 1.7983
       multiplier = 2 × 1 = 2
       production_per_second = 10 × 2 × 1.7983 = 35.97
action: tick(1000ms)
assert: antimatter += 35.97
```

### 7.4 Long Simulation (Exponential Growth)

```
state: AD1.amount=1, AD2.amount=1, no tickspeed/boosts
action: simulate(60000ms, tick_size=100ms)  // 1 minute
assert: antimatter ≈ [computed from JS]
// AD2 produces AD1, AD1 produces AM. This gives exponential growth:
// AD1(t) ≈ 1 + t (linear since AD2=1 produces 1/s into AD1)
// AM(t) ≈ integral of AD1 from 0 to t ≈ t + t^2/2
// After 60s: AM ≈ 60 + 1800 = 1860
// (Actually with discrete ticks at 100ms, this is an approximation)
```

### 7.5 Full Production Chain (8 Dimensions, No Boosts)

```
state: All 8 dims amount=1 and bought=0, dim_boosts=4 (to unlock all)
       No tickspeed, no galaxies.
       dimboost_mult(tier=1) = 2^max(0,4+1-1) = 2^4 = 16
       dimboost_mult(tier=4) = 2^max(0,4+1-4) = 2^1 = 2
       dimboost_mult(tier=5) = 2^max(0,4+1-5) = 2^0 = 1
action: simulate(10000ms, tick_size=50ms)
assert: antimatter ≈ [computed from JS with same state]
```

---

## Section 8: Dimension Boost Reset

### 8.1 Soft Reset Contents

After a dim boost:
```
state: dim_boosts=0, AD4.bought=20, AD4.amount=25, AD1.amount=1000,
       tickspeed.bought=10, antimatter=1e20, sacrificed=1e10
action: buy_dim_boost()
assert: dim_boosts = 1
assert: AD4.bought = 0
assert: AD4.amount = 0
assert: AD1.bought = 0
assert: AD1.amount = 0
assert: tickspeed.bought = 0
assert: antimatter = 10  (starting value)
assert: sacrificed = 0
assert: galaxies unchanged
```

### 8.2 Multiple Boosts Preserve Galaxies

```
state: galaxies=3, dim_boosts=4, AD8.bought=20 (meets req for boost 5)
action: buy_dim_boost()
assert: galaxies = 3  (preserved)
assert: dim_boosts = 5
assert: all dims reset
```

---

## Section 9: Infinity (Big Crunch)

### 9.1 Pre-Break IP Formula

Before "Break Infinity" is purchased, reaching 1e308 antimatter always gives exactly:
```
IP = floor(308 / div)
div = 308 (base, no upgrades)
IP = floor(308 / 308) = 1
```

```
state: pre-break, antimatter >= 1e308
action: big_crunch()
assert: IP gained = 1
```

### 9.2 Post-Break IP Formula

```
IP = floor(10^(log10(maxAM) / div - 0.75) × totalIPMult)
div = 308 (base)
totalIPMult = 1 (no upgrades initially)
```

| maxAM | log10(maxAM)/308 - 0.75 | Base IP |
|-------|------------------------|---------|
| 1e308 | 308/308 - 0.75 = 0.25 | 10^0.25 ≈ 1.778 → floor = 1 |
| 1e500 | 500/308 - 0.75 ≈ 0.873 | 10^0.873 ≈ 7.46 → floor = 7 |
| 1e1000 | 1000/308 - 0.75 ≈ 2.497 | 10^2.497 ≈ 314 |
| 1e2000 | 2000/308 - 0.75 ≈ 5.744 | 10^5.744 ≈ 554,573 |
| 1e5000 | 5000/308 - 0.75 ≈ 15.48 | 10^15.48 ≈ 3.05e15 |
| 1e10000 | 10000/308 - 0.75 ≈ 31.72 | 10^31.72 |

### 9.3 Infinity Reset

```
state: antimatter=1e500, dim_boosts=20, galaxies=5, tickspeed_bought=100,
       sacrificed=1e200, infinity_points=0, infinity_count=0
action: big_crunch()
assert: infinity_points = [calculated per formula]
assert: infinity_count = 1
assert: antimatter = 10
assert: dim_boosts = 0
assert: galaxies = 0
assert: tickspeed_bought = 0
assert: sacrificed = 0
assert: all dimensions reset
```

### 9.4 IP Accumulation Across Infinities

```
state: IP=5, break=true
action: big_crunch() with maxAM=1e1000
assert: IP = 5 + floor(10^2.497) = 5 + 314 = 319
action: big_crunch() with maxAM=1e2000
assert: IP = 319 + 554573 = 554892
```

---

## Section 10: Infinity Upgrades

### 10.1 Time Multiplier (`timeMult`)

```
effect = pow(totalTimePlayed_minutes / 2, 0.15)
```

Test:
```
state: total_time_played = 60 minutes
assert: timeMult effect = pow(60/2, 0.15) = pow(30, 0.15) ≈ 1.669

state: total_time_played = 600 minutes (10 hours)
assert: timeMult effect = pow(300, 0.15) ≈ 2.454
```

### 10.2 Buy-10 Multiplier Upgrade (`buy10Mult`)

The infinity upgrade provides a flat `×1.1` to the buy-10 multiplier base (added to the
base value of 2):
```
buyTenMultiplier = DC.D2.timesEffectsOf(InfinityUpgrade.buy10Mult, ...)
// InfinityUpgrade.buy10Mult effect = 1.1 (multiplicative)
// So: buyTenMultiplier = 2 × 1.1 = 2.2
```

Test:
```
state: buy10Mult upgrade purchased
assert: buyTenMultiplier = 2 × 1.1 = 2.2
```

### 10.3 Unspent IP Multiplier (`unspentIPMult`)

```
effect = (IP / 2)^1.5 + 1
```

| Unspent IP | Effect |
|-----------|--------|
| 2 | (2/2)^1.5 + 1 = 2.0 |
| 10 | (10/2)^1.5 + 1 = 5^1.5 + 1 ≈ 12.18 |
| 100 | (50)^1.5 + 1 ≈ 354.5 |
| 1e6 | (5e5)^1.5 + 1 ≈ 3.54e8 |
| 1e12 | (5e11)^1.5 + 1 ≈ 3.54e17 |

### 10.4 Dim Boost Multiplier Upgrade (`dimboostMult`)

```
effect = 2.5 (the DimBoost.power becomes 2.5 instead of 2)
```

Test:
```
state: dimboostMult upgrade purchased, dim_boosts=4
assert: DimBoost.power = max(2, 2.5, ...) = 2.5
assert: multiplier(tier=1) = 2.5^4 = 39.0625
assert: multiplier(tier=4) = 2.5^1 = 2.5
```

### 10.5 IP Multiplier (Repeatable, `ipMult`)

```
effect = 2^purchases (each purchase doubles IP gain from all sources)
cost = doubles each purchase (base cost from config)
cap at purchases >= 3,300,000: effect = 1e6
```

| Purchases | Effect | Cumulative IP Multiplier |
|-----------|--------|--------------------------|
| 1 | 2 | ×2 |
| 5 | 32 | ×32 |
| 10 | 1024 | ×1024 |
| 20 | ~1e6 | ×1e6 |

---

## Section 11: Normal Challenges

### 11.1 Challenge Entry/Exit

```
state: normal game, IP=0
action: enter_challenge(1)
assert: soft_reset occurs (antimatter=10, dims reset, etc.)
assert: active_challenge = NC1

action: reach antimatter >= 1e308
assert: challenge_completed(1) = true
assert: exits challenge, performs infinity reset
```

### 11.2 NC2: Production Halt on Purchase

```
state: in NC2
effect: buying anything sets a production penalty (chall2Pow = 0, recovers over time)
action: buy AD1
assert: chall2Pow = 0 (production temporarily stops)
action: tick(1000ms) with no purchases
assert: chall2Pow recovers toward 1
```

### 11.3 NC7: Reduced Buy-10 Multiplier

```
state: in NC7, dim_boosts = 5
assert: buyTenMultiplier = min(2, 1 + totalBoosts/5) = min(2, 1+5/5) = min(2,2) = 2
// With fewer boosts:
state: dim_boosts = 3
assert: buyTenMultiplier = min(2, 1 + 3/5) = min(2, 1.6) = 1.6
```

### 11.4 NC8: Galaxies Disabled, Sacrifice Stronger

```
state: in NC8
assert: can_buy_galaxy() = false
assert: DimBoost.power = 1 (boosts give no multiplier)
assert: sacrifice uses special NC8 formula
```

### 11.5 NC10: Only 6 Dimensions

```
state: in NC10
assert: max_dimensions_unlockable = 6
assert: galaxy requires tier 6 (not tier 8)
assert: galaxy base cost = 99 (not 80)
assert: galaxy cost mult = 90 (not 60)
```

### 11.6 NC12: Modified Production Chain

```
state: in NC12
// Each AD produces 2 tiers lower instead of 1
// AD8→AD6, AD6→AD4, AD4→AD2, AD2→AM
// AD7→AD5, AD5→AD3, AD3→AD1, AD1→AM
// Dims 2,4,6 have powered amounts
assert: production_chain[8] produces dim[6]
assert: production_chain[6] produces dim[4]
assert: AD2.production has amount^1.6
assert: AD4.production has amount^1.4
assert: AD6.production has amount^1.2
```

### 11.7 Challenge Completion Tracking

```
state: completed challenges = [1, 3, 5]
assert: challenge_completed_bits = 0b10101
assert: is_challenge_completed(1) = true
assert: is_challenge_completed(2) = false
```

---

## Section 12: Infinity Challenges

### 12.1 Unlock Thresholds

ICs unlock when max antimatter ever reaches certain thresholds:

| IC | Unlock Threshold |
|----|-----------------|
| 1 | 1e2000 (requires Break Infinity) |
| 2 | 1e10500 |
| 3 | 1e5000 |
| 4 | 1e13000 |
| 5 | 1e18000 |
| 6 | 1e22500 |
| 7 | 1e23000 |
| 8 | 1e28000 |

Note: These require Break Infinity since they exceed 1e308.

### 12.2 IC2 Completion Changes Sacrifice

```
state: IC2 completed
assert: sacrifice formula changes to:
  prePowerBoost = total_sacrificed (not log10/10)
  exponent = 1/120 (not 2)
  totalBoost = max(1, total_sacrificed)^(1/120)
```

| total_sacrificed | totalBoost (IC2 formula) |
|-----------------|--------------------------|
| 1e120 | (1e120)^(1/120) = 10 |
| 1e1200 | (1e1200)^(1/120) = 1e10 |
| 1e12000 | (1e12000)^(1/120) = 1e100 |

### 12.3 IC5: Galaxy Cost Reduction Reward

```
state: IC5 completed
assert: galaxy_requirement reduced by 1 for all galaxies
assert: dim_boost_requirement reduced by 1
```

### 12.4 IC8: Galaxy Threshold Reduction Reward

IC8 reward reduces galaxy cost scaling:
```
state: IC8 completed, IC8.reward effect = [depends on completion stats]
assert: galaxy requirement uses modified formula
```

---

## Section 13: Autobuyers

### 13.1 Basic Autobuyer Timing

```
state: AD1 autobuyer enabled, interval=1000ms, mode=single, antimatter=1e10
action: tick(500ms)
assert: AD1.bought = 0 (not yet fired)

action: tick(500ms)  // total 1000ms
assert: AD1.bought = 1 (fired once)

action: tick(2500ms)  // total 3500ms
assert: AD1.bought = 3 (fired at 2000ms and 3000ms)
```

### 13.2 Autobuyer Buy-Max Mode

```
state: AD1 autobuyer enabled, interval=1000ms, mode=max
       antimatter=1e10, AD1.cost=10
action: tick(1000ms)
assert: AD1.bought = max affordable = [depends on geometric series cost]
// With per-10 cost model: all 10 of first batch cost 10 each = 100 total
// Then next 10 cost 10000 each = 100000 total
// With 1e10 AM: can buy lots
```

### 13.3 Tickspeed Autobuyer

```
state: tickspeed autobuyer enabled, interval=500ms, mode=max, antimatter=1e20
action: tick(1000ms)
assert: tickspeed fired twice (at 500ms and 1000ms)
// First fire: buys max tickspeed
// Second fire: buys max with remaining AM
```

### 13.4 Dim Boost Autobuyer

```
state: dimboost autobuyer enabled, interval=5000ms, limit=10
       AD4.bought=20, dim_boosts=0
action: tick(5000ms)
assert: dim_boosts = 1 (autobuyer fired, conditions met)
```

### 13.5 Galaxy Autobuyer

```
state: galaxy autobuyer enabled, interval=10000ms
       AD8.bought=80, dim_boosts=4, galaxies=0
action: tick(10000ms)
assert: galaxies = 1
assert: dims reset
```

---

## Section 14: Infinity Dimensions

### 14.1 Unlock Conditions

IDs unlock when max antimatter reaches specific thresholds:

| ID Tier | AM Threshold | Base Cost (IP) | Cost Multiplier |
|---------|-------------|----------------|-----------------|
| 1 | 1e1100 | 1e8 | 1e3 |
| 2 | 1e1900 | 1e9 | 1e6 |
| 3 | 1e2400 | 1e10 | 1e8 |
| 4 | 1e10500 | 1e20 | 1e10 |
| 5 | 1e30000 | 1e140 | 1e15 |
| 6 | 1e45000 | 1e200 | 1e20 |
| 7 | 1e54000 | 1e250 | 1e25 |
| 8 | 1e60000 | 1e280 | 1e30 |

### 14.2 ID Purchase Model

IDs are bought in sets of 10 (like ADs):
```
state: ID1 unlocked, IP=1e10
action: buy_id(1)
assert: buys 10 at once
assert: ID1.bought += 10
assert: ID1.amount += 10
assert: cost increases by costMultiplier (1e3 for ID1)
```

### 14.3 Infinity Power

ID1 produces Infinity Power. Infinity Power gives a multiplier to ALL antimatter dims:
```
infinity_power_mult = max(1, infinity_power^conversionRate)
conversionRate = 7 (base)
```

| Infinity Power | Multiplier |
|---------------|------------|
| 1 | 1 |
| 10 | 10^7 = 1e7 |
| 1e10 | 1e70 |
| 1e100 | 1e700 |

### 14.4 ID Production Chain

```
ID8 → ID7 → ... → ID1 → Infinity Power
```

Test:
```
state: ID1.amount=10, ID2.amount=5, no tickspeed effects on IDs
action: tick(1000ms)
assert: infinity_power += ID1_production
assert: ID1.amount += ID2_production
```

---

## Section 15: Replicanti

### 15.1 Basic Growth

```
growth_per_interval = replicanti_amount × 2^(chance)
effective_growth_per_second = replicanti × 2^(chance / interval_ms × 1000)
```

Simplified continuous model:
```
replicanti(t) = replicanti(0) × 2^(chance × t / interval)
```

Test:
```
state: replicanti=1, chance=0.01, interval=1000ms
action: tick(1000ms)
assert: replicanti ≈ 1 × 2^0.01 ≈ 1.00695
// Actually the growth model may be: amount × (1 + chance) per interval
// Need to verify exact formula. In the simple case:
// Per interval: amount doubles with probability `chance`
// Expected value per interval: amount × (1 + chance)
// Per second with interval=1000ms: one growth event
// replicanti = 1 × (1 + 0.01) = 1.01
```

### 15.2 Replicanti Galaxy

```
state: replicanti = 1e308 (at cap), replicanti_galaxies=0
action: buy_replicanti_galaxy()
assert: replicanti resets to 1
assert: replicanti_galaxies = 1
assert: effective galaxy count for tickspeed includes replicanti_galaxies
```

### 15.3 Replicanti Cap Behavior

```
state: replicanti approaching 1e308
action: tick that would push replicanti above 1e308
assert: replicanti capped at 1e308
```

### 15.4 Replicanti Upgrades

| Upgrade | Base Cost | Effect |
|---------|-----------|--------|
| Chance | 1e150 IP | Increase chance (toward 100%) |
| Interval | 1e140 IP | Decrease interval (toward 1ms) |
| Max Galaxies | 1e170 IP | +1 max replicanti galaxy |

---

## Section 16: Full Progression Scenarios

### 16.1 Fresh Game to First Dim Boost

```
scenario: Start → first dim boost (no manual optimization, just autobuyers)
initial: fresh game
steps:
  1. Buy AD1 until can't afford
  2. Buy AD2, AD3, AD4 as affordable
  3. Let production run
  4. Buy more as affordable
  5. Eventually reach 20 bought on AD4
  6. Dim boost
verify: All intermediate states match JS game
expected_time: ~2-5 minutes real time
```

### 16.2 First Galaxy

```
scenario: Reach first galaxy from fresh game
strategy:
  1. Dim boost 4 times (unlock all 8 dims)
  2. Buy dim 8s while buying tickspeed
  3. Reach 80 bought on dim 8
  4. Buy galaxy
verify: galaxy count = 1, all dims reset, tickspeed improved
```

### 16.3 First Infinity

```
scenario: Reach 1e308 antimatter from fresh game
strategy:
  1. Get several galaxies (tickspeed improvement is key)
  2. With enough galaxies, production accelerates exponentially
  3. Reach 1e308 antimatter
verify: can perform big crunch, IP = 1 (pre-break)
expected_play: ~20-30 minutes first time
```

### 16.4 Break Infinity and Beyond

```
scenario: After accumulating enough IP for break infinity
steps:
  1. Multiple infinities to accumulate IP
  2. Buy infinity upgrades
  3. Buy Break Infinity
  4. Now antimatter goes past 1e308
  5. IP gain scales with antimatter
verify: IP gain formula matches for various antimatter values post-break
```

### 16.5 Full Pre-Infinity Speedrun Comparison

```
scenario: Optimal play from fresh game to first infinity
strategy: (mimics speedrun route)
  1. Buy dims 1-4 optimally
  2. Dim boost at earliest opportunity (×4)
  3. Buy dims 5-8
  4. Get first galaxy at 80 bought on dim 8
  5. Repeat: boost → galaxy → faster tickspeed → faster growth
  6. At ~3-4 galaxies, sacrifice becomes worthwhile
  7. Continue galaxy grinding
  8. Reach 1e308
verify: Timeline and intermediate antimatter values match JS within tolerance
```

---

## Section 17: Edge Cases

### 17.1 Zero Amount Production

```
state: AD1.amount = 0
assert: production_per_second(AD1) = 0
assert: tick produces no antimatter from AD1
```

### 17.2 Very Large Numbers

```
state: antimatter = 1e300, dimensions with large amounts
assert: no overflow or NaN
assert: production calculations remain finite
```

### 17.3 Sacrifice With Zero Prior Sacrifice

```
state: sacrificed = 0, AD1.amount = 1e50
action: sacrifice()
assert: sacrificed = 1e50
assert: totalBoost = max(1, log10(1e50)/10)^2 = 5^2 = 25
```

### 17.4 Galaxy at Minimum Requirement

```
state: AD8.bought = 80 (exactly meets requirement for galaxy 1)
assert: can_buy_galaxy() = true
state: AD8.bought = 79
assert: can_buy_galaxy() = false
```

### 17.5 Dim Boost When Already At Max Unlockable

```
state: dim_boosts = 4 (all 8 dims unlocked)
// Next boost still works, just doesn't unlock new dims
action: buy_dim_boost() (if AD8.bought >= 20)
assert: dim_boosts = 5
assert: unlocked_dimensions still = 8
```

### 17.6 Tickspeed With Many Galaxies (Approaching Floor)

```
state: galaxies = 50
multiplier = 0.965^(50-4) × 0.8 = 0.965^46 × 0.8 ≈ 0.1541
// Still well above the 0.01 floor
assert: multiplier ≈ 0.1541
```

### 17.7 Production Order Independence

Verify that all production for a tick is computed BEFORE being applied (no order-of-
update bugs):

```
state: AD1.amount=100, AD2.amount=10, AM=0
       multiplier=1, tickspeed=1
action: tick(1000ms)
// If computed simultaneously:
//   AM += AD1_prod = 100 × 1 × 1 = 100
//   AD1 += AD2_prod = 10 × 1 × 1 = 10
// If computed sequentially (wrong):
//   AM += AD1_prod = 100 (then AD1 becomes 110)
//   Result same because AD1 amount used was pre-tick value
assert: antimatter = 100
assert: AD1.amount = 110
```

---

## Appendix A: Static Data for Tests

### Dimension Base Costs and Multipliers

```
Tier | Base Cost | Cost Multiplier
  1  |     10    |     1e3
  2  |    100    |     1e4
  3  |   1e4     |     1e5
  4  |   1e6     |     1e6
  5  |   1e9     |     1e8
  6  |  1e13     |    1e10
  7  |  1e18     |    1e12
  8  |  1e24     |    1e15
```

### NC6 Modified Costs

```
Tier | NC6 Base Cost | NC6 Cost Multiplier
  1  |      10       |       1e3
  2  |     100       |      5e3
  3  |     100       |      1e4
  4  |     500       |     1.2e4
  5  |    2500       |     1.8e4
  6  |    2e4        |     2.6e4
  7  |    2e5        |     3.2e4
  8  |    4e6        |     4.2e4
```

### Galaxy Constants

```
Base cost:          80 (99 in NC10)
Cost multiplier:    60 (90 in NC10)
Required tier:      8  (6 in NC10)
Distant start:      100 (+ time study effects)
Remote start:       800 (+ Reality Upgrade 21)
```

### Dim Boost Constants

```
Base requirement:   20
Scaling per boost:  15 (after boost 4, per 8th dim)
Required tiers:     4,5,6,7,8,8,8,...
DimBoost.power:     2 (base, increased by upgrades)
```

---

## Appendix B: Test Implementation Strategy

### Phase 1: Unit Formula Tests

Implement as Rust unit tests in `ad-fidelity/tests/`:
- Test individual formula functions against known values
- No simulation needed, just verify mathematical operations
- Can be written immediately

### Phase 2: Scenario Tests (JSON Fixtures)

Pre-compute expected values from JS game:
1. Set up a JS test harness that can run the game headlessly
2. Execute specific action sequences
3. Record state snapshots at each step
4. Save as JSON fixtures
5. Rust tests load fixtures and compare against Rust simulation

### Phase 3: Statistical Comparison

Run long simulations (hours of game time) in both JS and Rust:
- Compare antimatter, IP, dimension amounts at regular intervals
- Compute relative error in log-space
- Flag any divergence beyond tolerance

### Tolerance Tiers

| Test Type | Tolerance | Rationale |
|-----------|-----------|-----------|
| Formula unit test | exact (f64 epsilon) | Single computation |
| Single-tick test | 1e-12 relative | Minimal accumulated error |
| Short simulation (<1min) | 1e-8 relative | Small accumulation |
| Long simulation (>10min) | 1e-4 relative | Significant accumulation |
| Full progression (hours) | 1e-2 relative | Many interacting systems |

---

*Document generated: 2026-06-23*
