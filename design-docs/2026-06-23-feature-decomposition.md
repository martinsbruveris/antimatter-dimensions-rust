# Antimatter Dimensions — Feature Decomposition

This document decomposes the original Antimatter Dimensions game (excluding the endgame
mod) into individual features that can be implemented in dependency order. Each feature
is described with its prerequisites, scope, and key formulas.

The game progresses through these major phases:

1. **Pre-Infinity** — Antimatter production with 8 dimension tiers
2. **Infinity** — First prestige layer, infinity points, upgrades
3. **Post-Break-Infinity** — Challenges, infinity dimensions, replicanti
4. **Eternity** — Second prestige layer, time dimensions, time studies
5. **Mid-Eternity** — Eternity challenges, time dilation
6. **Reality** — Third prestige layer, glyphs, perks, automator
7. **Celestials** — Seven endgame celestial encounters

---

## Phase 1: Pre-Infinity

These features cover the game from start until the player first reaches 1e308 antimatter.

### Feature 1.1: Antimatter Dimensions (Basic Production)

**Dependencies:** `break_infinity` crate (Decimal type)

**Scope:** Eight tiers of antimatter dimensions in a production chain. AD8 produces AD7,
AD7 produces AD6, ..., AD1 produces antimatter. Only the first 4 tiers are initially
available.

**Key data:**
- Base costs: `[10, 100, 1e4, 1e6, 1e9, 1e13, 1e18, 1e24]`
- Cost multipliers per purchase: `[1e3, 1e4, 1e5, 1e6, 1e8, 1e10, 1e12, 1e15]`
- Starting antimatter: 10
- Each purchase adds 1 to the dimension's amount and bought count
- Cost formula: `cost = baseCost * costMultiplier^bought`

**Production formula:**
```
production_per_second(tier) = amount * multiplier * tickspeed_effect
```

**Buy-10 multiplier:** Every 10 purchases of a dimension grants a `2x` multiplier
(stacking multiplicatively). The buy-10 multiplier is `2^floor(bought/10)`.

**Status:** ✅ Implemented (8 tiers, production chain, costs, single/bulk buy,
buy-10 multiplier)

---

### Feature 1.2: Tickspeed

**Dependencies:** Feature 1.1

**Scope:** A global upgrade that speeds up all dimension production. Purchased with
antimatter. Each purchase multiplies production by a factor.

**Key data:**
- Base cost: 1000 antimatter
- Cost multiplier: 10x per purchase
- Initial tickspeed interval: 1000ms
- Per-purchase multiplier: 0.8875 (retains this fraction of interval per purchase)

**Tickspeed effect on production:**
```
tickspeed_effect = 1000 / current_tickspeed_ms
current_tickspeed_ms = 1000 * multiplier^bought
```

The effective `multiplier` decreases with galaxies (see Feature 1.5).

**Status:** ✅ Implemented (purchases, cost formula, galaxy-based scaling,
effect on production)

---

### Feature 1.3: Buy-10 and Bulk Purchasing

**Dependencies:** Feature 1.1, Feature 1.2

**Scope:** The UI allows buying single dimensions or 10 at once. Buying 10 gives the
buy-10 multiplier bonus. Bulk-buy calculates the maximum affordable quantity using
geometric series cost summation.

**Key formulas:**
- Cost of next 10: geometric series `cost * (r^10 - 1) / (r - 1)` where `r` is the cost
  multiplier
- `ExponentialCostScaling` class: given current cost and budget, compute max purchasable
  using `floor(log(budget * (r-1) / cost + 1) / log(r))`

**Note:** The buy-10 multiplier in the original game is 2x by default, upgradeable later.

**Status:** ✅ Implemented (buy-10, buy-max via repeated single buys; no
closed-form bulk purchase optimization)

---

### Feature 1.4: Dimension Boosts

**Dependencies:** Feature 1.1

**Scope:** A soft reset that unlocks higher dimension tiers and provides a multiplier to
all dimensions.

**Requirements for each boost:**
- Boost 1: 20 bought of 4th dimension (unlocks 5th dim)
- Boost 2: 20 bought of 5th dimension (unlocks 6th dim)
- Boost 3: 20 bought of 6th dimension (unlocks 7th dim)
- Boost 4: 20 bought of 7th dimension (unlocks 8th dim)
- Boost 5+: 20 + 15*(boost-4) bought of 8th dimension

**Reset:** All dimension amounts, bought counts, costs, tickspeed purchases, and
antimatter reset to initial values. Galaxy count is preserved.

**Multiplier:** Each dimension boost provides a `2x` multiplier to dimensions of lower
tier:
```
boost_mult(tier) = 2^max(0, dim_boosts + 1 - tier)
```
(Tier is 1-indexed; boost only helps tiers ≤ dim_boosts)

**Status:** ✅ Implemented (requirements, soft reset, tier unlocking,
multiplier formula)

---

### Feature 1.5: Antimatter Galaxies

**Dependencies:** Feature 1.4, Feature 1.2

**Scope:** A harder reset that permanently improves the tickspeed purchase multiplier,
making all future tickspeed purchases more effective.

**Requirements:**
- Galaxy 1: 80 bought of 8th dimension
- Galaxy N: 80 + 60*(N-1) bought of 8th dimension
- (Distant scaling adds quadratic term past galaxy ~100; remote scaling adds exponential
  past galaxy 800 — these kick in much later)

**Reset:** Everything that a dimension boost resets, PLUS resets dimension boost count
back to 0.

**Tickspeed multiplier with galaxies:**
```
// Pre-3 galaxies:
multiplier = 0.8  (fixed, gets 0.82 after 1 galaxy, etc. — simplified)

// 3+ galaxies:
multiplier = 0.965^(galaxies - 2) * 0.8
// Each galaxy reduces tickspeed interval by ~3.5% more
```

