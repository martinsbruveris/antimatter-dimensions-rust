---
date: 2026-07-09
feature: 2.4
design_docs:
  - ../design/2026-06-30-achievements.md
---

# Normal achievements — wiring up the unwired rows (batches of 20)

## Summary

Continuing the achievements feature (`docs/design/2026-06-30-achievements.md`,
which shipped rows 1–2 minus the News achievement), this session wires the
unlock conditions **and effects** for the remaining normal achievements, working
through the id list in batches of 20 and committing after each. The substrate
(bitmask state, `achievement_power`, per-tier effect seam in
`dimension_multiplier`, save round-trip) was already in place; this is condition
+ effect wiring at the action seams.

## Architecture

The original registers each achievement on an event bus (`checkEvent`); we have
no bus, so — matching the existing design — the checks are called inline at the
equivalent action seam. Grouped into `check_*_achievements` methods on
`GameState` in `achievements.rs`, one per event:

- `check_tick_achievements(dt_ms)` — GAME_TICK_AFTER (called once per tick,
  replacing the old inline achievement-24 check). `dt_ms` drives the marathon
  timers.
- `check_crunch_before_achievements` / `check_crunch_after_achievements` — the
  BIG_CRUNCH_BEFORE / _AFTER seams in `big_crunch_reset` (at the goal).
- `check_galaxy_before_achievements` / `check_galaxy_after_achievements`.
- `check_sacrifice_after_achievements` — after a performed sacrifice.
- one-offs inline at their seam (41 at an Infinity-Upgrade purchase, 51 at
  Break Infinity).

`IMPLEMENTED_ACHIEVEMENTS` (the set the Reality-study requirement is checked
against) grows as coverage lands.

The two `AchievementTimers.marathon*` (transient module-level timers in the
original) become `#[serde(skip)]` `ach_marathon{1,2}_ms` fields on `GameState`
— reset to 0 on load, exactly like the original.

---

## Batch 1 — ids 31–54

### What shipped

Conditions (at the seams above) and effects for: 31, 32, 33, 34, 36, 37, 38,
41, 42, 43, 44, 45, 46, 47, 48, 51, 52, 53, 54.

Effects wired into their consumption sites:

- **AD multiplier** (`dimension_multiplier`): 31 (AD1 ×1.05), 34 (AD1–7 ×1.02),
  43 (each dim ×`1 + tier/100`), 48 (all ×1.1, via the new
  `achievement_ad_common_mult` helper that ports the achievement terms of the
  original's `antimatterDimensionCommonMultiplier`).
- **Starting antimatter** (`starting_antimatter`): 37/54 (+ 55/78 stubs) fold
  into the `Effects.max(10, …)` chain.
- **Sacrifice exponent** (`sacrifice_exponent`): 32 (+0.1 preIC2), with 57/88
  slots added to match the exact `base × preIC2 × postIC2` structure.
- **Tickspeed base** (`starting_tickspeed_mult`): 36 (×1/1.02), 45 (×0.98)
  multiply the base tickspeed interval (`Tickspeed.baseValue`).

### Decisions & why

- **52/53 (autobuyers maxed) are checked per-tick, not at REALITY_RESET_AFTER.**
  The original's listed event is a Reality reset, but a Reality clears autobuyer
  intervals, so a post-reset check can never see them maxed — they're only
  reachable mid-run. They carry no production effect, so the unlock timing has
  no numeric consequence; a guarded per-tick check keeps them reachable.
- **47/48 fire on the crunch that banks the challenge completion.** The original
  also registers them on Reality events, but a Reality clears challenge
  completions, so the crunch is the meaningful seam.
- **41's reward is a no-op here** (the `ipMult`/`ipOffline` upgrades it unlocks
  are not modelled), but the condition (`≥16` infinity upgrades, counting both
  the grid and Break-Infinity bitmasks like the original's single string set) is
  wired so its bit is set faithfully.

### Deferrals

- **35 (6-hour offline)** — no wall-clock `lastUpdate` model; excluded from
  `IMPLEMENTED_ACHIEVEMENTS` (only ever set via auto-achieve / ACHNR).
- **22 (News)** — unmodelled, as before.

### Surprises

- Unlocking *any* achievement bumps the global `achievement_power` (×1.03), so
  isolating a per-dimension effect in a test means comparing against a game with
  an equal-count, no-effect unlock (used achievement 11 as the baseline).
- The existing `crunch_at_threshold_resets_everything` test crunched with zero
  elapsed time, which now (correctly) trips 37/54 and raises the starting
  antimatter; gave that run a 3-hour real time to isolate the reset assertion.

### Tests

- 12 new unit tests in `achievements.rs` (33/34/36/37/54/38/46/44 conditions,
  44 reset, 48/47, 32 + sacrifice strength, 51, 31 effect).
- Full `ad-core` suite green (`--features serde`).
- Fidelity suite: **32 → 34** passing cells (the sacrifice/tickspeed/starting-AM
  effects now match the oracle on two more fixtures).

---

## Batch 2 — ids 55–78

### What shipped

Conditions + effects for: 55, 56, 57, 58, 63, 64, 66, 67, 68, 71, 72, 73, 75,
76, 77, 78. (61, 62, 65, 74 deferred — see below.)

- **Crunch-before** (fast/challenge infinities): 55 (≤1 min), 78 (≤250 ms),
  56/57/58 (NC2/NC8/NC9 in ≤3 min), 68 (NC3 in ≤10 s), 64 (challenge, no
  boosts/galaxies), 71 (NC2, one AD1, no boosts/galaxies).
- **Tick**: 63 / 77 (Infinity Power ≥ 1 / 1e6), 66 (tickspeed), 72 (all AD
  multipliers ≥ `NUMBER_MAX_VALUE`), 73 (9.9999e9999 antimatter), 75 (4th ID
  unlocked), 76 (8 days played), 61 (guarded — see below).
- **IC completion**: 67 (new `check_infinity_challenge_completed_achievements`,
  hooked in `complete_infinity_challenge`).

Effects:
- AD common multiplier (`achievement_ad_common_mult`): 56, 65, 72, 73, 74, 76.
- AD per-tier: 64 (AD1–4 ×1.25), 68 (AD1 ×1.5), 71 (AD1 ×3).
- Buy-10 multiplier: 58 (×1.01).
- Infinity-Dimension common multiplier: 75 (folds in `achievement_power`).
- Tickspeed base: 66 (×0.98). Sacrifice exponent: 57 (+0.1 preIC2, slot from
  batch 1). Starting antimatter: 55/78 (batch-1 `Effects.max` chain).

### Deferrals (unmodelled dependencies)

- **62** — `bestRunIPPM` needs a recent-infinities ring; `records` has recent
  *eternities*/*realities* but no recent *infinities*. No effect, so excluded.
- **65 / 74** — condition is `Time.challengeSum` (sum of Normal-Challenge best
  times); the engine tracks IC best times but not Normal-Challenge best times.
  Their **effects are wired** (gated on the bit) so an auto-achieved unlock still
  works; only the natural unlock is deferred.
- **61** — condition (all AD autobuyers at bulk cap) is wired and checked per
  tick, but the engine has no bulk-*upgrade* action, so it is only reachable via
  a loaded save. Excluded from `IMPLEMENTED_ACHIEVEMENTS`.

### Surprises

- A zero-time crunch now trips all four fast-Infinity achievements (37/54/55/78),
  so the starting antimatter jumps to 5e25; updated the batch-1 test.
- `NormalChallenge.isOnlyActiveChallenge` is `player.challenge.normal.current
  === id` — deliberately *not* the IC1-shared `challenge_running`; used the
  direct field check.

### Tests

- 8 new unit tests (64/68/71 effects, 58 buy-10, 66 tickspeed, 75 ID bonus, 72
  common, 73 AM-scaling, 63/77, 67).
- Full `ad-core` suite green. Fidelity unchanged at 34 (these achievements don't
  fire in the early-game fixtures).

---

## Batch 3 — ids 81–104

### What shipped

Conditions + effects for all 20 (rows 8–10): 81–88, 91–98, 101–104.

- **Crunch-before**: 81 (IC5 ≤15 s), 85/91/92/93 (pending-crunch IP thresholds,
  91/92 also time-gated).
- **Crunch-after**: 97 (IC best-times sum < 6.66 s, from `ic_best_times_ms`).
- **Tick**: 84 (1e35000 AM), 86 (tickspeed recip ≥1000), 87 (2e6 infinities),
  94 (1e260 IP power), 98 (8th ID), 102 (all Eternity milestones), 103 (1e1000 IP).
- **Galaxy-after**: 83 (50 galaxies).
- **IC-completion**: 82 (all 8 ICs).
- **Sacrifice-before**: 88 (nextBoost ≥ `NUMBER_MAX_VALUE`).
- **Replicanti-tick** (new seam in `tick`): 95.
- **Eternity-before** (new seam): 96 (go Eternal), 101 (only-AD8), 104 (≤30 s).

New effect sites:
- IP multiplier (`total_ip_mult`): 85/93 ×4 each.
- IP formula divisor (`ip_gain_divisor`): 103 → 307.8.
- Gained infinities (`gained_infinities`): 87 base → 250 (Infinity > 5 s).
- Galaxy strength (`galaxy_strength_effect`): 86 ×1.01.
- 1st Infinity Dimension (`id_multiplier`): 94 ×2.
- ID common mult (`id_common_multiplier`): 75 (batch 2) already; sacrifice
  exponent 88 (postIC2 +0.1, slot from batch 1).
- Starting tickspeed: 83 (×0.95^galaxies). Starting IP (`starting_ip`): 104 (5e25).
- AD common mult: 84/91/92.

### Decisions & why

- **85/93 self-boost their unlocking crunch's IP.** Conditions run in
  BIG_CRUNCH_BEFORE, then `big_crunch_reset` computes the reward IP — which now
  includes the freshly-unlocked ×4. The original's `totalIPMult` is cached and
  invalidated only on the next game-loop update, so JS awards the ×4 one crunch
  later. We follow the engine's compute-fresh (no-cache) philosophy; the
  difference is one crunch, only past 1e150 IP, and the fixtures never reach it.
- **88 is reachable only via the IC2-completed sacrifice formula.** With the
  i64 Decimal exponent, the normal `log10(AD1)/10` pre-power caps around 1e18, so
  `nextBoost` can't reach `NUMBER_MAX_VALUE`; but IC2-completed drops the log10,
  making it reachable. Kept in `IMPLEMENTED_ACHIEVEMENTS`.

### Surprises

- Fast eternities now trip 104 (5e25 starting IP); two eternity/perk tests were
  asserting a zero/perk-only starting IP, so gave them >30 s eternities.

### Tests

- 10 new unit tests (85/93 IP mult, 87 infinities, 94 ID1, 83 galaxies+tickspeed,
  88 sacrifice, 95 replicanti, 98/102 tick, 97 IC-sum, 96/101/104 eternity, 82).
- Full suite green. Fidelity unchanged at 34.

---

## Batch 4 — ids 105–128

### What shipped

Conditions + effects for rows 10–12: 105–108, 112–116, 118, 121–128 (111/117
conditions deferred — see below).

- **Tick**: 105 (308 free ticks), 121 (1e30008 IP), 124 (marathon2 — Infinity
  Power/s > Infinity Power for 60 s), 125 (1e90 IP, no infinities/AD1), 126
  (180× RG:AG), 127 (`NUMBER_MAX_VALUE` EP), 128 (1e22000 IP, no studies).
- **Crunch-after**: 112 (IC best-times sum < 750 ms).
- **Replicanti-tick**: 106 (10 RGs within 15 s).
- **Eternity-before**: 107 (<10 infinities), 108 (exactly 9 Replicanti), 113
  (≤250 ms), 116 (≤1 infinity), 122 (only-AD1).
- **Eternity-after** (new seam): 123 (50 EC completions).
- **Challenge-failed** (new seam in `ec_try_fail`): 114.
- **IC-start** (inline in `start_infinity_challenge`): 115.

New effect sites:
- Time-Dimension common mult: 105 (`perSecond^0.000005`), 128 (× study count).
- Gained eternities: 113 (×2).
- Soft reset (`galaxy.rs`): split the ANR gate — 111 keeps *antimatter*, perk 30
  keeps antimatter *and* dimensions (matching the original's two separate checks).
- Dimension-Boost power: 117 (×1.01).
- Sacrifice: 118 skips the AD reset (both the NC8 and normal paths).
- IP multiplier (`total_ip_mult`): 116 (`infinitiesTotal^(log10 2/4)`, ^TS31),
  125 (`2^(ln t · min(t^0.11,500))`) — both capped by the new
  `effarig_eternity_cap` (1e50 during Effarig's Eternity stage).

### Decisions & why

- **113 doubles its own eternity** (like the batch-3 85/93 case): the condition
  runs in ETERNITY_RESET_BEFORE, and `gained_eternities` is computed after —
  compute-fresh, no cache.
- **111 / 117 conditions deferred, effects wired.** 111 needs the
  recent-infinities ring (unmodelled); 117 needs a ≥750 bulk Dimension-Boost
  purchase (the engine boosts one at a time). Both effects have real consumption
  sites, so an auto-achieved unlock still behaves correctly.
- **126's RG-divide reward deferred.** Rewiring Replicanti-Galaxy gain (multi-gain
  + divide-by-`MAX`) is a larger change; the condition is wired (reachable) but
  the behavioural reward is a follow-up. Noted here rather than half-done.

### Surprises

- 113's ×2 tipped several test eternities (set up with 0 ms) to 2 total
  eternities, tripping the keepAutobuyers milestone; gave the shared
  `game_at_eternity_goal` helper and two autobuyer tests realistic (>30 s / >250
  ms) eternity times.

### Tests

- 10 new unit tests (128 TD, 111 AM-keep, 113 eternities, 117 dim-boost, 118
  sacrifice, 116 IP mult, 107/108, 123, 114, 124 marathon).
- Full suite green. Fidelity unchanged at 34.
