---
date: 2026-07-07
feature: 7.5
design_docs:
  - ../design/2026-07-07-ra.md
---

# Celestial 5 — Ra + Glyph Alchemy

## Summary
Ported Ra (Feature 7.5) end-to-end: the four Celestial-Memory pets, the 28
level-unlock rewards, Remembrance, the charged-Infinity-Upgrade gate, momentum,
Ra's Reality, and the 21-resource Glyph Alchemy lab with its reaction engine and
Glyph refinement. Engine + save/load + a Ra subtab. Ra unlocks behind the
existing V 36-Space-Theorem `raUnlock` bit.

## What shipped
- **`celestials/ra.rs`:** `RaState` (`player.celestials.ra`) — 4 pets, unlock
  bits, run flag, charged set, disCharge, peakGamespeed, petWithRemembrance,
  momentumTime, alchemy array, refinement ratchet. Pets accrue memory chunks
  from real time (`ra_memory_tick` in `tick.rs`, chunks off while storing real
  time), level up (`requiredMemoriesForLevel`), and buy the two per-pet
  upgrades. All 28 unlocks (`RA_UNLOCK_REQS`) flip on level-up via
  `ra_check_for_unlocks`; effect readers for the XP/theorem/achievement rewards.
- **`celestials/alchemy.rs`:** 21 resources (base + reaction), caps
  (`min(25000, highestRefinementValue)` / `min(reagent caps)`), the reaction
  engine (`reaction_yield` / `actual_yield` / `priority` / `combine_reagents`,
  `apply_alchemy_reactions` per rewarded Reality), and Glyph refinement wired
  into `sacrifice_glyph` (a basic Glyph refines into its type's resource once
  Alchemy is unlocked, with the `decoherence` spill).
- **Effects wired at their sites:** AD/ID/TD alchemy powers (power/infinity/
  time) + `dimensionality`/`momentum`/`force`/`inflation`; `exponential` IP;
  `replication`/`dilation` + `continuousTTBoost.replicanti`/`.dilatedTime`/
  `.ttGen`/`.infinities`/`.eternities`; `peakGamespeedDT`; `effarig` relic-shard
  mult; `achievementPower` (`^1.5`); `achievementTTMult`; `relicShardGlyph
  LevelBoost`; `unlockHardV` (V's `v_is_flipped`).
- **Save/load:** `RaDTO` ↔ `player.celestials.ra` (pets object, charged set ↔ id
  array, petWithRemembrance string ↔ index, alchemy array, refinement object).
  Round-trips; pre-Ra saves default.
- **GUI:** a Ra subtab (pets with level bars + upgrade/level/remembrance
  buttons, milestone icons, the run button + charged counter, the Alchemy
  resource grid with reaction toggles); vendored `celestial-ra` CSS.

## Decisions & why
- **Unlock = V's `raUnlock`.** Ra's tab/features gate on `v_space_theorems() >=
  36`, matching `Ra.isUnlocked`; no new unlock plumbing.
- **Charged Infinity Upgrades: state, not variants.** The charged set + count
  gate (`min(12, ⌊teresa.level/2⌋)`) + discharge-on-Reality are modelled and
  round-trip; the ~11 per-upgrade *charged effect variants* are deferred
  (disproportionate wiring for late-game power tweaks). Documented.
- **Refinement replaces sacrifice.** Once Alchemy is unlocked, a basic Glyph's
  sacrifice refines it into the type's resource (`attemptRefineGlyph`), which is
  the only way resources grow — so it had to be wired for Alchemy to function.
- **`unpredictability` mean, not RNG.** The Poisson re-trigger is modelled by its
  mean extra-run count (no divergence risk — it only speeds fill).

## Deviations from the design doc
- `uncountability` passive Reality/PP generation is deferred (our `realities` is
  a `u32`; fractional passive gen needs an accumulator) — noted in the reader.
- `boundless`/`multiversal` effect readers are omitted (their targets —
  tesseract strength / Reality amplification — are inert/unbuilt); the resources
  still store, cap, and react.
- The `reality` resource's Reality-Glyph creation is deferred (no Reality glyph
  type yet, same cut as Effarig's glyph type).

## Tests
- New `ad-core` unit tests across `ra` (unlock gate, leveling→pet unlock,
  required-memories curve, charges, memory tick, Remembrance) and `alchemy`
  (refinement cap ratchet, a reaction, effect readers), plus the Ra fields in
  the celestials save round-trip. `cargo test -p ad-core --features serde` =
  441 lib pass; `cargo clippy` clean; `cargo check -p ad-gui` + frontend
  `npm run build` green.

## Follow-ups
- Charged Infinity-Upgrade effect variants; `uncountability` generation (needs
  fractional realities); the Reality-resource glyph; `improvedStoredTime` /
  `unlockDilationStartingTP` deeper wiring. Several unblock with Pelle (which
  disables many Ra unlocks) — the `disabledByPelle` guard is stubbed
  (`ra_unlock_active` = `ra_has_unlock`) until Pelle lands (7.7).