(The exact formula has galaxy-count-dependent reduction of the per-purchase multiplier.
More galaxies → smaller multiplier → each tickspeed purchase reduces interval more.)

**Distant galaxy scaling** (kicks in around galaxy 100):
```
requirement += (galaxies - distantStart)^2 + (galaxies - distantStart)
```

**Remote galaxy scaling** (past galaxy 800):
```
requirement *= 1.002^(galaxies - 799)
```

**Status:** ✅ Implemented (requirements, reset, tickspeed improvement; no
distant/remote galaxy scaling yet)

---

### Feature 1.6: Dimensional Sacrifice

**Dependencies:** Feature 1.4 (requires 5th dimension unlocked = 1 boost)

**Scope:** Sacrifice all 1st dimension amount to boost 8th dimension production. The
multiplier grows with total antimatter sacrificed across all sacrifices.

**Pre-infinity formula:**
```
sacrifice_mult = max(log10(total_sacrificed) / 10, 1)^2.36
```

**Note:** The formula changes post-infinity (IC2 completion changes it to a ratio-based
formula). The exponent is modified by achievements.

**Reset:** All dimension amounts (1-7) are set to 0. Bought counts and costs are
preserved. 8th dimension amount is preserved.

**Status:** ✅ Implemented (sacrifice mechanic, multiplier formula, reset
behaviour)

---

### Feature 1.7: Buy-10 Multiplier Per Dimension

**Dependencies:** Feature 1.1

**Scope:** Every set of 10 purchases of a single dimension grants a permanent multiplier
to that dimension's production.

**Formula:**
```
buy10_mult(tier) = buy10_base ^ floor(bought[tier] / 10)
```

Where `buy10_base` is 2.0 by default (increased by Infinity Upgrade to a value based on
the number of galaxies + dim boosts).

**Status:** ✅ Implemented (integrated into dimension multiplier calculation)

---

## Phase 2: Infinity (First Prestige)

The player reaches infinity when antimatter hits `1e308` (≈ `Number.MAX_VALUE`).

### Feature 2.1: Big Crunch (Infinity Prestige)

**Dependencies:** Phase 1 complete

**Scope:** When antimatter reaches `1e308`, the player can "Big Crunch" to reset all
pre-infinity progress and gain Infinity Points (IP).

**IP formula (pre-break):**
```
IP = floor(308 / 307.8) = 1  (always exactly 1 IP before break infinity)
```

**What resets:** Antimatter, all dimensions, tickspeed, dimension boosts, galaxies,
sacrifice total.

**What persists:** Infinity Points (cumulative), infinity count, infinity upgrades (once
purchased), achievements.

**Records tracked:**
- Fastest infinity time
- Best IP/min for this eternity
- Total infinities performed
- Total antimatter produced (all-time)

**Status:** ✅ Implemented (Big Crunch awards IP — pre-break = 1 — and an
Infinity; `Records` tracks total time played, this/best-infinity time, and
this-infinity maxAM; IP/infinities/records persist across a crunch and
round-trip through the save; Infinity tab + IP header in the GUI). See
`design-docs/2026-07-02-infinity-points-and-records.md`. Best-IP/min and the
Statistics view are deferred until a consumer exists.

---

### Feature 2.2: Infinity Upgrades

**Dependencies:** Feature 2.1

**Scope:** 16 upgrades purchasable with IP. They form a grid where later upgrades require
earlier ones.

**Upgrades (1 IP each for the first column, increasing for later columns):**

| ID | Cost | Effect |
|----|------|--------|
| `timeMult` | 1 IP | Mult based on time in this infinity |
| `18mult` | 1 IP | 1st & 8th dim ×1.8 |
| `36mult` | 1 IP | 3rd & 6th dim ×2.6 |
| `resetBoost` | 1 IP | Start with 1 dim boost after reset |
| `buy10Mult` | 1 IP | Buy-10 multiplier: 2 → based on dim boosts |
| `galaxyBoost` | 2 IP | Galaxies are 1x more effective |
| `thisInfinityTimeMult` | 4 IP | Mult based on fastest infinity |
| `unspentIPMult` | 5 IP | Mult based on unspent IP |
| `dimboostMult` | 7 IP | Dim boost start higher (×2.5 per boost) |
| `ipGen` | 10 IP | Passively generate IP (10% of best IP/min) |
| `skipReset1` | 20 IP | Start with 1st dim boost on crunch |
| `skipReset2` | 40 IP | Start with 2nd dim boost on crunch |
| `skipReset3` | 80 IP | Start with 3rd dim boost on crunch |
| `skipResetGalaxy` | 300 IP | Start with 1 galaxy on crunch |
| `ipOffline` | 1000 IP | IP generation works offline |
| `ipMult` | 1e8 IP | ×2 IP from all sources |

**Grid structure:** 4 columns × 4 rows. Each column requires the previous column.
(Correction: each *column* is an independent top-to-bottom chain — an upgrade
requires the one directly above it in the same column, not "the previous column".)

**Status:** ✅ Implemented (all 16 grid upgrades: bitmask state, IP-gated purchase
with column prerequisites, every effect wired into the engine — dim multipliers,
buy-10 base, dim-boost power, boost/galaxy requirement reduction, galaxy strength,
skip-reset starting boosts, passive `ipGen`; save/load; the vendored 4×4 grid UI).
The **bottom row** (`ipMult` rebuyable + `ipOffline`, unlocked by Achievement 41)
is deferred to land with Break Infinity (§2.3), where `totalIPMult` also grows. See
`design-docs/2026-07-03-infinity-upgrades.md`.

---

### Feature 2.3: Break Infinity

**Dependencies:** Feature 2.2 — **and, in true gameplay order, Features 2.5 + 2.6.**

