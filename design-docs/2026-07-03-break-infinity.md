# Break Infinity — Feature 2.3

The upgrade that lets antimatter exceed `1e308`, after which Infinity Points scale
with how far past the cap you go. Gated (per the §2.5/§2.6 ordering) behind the Big
Crunch autobuyer reaching its 100 ms interval floor — which needs NC12 completed.
That gate is already exposed as `break_infinity_unlockable()` (Feature 2.6).

Original source: `game.js::gainedInfinityPoints` (the IP formula),
`AntimatterDimensions.tick` (the `1e308` production cap),
`components/tabs/break-infinity/BreakInfinityButton.vue` (the unlock),
`core/break-infinity-upgrades.js` + `secret-formula/infinity/break-infinity-upgrades.js`
(the 12 upgrades), and `player-progress.js` (`hasBroken` / `infinityUnlocked`).

---

## 1. Two flags: `break` vs. infinity-unlocked

The port currently conflates two distinct things onto our single `infinity_unlocked`
field, which it writes to the save's `player.break`. They must be separated:

- **`broke_infinity`** (new) ↔ `player.break`: has bought Break Infinity. A
  permanent unlock (reset only on Eternity, later). Lifts the `1e308` cap and
  switches the IP formula.
- **`infinity_unlocked`** (keep): has reached Infinity at least once. Gates the
  Infinity/Challenges UI. In the original this is a `PlayerProgress` bit; for us it
  stays a runtime flag set on the first crunch, and on load it is *derived*
  (`broke_infinity || infinities > 0 || infinity_points > 0`) — so it needs no
  save key of its own.

**Save fix:** write `player["break"] = broke_infinity` (not `infinity_unlocked`),
and read `broke_infinity = dto.break`. This also fixes a latent bug: a crunched-
but-not-broken save currently writes `break = true`, which the real game would
misread as "already broke infinity."

`hasBroken()` in the original is `break || eternity || reality`; pre-Eternity that
is just `broke_infinity`.

---

## 2. The unlock + break action

- **Unlockable:** `Autobuyer.bigCrunch.hasMaxedInterval` — the button appears once
  the Big Crunch autobuyer's interval is at the 100 ms floor. Our
  `break_infinity_unlockable()` already returns this (simplify it to *just*
  `has_maxed_interval(BigCrunch)`; `is_unlocked`/NC12 is implied, since the interval
  can only be reduced after NC12).
- **Action:** `break_infinity()` — sets `broke_infinity = true`. One-way
  pre-Eternity. The original shows a confirmation modal first; we can add that in
  the UI (or defer, like the challenge-start modal).

The button reads "BREAK INFINITY" when unlockable, "INFINITY IS BROKEN" after.

---

## 3. Lifting the cap + the IP formula

### 3.1 Production cap
`AntimatterDimensions.tick` caps antimatter at the infinity goal when
`hasBigCrunchGoal = !player.break || Player.isInAntimatterChallenge`. So:

```
cap antimatter at BIG_CRUNCH_THRESHOLD  iff  !broke_infinity || any_challenge_running()
```

Post-break and outside a challenge, the cap is gone and antimatter grows without
bound. **Inside a normal challenge the cap stays even post-break** (the challenge
still targets `1e308`). Our `tick` already caps unconditionally; this just adds the
guard.

### 3.2 IP formula (`gained_infinity_points`)
Our `crunch.rs` already has the shape with `ip_gain_divisor()` (= 308) and
`total_ip_mult()` (= 1). Add the post-break branch (mirrors `gainedInfinityPoints`):

```
div = 308                                  (Achievement 103 / TimeStudy 111 later)
base = broke_infinity
     ? 10 ^ (log10(thisInfinity.maxAM) / div - 0.75)
     : 308 / div            (= 1)
IP = floor(base * total_ip_mult)
```

So reaching `1e616` gives ≈ `10^(2 - 0.75) ≈ 17.8` base IP, and it climbs from
there. `total_ip_mult` grows once the `ipMult`-style upgrades land (the rebuyable
`ipMult` is actually an *Infinity* Upgrade bottom-row item, deferred with §2.2's
bottom row; the break upgrades feed AD multipliers, not `totalIPMult`).

### 3.3 `can_big_crunch`
Unchanged (`antimatter >= BIG_CRUNCH_THRESHOLD`). The original's `canCrunch` reads
`thisInfinity.maxAM >= goal`; since antimatter is monotonic within an infinity,
`antimatter == maxAM` at the peak, so the two agree. Post-break you simply crunch
manually (or via the autobuyer) at higher antimatter for more IP.

---

## 4. The 12 Break Infinity Upgrades

