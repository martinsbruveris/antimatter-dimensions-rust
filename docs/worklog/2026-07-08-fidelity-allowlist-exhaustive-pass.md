---
date: 2026-07-08
feature: fidelity
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
---

# Fidelity — exhaustive allowlist reclassification (full player-tree walkthrough)

## Summary
Walked the entire `player` tree field-by-field with the user and reclassified ~60
fields from `opt`/`rt` (skip) to include, each verified against the original JS
engine. Added two comparison modes (`AutomatorMode`, `AutomatorStack`) to handle
the Automator run-state. The one near-universal gap the pass surfaced
(`records.thisInfinity.lastBuyTime`, which encode never wrote) was fixed in the
same pass, so fixtures 0–7 stay green: **net 32/312 with ~60 more fields verified
per cell.**

## What shipped
- **`compare.rs`:** two new `Compare` variants.
  - `AutomatorMode` — treats an absent side as `1` (pause), because JS omits
    `state.mode` on a never-run Automator while Rust always writes it (would
    otherwise false-positive on every save).
  - `AutomatorStack` — depth/order-sensitive frame compare; `lineNumber` +
    `commandState` shape exact, WAIT `commandState.timeMs` numerically tolerant.
- **`allowlist.rs`:** additions across every section —
  - Game-time records (`totalTimePlayed`, `this*.time`, `best*.time`,
    `thisInfinity.lastBuyTime`, `timePlayedAtBHUnlock`) and the `recent*` rings
    (`Decimal`, order-sensitive element-wise).
  - Black holes: `phase`, `activations`, `blackHolePause`, `blackHolePauseTime`,
    `blackHoleAutoPauseMode`.
  - `challenge.{normal,infinity}.bestTimes[]` (game-time, gate reward multipliers).
  - Autobuyers: `auto.*.lastTick`, galaxy/dimBoost caps, `disableContinuum`.
  - Timestudy `presets`/`preferredPaths`, top-level `respec`.
  - Reality: `reqLock.reality`, `respec`, `autoAchieve`, `gainedAutoAchievements`,
    the glyph-automation toggles (`autoEC`, `autoAutoClean`, `applyFilterToPurge`,
    `hasCheckedFilter`, `autoSort`, `autoCollapse`, `moveGlyphsOnProtection`), and
    the Automator run-state (`state.{topLevelScript,repeat,forceRestart,mode,stack}`,
    `execTimer`, `forceUnlock`).
  - Celestials: Effarig `glyphWeights`/`autoAdjustGlyphWeights`; Enslaved
    `storedReal`/`autoStoreReal`/`isAutoReleasing`; Ra `momentumTime`; Lai'tela
    `thisCompletion`/`fastestCompletion` + DMD `timeSinceLastUpdate`.
- Module + section docs rewritten to the refined include principle.
- **`ad-core` `lastBuyTime` round-trip fix:** `ThisInfinityDTO` now reads the real
  `lastBuyTime` key (was approximated from `thisInfinity.time`) and `encode.rs`
  writes it back — closing the one near-universal gap this pass introduced.

## Decisions & why
- **Refined the skip rule.** The old "skip real-time/game-time as bookkeeping" was
  too broad. New rule: a time-based field that *feeds a mechanic* (game-speed
  timer, completion timer gating a reward, a resource banked over time) is
  engine-relevant and reproducible (the harness feeds both engines the same diff),
  so it's included. Only *pure* real-time bookkeeping nothing reads
  (`realTimePlayed`, `this*.realTime`) and wall-clock snapshots (`Date.now`:
  `lastUpdate`, `backupTimer`, `zeroHintTime`) stay out.
- **Automator effects vs run-state.** The Automator runs during a tick
  (`tick.rs:211`) and its effects ride the economy comparison; this pass adds its
  *run-state* for direct lockstep checking. Scripts/constants (program input) were
  left out; editor/UI sub-fields stay skip.
- **Kept skips:** all `quoteBits`; glyph loadout snapshots (`teresa.bestAMSet`,
  `v.runGlyphs`) — display-only, redundant with the glyph comparison;
  `requirementChecks.permanent.*` (secret achievements only, and mostly
  non-reproducible); Enslaved hints; Lai'tela/Pelle display toggles.

## Deviations from the design doc
- Supersedes several first-pass celestial skip notes (`storedReal`, `momentumTime`,
  `thisCompletion`/`fastestCompletion`, `timeSinceLastUpdate`) — all now included.
  The design doc's §5 table predates this; left unchanged (historical).

## Surprises & gotchas
- The whole ~60-field pass introduced exactly one near-universal divergence,
  `records.thisInfinity.lastBuyTime` (encode never wrote it; the DTO approximated
  it from `thisInfinity.time`). That it was *only* one field is the validation the
  additions are sound: every other addition + both new compare modes pass on
  fixtures 0–7, and the roundtrip identity guard still passes. Fixing that one
  round-trip restored the count.

## Follow-ups
- Deferred gaps to model next (each recovers cells): the earlier rate records; the
  many `(gap)`-tagged fields added this pass (glyph-automation toggles, autobuyer
  subsystems, Effarig weights, `disableContinuum` encode write, `recentInfinities`
  ring, …).
- Capture a fixture with an active Automator — nothing exercises the run-state
  (`mode`/`stack`) path yet.

## Tests
- `cargo test -p ad-fidelity`: 6/6 (incl. `roundtrip_identity_holds_over_allowlist`).
- `cargo test -p ad-core --features serde`: 29/29.
- `cargo run -p ad-fidelity`: **32/312** — unchanged count (fixtures 0–7 green),
  ~60 more fields verified per cell.
- `cargo fmt` / `cargo clippy` (both crates): clean.