> **Ordering correction (found during the 2.2 port).** Despite its "2.3" number,
> Break Infinity is gated *later* than this document's linear order implies. The
> "Break Infinity" button unlocks only once the **Big Crunch Autobuyer** reaches a
> 0.1 s interval (`Autobuyer.bigCrunch.hasMaxedInterval`), and that autobuyer's
> interval can only be upgraded after **Normal Challenge 12 is completed**
> (`BigCrunchAutobuyerState.canBeUpgraded = NormalChallenge(12).isCompleted`). So
> the faithful implementation order is **2.1 → 2.2 → 2.5 (Normal Challenges) → 2.6
> (Autobuyers, incl. the Big Crunch autobuyer + IP interval upgrades) → 2.3 (Break
> Infinity) → 2.7 (Infinity Challenges)**. A future Break-Infinity design doc should
> either build that prerequisite chain first or, if an out-of-order slice is wanted,
> introduce a temporary unlock gate and flag the divergence from the original.

**Scope:** An upgrade that allows antimatter to exceed `1e308`. Before breaking, the game
forces a Big Crunch at exactly `1e308`. After breaking, antimatter can grow without limit
and IP scales with how far past `1e308` you go.

**IP formula (post-break):**
```
IP = 10^(log10(antimatter) / 308 - 0.75) * ipMultiplier
```

This means reaching `1e616` gives approximately `10^(2 - 0.75) = 10^1.25 ≈ 17.8` base IP,
and IP grows exponentially with higher antimatter.

**Break Infinity Upgrades** (12 upgrades, bought with IP):

| ID | Cost | Effect |
|----|------|--------|
| `totalAMMult` | 1e4 IP | Mult based on total AM produced |
| `currentAMMult` | 5e4 IP | Mult based on current AM |
| `galaxyBoost` | 5e11 IP | Galaxy requirement reduced |
| `infinitiedMult` | 1e5 IP | Mult based on infinity count |
| `achievementMult` | 1e6 IP | Mult based on achievement count |
| `slowestChallengeMult` | 1e7 IP | Mult based on slowest challenge time |
| `infinitiedGeneration` | 2e7 IP | Passively generate infinitied stat |
| `autobuyMaxDimboosts` | 5e9 IP | Autobuyer for max dim boosts |
| `autobuyerSpeed` | 1e15 IP | Autobuyers tick faster |
| `tickspeedCostMult` | ∞ | Tickspeed cost multiplier ×5 → ×4 → ×3 (rebuyable) |
| `dimCostMult` | ∞ | Dimension cost multipliers reduced (rebuyable) |
| `ipGen` | ∞ | Improve passive IP generation rate |

---

### Feature 2.4: Achievements

**Dependencies:** Feature 2.1 (basic system earlier, but rewards matter post-infinity)

**Scope:** ~180 achievements organized in rows of 8. Many provide permanent multipliers
or other bonuses. Achievements are retained across all resets.

**Structure:**
- 18 rows × 8 achievements per row (+ secret achievements)
- Rows 1-14 cover pre-Reality content
- Each achievement has unlock condition and optional reward
- Achievement rewards are multiplicative effects on dimensions/production

**Key achievement rewards affecting core mechanics:**
- Row 2: Dimension multiplier bonuses
- Row 3: Tickspeed/galaxy improvements
- Row 4-5: Various production multipliers
- Achievements 32, 57, 88: Improve sacrifice exponent

**Storage:** Bitfield (17 × 32-bit integers for ~180 booleans)

---

### Feature 2.5: Normal Challenges (12)

**Dependencies:** Feature 2.1

**Scope:** 12 challenges that modify pre-infinity rules. Complete a challenge by reaching
infinity under its restrictions. Each completion unlocks the next challenge and provides
a permanent reward.

**Challenges:**

| # | Restriction | Reward |
|---|-------------|--------|
| 1 | None (tutorial) | 1st AD autobuyer |
| 2 | Buying AD/tickspeed halts all AD production temporarily | 2nd AD autobuyer |
| 3 | 1st AD heavily weakened but gets uncapped exp. mult (resets on boosts/galaxies) | 3rd AD autobuyer |
| 4 | Buying an AD erases lower tiers | 4th AD autobuyer |
| 5 | Tickspeed purchase multiplier starts lower | 5th AD autobuyer |
| 6 | AD upgrades cost 2 tiers below instead of antimatter | 6th AD autobuyer |
| 7 | 10-buy multiplier reduced, scaling with boosts | 7th AD autobuyer |
| 8 | Dim boosts give no mult, galaxies disabled; sacrifice stronger | 8th AD autobuyer |
| 9 | Tickspeed/10-buy auto-scaling | Tickspeed autobuyer |
| 10 | Only 6 ADs; dim boost/galaxy costs modified | Dim boost autobuyer |
| 11 | Matter mechanic; if matter > antimatter, dim boost without bonus | Galaxy autobuyer |
| 12 | Each AD produces 2 tiers lower; 1st/2nd make AM; 2/4/6 stronger | Big Crunch autobuyer |

**Reset on enter/exit:** Same as infinity (soft reset).

---

### Feature 2.6: Autobuyers (Basic)

**Dependencies:** Feature 2.5 (unlocked via normal challenge completions)

**Scope:** Automatic purchasers for dimensions, tickspeed, dim boosts, and galaxies.
Initially have a tick interval (e.g., buys every 300ms). Intervals are upgradeable.

**Autobuyer types (pre-infinity rewards):**
- 8 dimension autobuyers (from NC2-NC9)
- Tickspeed autobuyer (from NC10)
- Dimension boost autobuyer (from NC11)
- Galaxy autobuyer (from NC12)
- Big Crunch autobuyer (from break infinity, at specific IP amount)

**Configuration per autobuyer:**
- Interval (decreases with upgrades, eventually instant)
- Mode: buy singles / buy 10 / buy max
- Dim boost autobuyer: configurable limit or "buy until X galaxies"
- Galaxy autobuyer: buy when requirement met
- Big crunch autobuyer: configurable threshold (AM or IP amount)

