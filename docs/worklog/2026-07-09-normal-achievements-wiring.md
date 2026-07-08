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
