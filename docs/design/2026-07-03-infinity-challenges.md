# Infinity Challenges — Feature 2.7

8 harder challenges unlocked by reaching antimatter thresholds that all exceed
`1e308` — so they require Break Infinity (2.3, now in place). Each modifies the
pre-Infinity rules; completing one (reaching its goal, then crunching) grants a
permanent reward. Structurally like Normal Challenges (2.5) but with a per-challenge
*goal* (not always `1e308`) and richer effects.

Original: `secret-formula/challenges/infinity-challenges.js` (data),
`core/infinity-challenges.js` (state machine), and the `InfinityChallenge(N).isRunning`
/ `.reward.effectValue` sites in dimension/sacrifice/tickspeed/galaxy/dimboost.

---

## 1. State machine (`player.challenge.infinity`)

- `current` — active IC id (0 = none), `challenge.infinity.current`.
- `completedBits` — bitmask `1 << id`.
- Rust: an `InfinityChallengeState { current: u8, completed: u16 }` on `GameState`
  (mirrors `NormalChallengeState`). `bestTimes` deferred.

Transitions (mirror `InfinityChallengeState`):
- **start(id)**: `bigCrunchReset(true, true)`, then `normal.current = 0`,
  `infinity.current = id`, **and `player.break = true`** (starting an IC breaks
  Infinity — the thresholds are unreachable otherwise).
- **exit**: `infinity.current = 0`, `bigCrunchReset(true, false)`.
- **complete**: on a crunch while an IC runs and `thisInfinity.maxAM >= goal`, set
  the bit (generalise `handle_challenge_completion` to complete whichever of
  normal/infinity is running — `Player.antimatterChallenge`).
- **isUnlocked**: `thisEternity.maxAM >= unlockAM`. We add a
  `records.max_am_this_eternity` (persists across crunch; would reset on Eternity),
  advanced in `tick`. `unlockAM = [1e2000, 1e11000, 1e12000, 1e14000, 1e18000,
  1e22500, 1e23000, 1e28000]` (id 1..8).

The **Infinity Challenges** subtab unlocks once any IC is unlocked (or Break
Infinity, practically).

---

## 2. Goal / cap generalisation

Currently the tick caps antimatter at `1e308` and `can_big_crunch` tests `>= 1e308`.
The original uses a per-challenge goal:

```
infinity_goal = current antimatter challenge ? challenge.goal : 1e308
can_big_crunch = thisInfinity.maxAM >= infinity_goal
cap: if (!broke_infinity || in_any_antimatter_challenge) antimatter → infinity_goal
```

- Normal challenges keep goal `1e308` (their `goal` is `NUMBER_MAX_VALUE`).
- ICs use their own `goal` (`[1e650, 1e10500, 1e5000, 1e13000, 1e16500, 2e22222,
  1e10000, 1e27000]`, id 1..8) — much higher, so post-break antimatter is capped at
  the IC goal, not `1e308`.
- `in_any_antimatter_challenge()` = normal running **or** infinity running; replaces
  `any_challenge_running()` at the cap + `skip_resets_if_possible` sites.

IP on the completing crunch uses the post-break formula on `thisInfinity.maxAM`
(already in place).

---

## 3. The 8 modifiers

| IC | goal | restriction | reward (target) |
|----|------|-------------|-----------------|
| 1 | 1e650 | **all Normal Challenges except NC9 & NC12 at once** | ×1.3^(IC completed) to Infinity Dimensions — *deferred to 3.1* |
| 2 | 1e10500 | auto-Sacrifice every 400 ms once AD8 exists | Sacrifice autobuyer + **stronger sacrifice** (IC2-completed formula) |
| 3 | 1e5000 | Tickspeed always ×1; instead a static AD mult `(1.05+galaxies·0.005)^totalTickBought` | that same AD mult, as a completion reward |
| 4 | 1e13000 | only the last-bought AD produces normally; others `^0.25` | all AD multipliers `^1.05` |
| 5 | 1e16500 | buying AD1–4 raises cheaper AD costs, AD5–8 raises pricier (IC5 cost bumps) | Galaxies ×1.1 and −1 to boost/galaxy requirement |
| 6 | 2e22222 | rising `matter` divides AD mult once AD2 exists | Infinity Dimension mult from tickspeed — *deferred to 3.1* |
| 7 | 1e10000 | no Antimatter Galaxies; Dim-Boost base mult capped at ×10 | Dim-Boost base mult floored at ×4 |
| 8 | 1e27000 | AD production decays over time; buying an AD/tickspeed resets it | AD 2–7 mult from AD1×AD8 mults |