**Autobuyer upgrades:** Each dim autobuyer can be improved (reducing interval) by
spending antimatter on it, up to fastest tier (cosmetic in late game since everything
becomes instant via infinity upgrades).

---

### Feature 2.7: Infinity Challenges (8)

**Dependencies:** Feature 2.1, Feature 2.3 (break infinity)

**Scope:** 8 harder challenges unlocked by reaching specific antimatter thresholds.
Completion provides permanent rewards that significantly boost progression.

**Unlock thresholds:** `[1e2000, 1e2700, 1e3200, 1e4500, 1e5500, 1e6500, 1e9000,
1e12000]` (These require break infinity since they exceed 1e308.)

**Key challenge effects and rewards:**

| IC | Restriction | Reward |
|----|-------------|--------|
| 1 | Normal challenges all at once | Production ×1.3^(IC completions) |
| 2 | Sacrifice ratio-based | AM dim multiplier based on sacrifice |
| 3 | Tickspeed effect reduced (only applies to tier 1) | Dimension multiplier based on 1st dim |
| 4 | Only latest purchased dim produces | All dims produce (unblocks multi-dim) |
| 5 | Buy-10 cost × buy-10 multiplier; multiplier = cost/cost | Exponential mult by current AM |
| 6 | Exponential pricing | Mult by current AM |
| 7 | Tickspeed only works for 7th & 8th dims | 7th dim produces 8th dim |
| 8 | Dim mult = bought^(0.4+IC8 × 0.1); sacrifice disabled | Galaxy threshold cost reduced |

**Status:** ✅ Implemented (run state machine, all 8 restrictions + rewards, forced
crunch on start/exit, save/load, UI subtab). See
`design-docs/2026-07-03-infinity-challenges.md`.

---

## Phase 3: Infinity Dimensions & Replicanti

### Feature 3.1: Infinity Power & Infinity Dimensions

**Dependencies:** Feature 2.1 (Infinity Points for purchasing)

**Scope:** 8 tiers of infinity dimensions, purchased with IP. They produce "infinity
power" which gives a multiplier to all antimatter dimensions.

**Unlock requirements (antimatter thresholds):**
- ID1: 1e1100 AM (base cost: 1e8 IP)
- ID2: 1e1900 AM (base cost: 1e9 IP)
- ID3: 1e2400 AM (base cost: 1e10 IP)
- ID4: 1e10500 AM (base cost: 1e20 IP)
- ID5: 1e30000 AM (base cost: 1e140 IP)
- ID6: 1e45000 AM (base cost: 1e200 IP)
- ID7: 1e54000 AM (base cost: 1e250 IP)
- ID8: 1e60000 AM (base cost: 1e280 IP)

**Cost multipliers per purchase:** `[1e3, 1e6, 1e8, 1e10, 1e15, 1e20, 1e25, 1e30]`

**Production chain:** ID8 → ID7 → ... → ID1 → Infinity Power

**Infinity Power effect:**
```
infinity_power_mult = infinity_power ^ 7
// (exponent = 7, increased by IC4 reward and Infinity Upgrades)
```

This multiplier applies to ALL antimatter dimensions.

**Purchasing:**
- IDs are bought in batches of 10 (like ADs)
- Cost uses `LinearCostScaling` (initial cost + increment per purchase)
- Base costs and multipliers vary by tier

**Persistence:** IDs are NOT reset on infinity. They ARE reset on eternity (initially;
eternity milestones eventually preserve them).

**Status:** ✅ Implemented (8 tiers, production chain → Infinity Power → `^7` AD
multiplier, unlock/buy/buy-max, per-crunch amount reset with purchases kept,
save/load, UI subtab). See `design-docs/2026-07-03-infinity-dimensions.md`.

---

### Feature 3.2: Replicanti

**Dependencies:** Feature 2.1 (unlocked with IP), Feature 2.3 (break infinity)

**Scope:** Self-replicating entities that provide a multiplier to all infinity dimensions
and can be converted into "Replicanti Galaxies" (free galaxy equivalents).

**Unlock:** Costs 1e140 IP.

**Growth model:**
- Replicanti have a "chance" (starts at 1%) and an "interval" (starts at 1000ms)
- Each tick a replicanti reproduces with probability `chance` per interval
- Effective growth: `amount *= (1 + chance)^(diff / interval)` (the JS "fast gain"
  continuous approximation; the binomial/Poisson randomness at tiny amounts is dropped)
- Cap: 1e308 (then galaxies can be purchased)
- Post-cap growth: slows down (scale factor per log10) — **unreachable pre-Eternity**
  (`isUncapped` is always false, so amount stays clamped at the cap; omitted)

**Replicanti Galaxies:**
- When replicanti reach cap, can buy a Replicanti Galaxy (resets replicanti to 1)
- Each Replicanti Galaxy acts exactly like an Antimatter Galaxy for tickspeed calculation
- Max galaxies starts at 1 (increased by upgrades and time studies)

**Replicanti Upgrades (purchased with IP):**
- Chance upgrade: increases chance (cap 100%)
- Interval upgrade: decreases interval
- Max galaxy upgrade: increases max replicanti galaxies

**Replicanti multiplier to Infinity Dimensions:**
```
replicanti_mult = max(log2(max(replicanti_amount, 1))^2, 1)
// (applied while unlocked and amount > 1; the TS/Glyph terms are later features)
```

**Status:** ✅ Implemented (unlock at 1e140 IP, capped continuous growth,
Replicanti Galaxies feeding tickspeed via `effective_galaxies`, the 3 IP upgrades,
`replicanti_mult` into Infinity Dimensions, persistence across Big Crunch, save/load,
UI subtab). See `design-docs/2026-07-03-replicanti.md`.

---

## Phase 4: Eternity (Second Prestige)