Bought with IP. **The 9 one-time upgrades share the save's `player.infinityUpgrades`
string set** with the §2.2 Infinity Upgrades (their state class's `set` is
`player.infinityUpgrades`); the **3 rebuyables** live in `player.infinityRebuyables`
(a 3-int array). Model as a separate `BreakInfinityUpgrade` enum + `u32` bitmask
(distinct from `InfinityUpgrade`) plus a `[u32; 3]` rebuyable-count array; on load,
parse both upgrade sets from the same `infinityUpgrades` array (unknown ids already
ignored), and encode writes both sets' owned ids back into it.

| id (save id) | cost | effect | port status |
|---|---|---|---|
| totalAMMult (`totalMult`) | 1e4 | AD ×`(totalAM.exp + 1)^0.5` | ✅ wireable (`total_antimatter`) |
| currentAMMult (`currentMult`) | 5e4 | AD ×`(AM.exp + 1)^0.5` | ✅ wireable |
| galaxyBoost (`postGalaxy`) | 5e11 | galaxies ×1.5 stronger | ✅ wireable (galaxy strength) |
| infinitiedMult (`infinitiedMult`) | 1e5 | AD ×`1 + infinities.pLog10()·10` | ✅ wireable (`infinities`) |
| achievementMult (`achievementMult`) | 1e6 | AD ×`max((achCount−30)^3/40, 1)` | ✅ wireable (achievement count) |
| slowestChallengeMult (`challengeMult`) | 1e7 | AD ×`clampMin(50/worstChallengeMin, 1)`, cap 3e4 | ⏳ deferred (no challenge best-times yet) |
| infinitiedGen (`infinitiedGeneration`) | 2e7 | passive infinities from `bestInfinity.time` | ⏳ deferred (generation loop) |
| autobuyMaxDimboosts (`autobuyMaxDimboosts`) | 5e9 | unlock the buy-max Dim Boost autobuyer mode | ◑ purchase modelled; buy-max behaviour deferred |
| autobuyerSpeed (`autoBuyerUpgrade`) | 1e15 | challenge autobuyers 2× faster | ✅ wireable (the `interval()` halving I already stubbed) |
| tickspeedCostMult (rebuyable id 0) | 1e6 ×5, max 8 | reduce post-infinity tickspeed cost scaling | ⏳ deferred (`tickSpeedMultDecrease` not modelled) |
| dimCostMult (rebuyable id 1) | 1e7 ×5e3, max 7 | reduce post-infinity AD cost scaling | ⏳ deferred (`dimensionMultDecrease` not modelled) |
| ipGen (rebuyable id 2) | 1e7 ×10, max 10 | generate % of best IP/min | ⏳ deferred (IP/min tracking, like §2.2 `ipGen`) |

The wireable AD multipliers slot into `antimatterDimensionCommonMultiplier` /
`applyNDMultipliers` at the same sites the §2.2 Infinity-Upgrade multipliers already
use. Deferred effects return the neutral value (×1) so purchasing is harmless.

`achCount` = number of unlocked achievements (the original's `effectiveCount`
subtracts locked-secret handling we don't have; a straight count of set achievement
bits is the faithful pre-Reality value).

---

## 5. UI

- **Break Infinity button** — vendored `BreakInfinityButton.vue`: shows on the
  Antimatter Dimensions tab (or its own slot) once `break_infinity_unlockable()`;
  "BREAK INFINITY" → click → `break_infinity()`; "INFINITY IS BROKEN" after. A
  confirmation modal mirrors the prestige-confirm shell (optional first cut).
- **Break Infinity tab** — a new top-level tab (shown once `broke_infinity`) with
  the 12 upgrade buttons (vendored `break-infinity` styles), each showing cost /
  effect / owned state, calling `buy_break_infinity_upgrade(id)`.

---

## 6. Incremental plan

1. **Core (slice 1)**: `broke_infinity` field + save separation; simplify
   `break_infinity_unlockable`; `break_infinity()` action; guard the tick cap;
   post-break IP branch; the Break Infinity button + snapshot flag. Tests. Commit.
2. **Upgrades (slice 2)**: `BreakInfinityUpgrade` enum + bitmask + rebuyable counts;
   IP-gated purchase; save/load (shared `infinityUpgrades` + `infinityRebuyables`);
   wire the six wireable effects; stub the deferred ones; the Break Infinity tab.
   Tests. Commit.

---

## 7. Open questions (best-guess defaults)

- **`effectiveCount`** for `achievementMult`: use a straight unlocked-achievement
  count (the exact "effective" bookkeeping is Reality-era).
- **Deferred effects** (slowestChallenge / infinitiedGen / ipGen / the two cost-
  scaling rebuyables): purchase + persist now, effect = neutral until their inputs
  (challenge best-times, IP/min, the cost-scaling knobs) exist. They don't block
  the core loop.
- **Confirmation modal** for the break: deferred like the challenge-start one; the
  button can break directly for a first cut.

*Document generated: 2026-07-03.*
