---
date: 2026-07-07
feature: 7.7
design_docs:
  - ../design/2026-07-07-pelle.md
---

# Celestial 7 — Pelle, The Doomed (the final Celestial)

## Summary
Ported Pelle's self-contained layer end-to-end: dooming/Armageddon, Remnants →
Reality Shards, the 5 Rifts, Strikes, Pelle Upgrades, the Galaxy Generator, and
the antimatter game-end. Engine + save/load + a Pelle subtab. Pelle unlocks
behind Imaginary Upgrade 25 (built in 7.6). This completes Phase 7.

## What shipped
- **`pelle.rs`:** `PelleState` — `doom_reality` (permanent, gated on iU25) +
  `armageddon` (Remnants from the doomed records, `((Σ log10(x+2))/1.64)^7.5`);
  Reality Shards accrue per second (`pelle_tick`). The 5 Rifts (fill/percentage/
  `percentageToFill`/effect/3 milestones each), drained 3%/s from IP/Replicanti/
  decay-%/EP/DT; the Strikes (`progress_bits`, triggered from the crunch/eternity/
  galaxy/dilate paths + a 115-TT check) that unlock each rift; the 5 rebuyable +
  23 one-time Pelle Upgrades (Reality Shards); the Galaxy Generator (gain/cap/
  phase/sacrifice); and `game_end_state` → `is_game_end`.
- **Effects wired at their sites:** the doomed `antimatterDimensionMult` +
  `timeSpeedMult` rebuyables, the Infinity-Strike AD `^0.5` penalty, and the
  Paradox rift's all-Dimension power. `pelle_is_disabled(mechanic)` exposes the
  doom's disables as a query.
- **Save/load:** `PelleDTO` (with a string-or-number Decimal reader for chaos's
  numeric `fill`) + the new root `is_game_end` round-trip.
- **GUI:** a Pelle subtab (the Doom button pre-doom → Remnants/Reality-Shards
  header + Armageddon, the 5 rift bars with milestone dots + drain toggles, the
  Pelle Upgrade grid, the Galaxy Generator, and a plain game-end bar).

## Decisions & why
- **Self-contained layer, not the full disable sweep.** Pelle's defining behavior
  is disabling ~30 mechanics engine-wide via `isDisabled`. Porting every guard is
  a huge, tedious cross-cutting change; this port models the Pelle *layer*
  faithfully and exposes `pelle_is_disabled` + `is_doomed`, wiring only the
  highest-impact seams (doomed AD/time-speed, the AD `^0.5` strike, the paradox
  power). The rest is a **documented cut**.
- **Cosmetics cut.** The credits sequence, its song, and the `zalgo`/`wordShift`
  text corruption are omitted entirely (out of scope for the engine).
- **`is_game_end` on `GameState`.** Mirrors `player.isGameEnd`; the game-end is a
  pure `endState ≥ 1` check on the doomed total antimatter (~1e9e15).

## Deviations from the design doc
- The "keep on Armageddon" upgrade gates and several deep rift-milestone effects
  are stored but not fully wired (their targets need the disable sweep or unbuilt
  records). Strike penalties beyond AD/ID `^0.5` are documented, not all wired.

## Tests
- New `ad-core` unit tests across `pelle` (dooming, remnants scaling, strikes
  unlocking rifts, the dilation-strike record reset, rebuyable caps, game-end),
  plus the Pelle fields in the celestials save round-trip. `cargo test -p ad-core
  --features serde` = 456 lib pass; `cargo clippy` clean on `ad-core` + `ad-gui`;
  frontend `npm run build` green.

## Follow-ups
- The full `isDisabled` disable-everything sweep; the "keep on Armageddon" reset
  gates; the special Pelle glyph effect; the deep rift-milestone effects; the
  Galaxy-Generator glyph-slot nicety. Phase 7 (all seven Celestials) is now
  functionally complete.