### Feature 4.1: Eternity Prestige

**Dependencies:** Phase 2 & 3 complete (need IP ≥ 1e308)

**Scope:** When Infinity Points reach `1e308`, the player can "Eternity" to reset all
infinity-layer progress and gain Eternity Points (EP).

**EP formula:**
```
EP = 5^(log10(IP) / 308 - 0.7) * epMultiplier
```

**What resets:** Antimatter, all ADs, tickspeed, boosts, galaxies, IP, infinity upgrades
(initially — milestones keep them), infinity dimensions, replicanti, sacrifice.

**What persists:** Eternity Points, eternity count, time dimensions, time studies,
eternity milestones, eternity challenges.

**Records tracked:**
- Best EP in a single eternity
- Fastest eternity time
- Best EP/min

**Status:** ✅ Implemented (EP formula incl. pending crunch IP, milestone-aware
Eternity reset, ThisEternity/BestEternity records + bestIP/EPmin rates, save
round-trip, header EP readout + Eternity button + confirmation modal + E
hotkey, post-break header Big Crunch button). See
`design-docs/2026-07-04-eternity.md` §1.

---

### Feature 4.2: Eternity Milestones

**Dependencies:** Feature 4.1

**Scope:** Permanent unlocks earned by total eternities performed. They reduce the grind
by auto-keeping resources across resets.

**Key milestones (by eternity count required):**

| Eternities | Effect |
|-----------|--------|
| 1 | Unlock time dimension 1; auto IP mult |
| 2 | Unlock time dimension 2; keep autobuyers |
| 3 | Unlock time dimension 3; replicanti galaxy autobuyer |
| 4 | Unlock time dimension 4; keep infinity upgrades |
| 5 | Keep break infinity upgrades |
| 6 | Auto dim boost autobuyer modes |
| 7 | Keep more autobuyer modes |
| 8 | Auto EP/auto infinities offline |
| 9 | Auto-complete infinity challenges |
| 10 | Unlock replicanti |
| 11-18 | ID autobuyers (one per milestone) |
| 25 | Auto-unlock infinity dimensions |
| 30 | Unlock all normal dimensions |
| 40 | Replicanti no-reset |
| 50 | Start with certain resources |
| 100 | Keep various infinity progress |
| 200 | Start with higher IP |
| 1000 | Keep galaxies on eternity |

**Status:** ✅ Implemented (derived milestone state, reset-time keeps
2/4/8/10, per-tick autoIC + autoUnlockID, unlockAllND, replicantiNoReset,
milestone grid UI). The milestones unlocking not-yet-built autobuyer types and
the offline generators display as reached but gain their effects with the
automation/offline systems. See `design-docs/2026-07-04-eternity.md` §2.
(Note: the table above is approximate; the exact catalogue is in the design
doc / `eternity_milestones.rs`.)

---

### Feature 4.3: Time Dimensions

**Dependencies:** Feature 4.1, Feature 4.2

**Scope:** 8 tiers of time dimensions purchased with EP. They produce "time shards" which
provide free tickspeed upgrades.

**Unlock:**
- TD1-4: Unlocked by eternity milestones (eternities 1-4)
- TD5-8: Unlocked by Time Studies (studies 71, 72, 73, 74)

**Production chain:** TD8 → TD7 → ... → TD1 → Time Shards

**Time Shards → Free Tickspeed:**
```
free_tickspeed = floor(log(time_shards + 1) / log(threshold))
```
where `threshold` starts at `3.33e3` and decreases with more free tickspeed upgrades (the
cost per free upgrade increases).

**Cost scaling:**
- Uses `ExponentialCostScaling`
- Super-exponential breakpoint at `e6000` EP

**Persistence:** Time dimensions persist across eternities (never reset except on
Reality).

**Status:** ✅ Implemented (TD1–4 purchase/production chain, Time Shards → free
Tickspeed upgrades incl. the 300 000 softcap + Newton inversion, the full
threshold/e6000 cost curve, Decimal tickspeed refactor, save round-trip, tab
UI). **Correction:** TD5–8 are unlocked by *Dilation* studies 2–5 (Phase 5),
not time studies 71–74 as stated above; they are modelled but stay locked. See
`design-docs/2026-07-04-eternity.md` §3.

---

### Feature 4.4: Time Studies (Tree)

**Dependencies:** Feature 4.1

**Scope:** A tree-shaped system of ~220 upgrades purchased with Time Theorems (TT). Time
Theorems are purchasable with AM, IP, or EP (each with escalating costs).

**Time Theorem costs:**
- AM: starts at 1e20000, multiplied by 1e20000 each purchase
- IP: starts at 1, multiplied by 10 each purchase
- EP: starts at 1, multiplied by 2 each purchase

**Tree structure (simplified):**
```
[11] ─┬─ [21] ─ [22] ─ ...  (Antimatter path)
      ├─ [31] ─ [32] ─ ...  (Infinity path)
      └─ [41] ─ [42] ─ ...  (Time path)

After initial splits, paths converge and diverge through:
- Active / Passive / Idle studies (choose one per section)
- Light / Dark studies (choose one)
- EC studies (gate eternity challenge entry)
- Dilation studies
```

**Notable studies:**
- TS 11: ×3 to AD production (first study, always bought)
- TS 32-33: Choices between faster growth vs stronger passive
- TS 71-74: Unlock TD5-TD8
- TS 101-103: Active/Passive/Idle split
- TS 111: Improves IP formula divisor
- TS 191-193: Second Active/Passive/Idle split
- TS 181-214: EC unlock studies
- TS 225-228: Dilation prerequisite studies

**Respec:** Can refund all Time Studies, getting back Time Theorems. Some resets auto-
respec (on eternity by default, configurable).