Implementation notes:
- **IC1 composition**: `challenge_running(N)` becomes `normal.current == N ||
  (N != 9 && N != 12 && infinity_challenge_running(1))` — so every NC modifier site
  lights up under IC1 for free.
- **IC2**: reuses the sacrifice path. `sacrifice_exponent` base `1/120` and
  `total_boost`/`next_sacrifice_boost` "prePower = sacrificed / (nd1/sacrificed)"
  branches gate on `challenge_completed_infinity(2)`. The auto-sacrifice every 400 ms
  is a tick step (`ic2_count`).
- **IC3**: `tickspeed_effect` → 1 while IC3 runs; the AD common mult gains
  `(1.05+galaxies·0.005)^totalTickBought` while running *or* completed (reward).
- **IC4**: AD production for tiers other than `post_c4_tier` (the last-bought tier,
  a new field) is `production^0.25`. Reward: `finalMultiplier^1.05` all tiers.
- **IC5**: `challenge_cost_bump` already exists (NC9); add the IC5 branch
  (`multiplyIC5Costs`: cheaper/pricier by tier). Reward folds into
  `galaxy_strength_effect` (×1.1) and `reset_boost_reduction` (+1).
- **IC6**: reuses `matter` (the NC11 field; the tick already grows it for
  "NC11 || IC6"). AD common mult `/= matter.clampMin(1)` while IC6 runs.
- **IC7**: `can_buy_galaxy` → false; `dim_boost_power` capped at ×10 (running) /
  floored at ×4 (completed reward).
- **IC8**: AD production `× 0.8446303389034288^(thisInfinity.time − lastBuyTime)`.
  Needs a `this_infinity.last_buy_time` (set on any AD/tickspeed buy). Reward: AD 2–7
  mult from `AD1.mult × AD8.mult ^ 0.02`.

Rewards targeting **Infinity Dimensions** (IC1, IC6) are stubbed until 3.1 and wired
there. The `postC4Tier` / `lastBuyTime` / `ic2Count` fields are new `GameState`
state (mirroring `player.postC4Tier` etc.).

---

## 4. Save / load

`player.challenge.infinity.{current, completedBits}` → `infinity_challenge.{current,
completed}`. New scalars: `player.postC4Tier` (u8), `player.records.thisInfinity
.lastBuyTime` (f64), `player.ic2Count` (f64), and `records.max_am_this_eternity`
(via `thisEternity.maxAM`). All present in the template.

---

## 5. UI

A **Challenges → Infinity Challenges** subtab (conditional on any IC unlocked): a
grid of 8 tiles (vendored `infinity-challenges` CSS) — id, description, goal, a
Start/Running/Completed button, and the unlock hint. `start_infinity_challenge(id)`
/ `exit_challenge` commands + snapshot `infinity_challenges[]`.

---

## 6. Incremental plan

1. ✅ **State machine + goal/cap generalisation + IC1 composition** (part 1).
2. ✅ **Per-IC modifiers + rewards** (part 2): IC3 (tickspeed + static mult), IC4
   (`^0.25`/`^1.05` powers + `post_c4_tier`), IC5 (cost bumps + galaxy/requirement
   reward), IC6 (matter divide), IC7 (no galaxy + dim-boost power), IC8 (decay +
   `last_buy_time_ms`), IC2 (auto-sacrifice + stronger-sacrifice formula).
3. ✅ **UI** (part 3): the Challenges → Infinity Challenges subtab (8 tiles,
   start/exit), snapshot `infinity_challenges[]` + `start_infinity_challenge`.

**Feature 2.7 is complete**, save the deferred rewards: IC1 and IC6 reward
Infinity Dimensions (wired in 3.1), and the IC8 reward (AD 2–7 from AD1×AD8) is a
niche late-game multiplier left for a follow-up. All are purchasable/persisted and
their restrictions fully active.

---

## 7. Open questions (best-guess defaults)

- `thisEternity.maxAM`: modelled as `records.max_am_this_eternity` (all-time until
  Eternity exists).
- IC1/IC6 rewards on Infinity Dimensions: deferred, wired in 3.1.
- Best-times / retry-challenge: deferred (consistent with 2.5).

*Document generated: 2026-07-03.*