**Status:** ✅ Implemented (TT purchases with AM/IP/EP — note the IP cost step
is ×1e100, not the ×10 above; the 58-study pre-dilation catalogue with the
Dimension split, mutually exclusive Pace columns and Light/Dark pairs, respec
flag + refund; ~40 in-frontier study effects wired at their engine sites incl.
banked Infinities (TS191), uncapped Replicanti (TS192), the crunch-time RG keeps
(TS33) — which also fixed our port to reset Replicanti on a Big Crunch like the
original — distant/remote galaxy scaling (needed by TS223/224), and the last-10-
eternities record ring (TS121); save round-trip; the vendored tree UI with SVG
connections + vendored time-studies.css). EC study slots render as placeholders
until Feature 4.5. See `design-docs/2026-07-04-eternity.md` §4.

---

### Feature 4.5: Eternity Challenges (12)

**Dependencies:** Feature 4.4 (gated by time studies)

**Scope:** 12 challenges with up to 5 completions each. Each completion has a scaling
goal. Rewards scale with completion count.

**Entry:** Must buy the corresponding EC time study (TS 181-192 + unlock requirements).

**Structure per challenge:**
```
{
  id: N,
  cost: TT_cost_to_unlock,
  requirement: { secondary condition, e.g., "no time studies" },
  goal: base_goal,
  goalIncrease: goal_multiplier_per_completion,
  reward: { formula that scales with completions }
}
```

**Example challenges:**

| EC | Goal (×goalIncrease^completions) | Restriction | Reward |
|----|------|-------------|--------|
| 1 | 1e1800 × 1e200 | Time study count limited | Time dim mult based on time studies |
| 2 | 1e975 × 5e5 | ID multiplier = AM^0.0003 | ID multiplier based on current AM |
| 3 | 1e600 × 5e5 | AD multiplier = amount^0.25 | AD mult based on dim 1 amount |
| 4 | 1e2750 × 1e400 | All ID mult = 1 during infinity | ID production ×1e4 per completion |
| 5 | 1e750 × 1e250 | Galaxy cost ×5; limit on dim boosts | Galaxy cost −1 per completion |
| 6 | 1e850 × 5e5 | Replicanti speeds up AM; no galaxy | Replicanti speeds up AM dims |
| 7 | 1e2000 (fixed) | Tickspeed affects only 8th dim | 1st dim produces 8th dim too |
| 8 | 1e1300 × 5e5 | Replicanti galaxy gives +0.1% IP | Replicanti galaxy gives IP |
| 9 | 1e1750 × 5e10 | AM dim buy-10 mult reduced; tickspeed only helps after infinity | ID buy-10 mult based on tick bought |
| 10 | 1e3000 × 5e5 | TDs disabled; ID multiplier = AM^0.0002; galaxy threshold ×10 | Time dim mult × IP per completion |
| 11 | 1e500 × 1e200 | AD multiplier formula (11111/amount) | AD mult from time spent |
| 12 | 1e110000 × 1e12000 | AD = AD^0.1; ID disabled; TD = TD^0.5 | TD mult based on AM produced |

---

### Feature 4.6: Eternity Upgrades

**Dependencies:** Feature 4.1

**Scope:** A set of upgrades purchasable with EP providing permanent bonuses.

**Upgrades include:**
- EP multiplier (rebuyable, ×5 per purchase, doubling cost each time)
- ID cross-multiplier (each ID gives mult to next)
- Time dimension multiplier ×2
- Time dimension multiplier by eternities performed
- Reduce galaxy requirement by 5 per purchase

---

## Phase 5: Time Dilation

### Feature 5.1: Time Dilation

**Dependencies:** Feature 4.4 (requires buying dilation studies TS 225-228 + total TT
threshold)

**Scope:** A special Eternity run where all multipliers are "dilated" — exponentially
compressed. Rewards Tachyon Particles, which passively produce Dilated Time.

**Dilation formula:**
```
dilated_value(x) = 10^(sign(log10(x)) * |log10(x)|^0.75)
```
This dramatically compresses large multipliers (e.g., 1e1000 → 1e177).

**Tachyon Particles (TP):**
```
TP_gain = (log10(antimatter) / 400)^1.5 * tachyon_mult
```
(Gained at the end of a dilated eternity run)

**Dilated Time (DT):**
- Produced passively from TP: `DT_per_second = TP^0.5 * some_multiplier`
- Used to purchase Dilation Upgrades and Tachyon Galaxies

**Tachyon Galaxies:**
- Bought with DT (escalating cost)
- Act like free galaxies for tickspeed calculation (same as replicanti galaxies)

---

### Feature 5.2: Dilation Upgrades

**Dependencies:** Feature 5.1

**Scope:** ~15 upgrades purchased with Dilated Time.

**Key upgrades:**
- DT gain multiplier (rebuyable)
- Galaxy threshold reduction
- Tachyon Galaxy threshold reduction (rebuyable)
- AD mult based on DT
- IP multiplier
- Time dimension mult from DT
- Tachyon Particle multiplier (rebuyable)
- Dilation penalty reduced (0.75 → higher)

---

## Phase 6: Reality (Third Prestige)

### Feature 6.1: Reality Prestige

**Dependencies:** Feature 5.1 (requires dilation unlock + EP ≥ 1e4000)

**Scope:** The third and final prestige layer. Resets everything below and grants Reality
Machines (RM), a Glyph, and Perk Points.

**RM formula:**
```
RM = floor(1000^(log10(EP) / 4000 - 1)) * rm_mult
```

**What resets:** Everything from eternity (EP, time studies, time dimensions, dilation,
eternity challenges, infinity dimensions, replicanti, all pre-eternity).

**What persists:** Reality Machines, Glyphs, Perks, Reality Upgrades, Black Holes,
Celestial progress, Achievement count.

---

### Feature 6.2: Glyphs

**Dependencies:** Feature 6.1

**Scope:** Equippable items gained on each Reality. Each glyph has a type, level, rarity,
and 1-4 random effects. Up to 5 glyphs can be equipped simultaneously.

**Glyph types:** Power, Infinity, Replication, Time, Dilation (5 base types; more from
celestials)

**Glyph effects (examples):**
- Power: AD multiplier, AD exponent, dim boost multiplier
- Infinity: IP multiplier, ID multiplier, infinity count mult
- Replication: Replicanti speed, Replicanti galaxies, DT multiplier
- Time: EP multiplier, game speed, eternity count mult
- Dilation: DT production, galaxies from dilation, AD exponent in dilation

**Glyph level:** Based on Reality stats (EP gained, replicanti, time spent, etc.) **Glyph
rarity (strength):** Random 1-4 (higher = better effect values)

**Combining effects:** Multiple equipped glyphs of the same type have effects combined
using type-specific rules (additive, multiplicative, or max).

**Glyph sacrifice:** Sacrifice unneeded glyphs for Alchemy resources (later feature).

---

### Feature 6.3: Perks

**Dependencies:** Feature 6.1

**Scope:** A tree of ~100 permanent unlocks purchased with Perk Points (1 PP per
Reality). Perks provide QoL and progression bonuses.

**Notable perks:**
- START group: Start with various resources after Reality
- AUTOMATION: Various autobuyers become instant/permanent
- ACHIEVEMENT: Keep achievements on Reality
- INFINITY: Start with IP/break/infinity upgrades
- ETERNITY: Start with EP/time studies
- DILATION: Auto-unlock dilation features
- REALITY: Improve RM gain, glyph selection, etc.

**Structure:** Tree with branching paths. Each perk costs 1 PP and requires its parent
perk to be purchased.

---

### Feature 6.4: Reality Upgrades

**Dependencies:** Feature 6.1

**Scope:** 25 permanent upgrades purchased with RM. Provide multipliers and QoL
improvements.

**Key upgrades:**
- Dimension multiplier based on total RM
- Glyph level multiplier
- Time theorem generation (passive)
- Replicanti speed multiplier
- EP multiplier
- Galaxy requirement reduction
- "Imaginary" upgrade prerequisites for endgame

**Imaginary Upgrades:** 25 additional endgame upgrades purchased with Imaginary Machines
(iM). Unlocked progressively from RM-based gates.

---

### Feature 6.5: Black Holes

**Dependencies:** Feature 6.1 (purchased with RM)

**Scope:** Two black holes that periodically speed up game time. They pulse on/off with
configurable intervals.

**Black Hole 1:** Unlocked by RM cost. **Black Hole 2:** Unlocked by Reality Upgrade.

**Mechanics:**
- Active duration: configurable (starts short, upgrades lengthen)
- Game speed multiplier when active: starts at ~3x, upgradeable to massive values
- Interval between activations: configurable

**Black hole upgrades (purchased with RM):**
- Power (speed multiplier)
- Duration (active time)
- Interval (time between activations, shorter = better)

---

### Feature 6.6: Automator

**Dependencies:** Feature 6.3 (perks unlock automator)

**Scope:** A scripting language that automates game actions (buying studies, prestiging,
waiting for conditions, etc.). Uses a visual block editor or text editor.

**Commands:**
- `start/stop [dilation|ec N|challenge N]`
- `studies [load/respec/buy]`
- `auto [infinity/eternity/reality] [on/off/setting]`
- `if/while [condition] { ... }`
- `wait [condition]`
- `unlock [dilation/ec/etc]`
- `buy [upgrade]`

**Parser:** Uses the Chevrotain parser library in JS. For Rust, a simple hand-written
parser or `nom`/`pest` would work.

---

## Phase 7: Celestials

Celestials are seven endgame encounters unlocked after the first Reality. Each provides a
special "Reality" (a run under modified rules) and progressive unlocks.

### Feature 7.1: Teresa

**Dependencies:** Feature 6.1

**Scope:** The first and simplest celestial. Unlocked immediately after first Reality.

**Mechanics:**
- Teresa's Reality: a special run where all multipliers are raised to a power < 1
- IP storage: store IP to increase Teresa's RM reward multiplier
- Perk shop (uses RM)

**Rewards:** Increased RM gain, Glyph level bonus, EP multiplier

---

### Feature 7.2: Effarig

**Dependencies:** Feature 7.1

**Scope:** A multi-stage Reality with a glyph forge.

**Mechanics:**
- Effarig's Reality: 3 stages (Infinity → Eternity → Reality completion)
- Relic Shards: currency gained from running Effarig's Reality
- Glyph Forge: uses Relic Shards to improve glyph quality

**Rewards:** Better glyphs, new glyph type ("Effarig"), glyph cap increases

---

### Feature 7.3: The Nameless Ones (Enslaved)

**Dependencies:** Feature 7.2

**Scope:** Time storage and release mechanics.

**Mechanics:**
- Store real time: game runs slower, builds stored time
- Store game time: absorbs game speed factor
- Release: dumps stored time as a massive speed burst
- Enslaved's Reality: a run with unique restrictions (can't store time, limited
  mechanics)

**Rewards:** Improved time storage capacity, auto-release features, glyph enhancements

---

### Feature 7.4: V

**Dependencies:** Feature 7.3

**Scope:** Achievement-like hard goals ("V-achievements"). V is about reaching specific
targets in constrained runs.

**Mechanics:**
- V-achievements: 36 goals (6 sets × 6 tiers)
- Goals include: "Reach X EP with ≤Y time studies", "Complete EC N in <Z time"
- Hard V-achievements: even harder versions

**Rewards:** V-achievement count unlocks permanent bonuses (dimension multipliers, glyph
improvements, new features)

---

### Feature 7.5: Ra

**Dependencies:** Feature 7.4

**Scope:** Pet system with memory/chunk mechanics and the Alchemy lab.

**Mechanics:**
- 4 Pets (one per previous celestial): Teresa, Effarig, Enslaved, V
- Each pet gains "memories" from real time while in Ra's Reality
- Memory chunks → level ups → unlock abilities
- Alchemy: 21 resources in a tree, produced from glyph sacrifice

**Alchemy resources:** dimensionality, infinity, time, replication, dilation, effarig,
power, momentum, decoherence, etc. (each provides a multiplier or effect)

**Rewards:** Celestial memories provide massive multipliers, alchemy enables endgame
power

---

### Feature 7.6: Lai'tela

**Dependencies:** Feature 7.5

**Scope:** Dark Matter Dimensions, continuum, and singularities.

**Mechanics:**
- Dark Matter Dimensions (4 tiers): produce Dark Matter
- Dark Energy: generated from Dark Matter Dimensions
- Singularities: condensed from Dark Energy
- Continuum: dimensions are purchased "continuously" (fractional purchases, replaces
  buy-10 system)
- Entropy: a mechanic that destabilizes reality (triggers a Reality if too high)
- Lai'tela's Reality: AD exponents heavily nerfed

**Rewards:** Singularity milestones, massive production multipliers, continuum unlock

---

### Feature 7.7: Pelle (Final Boss)

**Dependencies:** Feature 7.6

**Scope:** "The Doomed" — the final celestial that disables most previous mechanics and
adds new resource systems.

**Mechanics:**
- Dooming: entering Pelle's reality permanently (within a run) disables most celestial
  bonuses, achievements, and upgrades
- Remnants: primary Pelle currency
- Reality Shards: secondary currency from Remnants
- Rifts: 5 rifts that fill over time, providing bonuses when drained
- Pelle Upgrades: purchased with Reality Shards
- Galaxy Generator: late Pelle mechanic, passively generates galaxies

**Progression:**
- Fill rifts (Vacuum, Decay, Chaos, Recursion, Paradox)
- Buy Pelle upgrades
- Reach the endgame antimatter goal

**Game ending:** Reaching a specific antimatter threshold in Pelle completes the game.

---

## Implementation Order Summary

The features above are designed to be implemented roughly in the listed order. Here's a
condensed dependency graph:

```
Phase 1 (Pre-Infinity):
  1.1 Dimensions → 1.2 Tickspeed → 1.3 Bulk Buy → 1.4 Dim Boosts → 1.5 Galaxies
       └→ 1.6 Sacrifice
       └→ 1.7 Buy-10 Multiplier

Phase 2 (Infinity):
  2.1 Big Crunch → 2.2 Infinity Upgrades → 2.3 Break Infinity
       └→ 2.4 Achievements (ongoing)
       └→ 2.5 Normal Challenges → 2.6 Autobuyers
       └→ 2.7 Infinity Challenges

Phase 3 (Infinity Dimensions + Replicanti):
  3.1 Infinity Dimensions (requires 2.1)
  3.2 Replicanti (requires 2.3)

Phase 4 (Eternity):
  4.1 Eternity Prestige (requires 3.x)
  4.2 Eternity Milestones → 4.3 Time Dimensions
  4.4 Time Studies
  4.5 Eternity Challenges (requires 4.4)
  4.6 Eternity Upgrades

Phase 5 (Dilation):
  5.1 Time Dilation (requires 4.4)
  5.2 Dilation Upgrades

Phase 6 (Reality):
  6.1 Reality Prestige (requires 5.1)
  6.2 Glyphs → 6.3 Perks → 6.4 Reality Upgrades
  6.5 Black Holes
  6.6 Automator (requires 6.3)

Phase 7 (Celestials):
  7.1 Teresa → 7.2 Effarig → 7.3 Enslaved → 7.4 V → 7.5 Ra → 7.6 Lai'tela → 7.7 Pelle
```

---

## Cross-Cutting Concerns

These systems span multiple features and should be designed for extensibility from the
start:

### Multiplier Pipeline

Almost every feature adds multipliers to dimensions, IP, EP, etc. The multiplier
computation system needs to support conditional effect sources that can be added
incrementally. Design the `EffectSource` enum and multiplier builder to be extendable.

### Challenge Modifiers

Challenges (normal, infinity, eternity, celestial realities) all modify game rules. The
`ActiveModifiers` struct should be designed to accommodate modifiers from any source.

### Prestige Reset Chain

Each prestige layer triggers resets of lower layers with complex "keep" conditions based
on milestones, perks, and upgrades. Design the reset system to be configurable per-layer.

### Records and Statistics

The game tracks extensive records (best times, best rates, recent runs). These need to be
updated at prestige events and preserved across appropriate reset boundaries.

### Cost Scaling

Multiple cost models are used throughout:
- `ExponentialCostScaling`: base × mult^n with optional super-exponential breakpoint
- `LinearCostScaling`: base + increment × n
- `FreeTickspeed`: logarithmic conversion of shards to free upgrades

These should be generic utilities reusable across features.

### Save/Load

Game state serialization. Can be implemented incrementally as features are added, but the
`GameState` struct needs to be designed for easy serde support from the start.

---

## Estimated Scope

| Phase | Features | Approximate Lines (Rust est.) | JS Reference Lines |
|-------|----------|------|------|
| 1 | 7 | 1,500-2,000 | ~2,000 |
| 2 | 7 | 3,000-4,000 | ~5,000 |
| 3 | 2 | 1,500-2,000 | ~2,000 |
| 4 | 6 | 4,000-5,000 | ~8,000 |
| 5 | 2 | 1,500-2,000 | ~2,000 |
| 6 | 6 | 5,000-7,000 | ~12,000 |
| 7 | 7 | 8,000-12,000 | ~20,000 |
| **Total** | **37** | **~25,000-34,000** | **~51,000** |

The Rust implementation should be more concise than the JS due to stronger typing, less
boilerplate around effect composition, and no UI code mixed in.

---

*Document generated: 2026-06-23*
