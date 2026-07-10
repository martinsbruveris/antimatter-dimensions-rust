---
date: 2026-07-10
feature: port-gap-clusters
design_docs:
  - ../design/2026-07-09-port-audit.md
---

# Closing the port-audit gap clusters (1–4)

## Summary
A working session implementing the four remaining game-mechanics clusters from
the 2026-07-09 port audit, one at a time: (1) the Tesseract cluster
(7.3/7.6/4.5), (2) Pelle's `isDisabled` sweep (7.7), (3) per-celestial polish
(7.2/7.4/7.5/7.6), (4) glyph extras (6.2). Every step keeps the fidelity suite
at its 1469/1476 baseline (the 7 residuals are documented precision bounds).

## Cluster 1 — the Tesseract cluster

### 1a. EC completions u8 → u16 + EC1 goal-1000 in Enslaved

`maxCompletions` is dynamic in the original: `Enslaved.isRunning && id === 1 ?
1000 : 5`. Our engine had a constant `EC_MAX_COMPLETIONS: u8 = 5` baked into
the goal formula, the completion banking, the pending-completion scan, and the
EC1 study requirement.

- Widened `GameState.eternity_challenges` to `[u16; 12]` (and the glyph-undo
  snapshot's `ecs`, the automator's `last_ec_completions`, the GUI view field,
  the save decode — which now clamps EC1 at 1000, others at 5).
- Added `ec_max_completions(id)` and threaded it through `ec_goal_at`
  (`goalAtCompletions` clamps at `max − 1`, so EC1's goal keeps scaling to
  1e1800 × (1e200)^999 inside the run), `complete_running_ec`,
  `ec_pending_total_completions`, and EC1's secondary study requirement
  (`20000 + min(completions, isRunning ? 999 : 4) × 20000`).
- Test: cap flips 5 ↔ 1000 with the run flag, goal scaling past 5, requirement
  scaling, banking an 11th completion. Fidelity: 1469/1476 (unchanged).

### 1b. Tesseracts

The ID-purchase-cap currency (`Tesseracts` in `enslaved.js`), previously
state-only. Engine: the hardcoded `BASE_COSTS` table (`10^(1e7·base)` IP,
a *threshold* — buying does not spend IP), `can_buy/buy_tesseract` gated on a
completed Enslaved run, `effective_count` (scaled by the
`tesseractMultFromSingularities` milestone, now implemented:
`1 + log10(singularities)/80`), and `capIncrease` (`250e3 × 2^total`, times
`boundless + 1` — the `boundless` alchemy effect accessor also landed:
`amount/80000`). Wired: the ID purchase cap (`id_purchase_cap` is now
instance-scoped and adds `floor(capIncrease)`), the `darkFromTesseracts`
singularity milestone (`1.1^effectiveCount` into the common dark multiplier),
and IU23's real effect (`floor(0.25 × effectiveCount²)` multiplying IU12's
free Dim Boosts — the two stubs the audit called out). GUI: the vendored
tesseract button on the Infinity Dimensions tab (visible once Enslaved is
completed), `buy_tesseract` command + store action.

Tests: cost table/threshold semantics, cap raise on `id_is_capped`, IU23
scaling. Fidelity: 1469/1476 (unchanged).

### 1c. Real-time storage, amplified Realities, auto-release/auto-store

The remaining Enslaved mechanics:

- **Real-time storage** (`isStoringReal`): a storing tick now mirrors
  `realTimeMechanics` — only Ra memories/momentum + Dark Matter Dimensions run,
  real-time records advance, the interval banks at 70% into `storedReal`
  (cap 8 h + 1 h/Nameless level via Ra's `improvedStoredTime`, self-stopping at
  the cap), autobuyers still tick, everything else freezes. The offline path
  (`autoStoreReal`) banks what fits under the cap in `simulate_offline` and
  simulates the remainder.
- **Amplified Realities** (`boostReality` — a module flag in the original, so
  deliberately `serde(skip)`): `realityBoostRatio`/`canAmplify`, the
  `simulatedRealityCount` machinery with the `partSimulatedReality` fractional
  carry (new save passthrough) and the `multiversal` alchemy effect accessor
  (`32·(amount/25000)²`), threading the multiplier through RM / Realities /
  Perk Points / Relic Shards / the recent-Realities ring, and consuming
  `storedReal` (proportionally under 1 real second). The Achievement-154
  binomial extra stays unmodelled (the engine avoids unseeded randomness).
  IU13's projected-RM requirement now includes the amplification factor.
- **Ra auto-release** (`isAutoReleasing`, persisted): every 5th tick discharges
  1% (`useStoredTime(true)`), keeping 99% banked; the release path also gained
  the full `canRelease` guard set (real-time storing / EC12 / Lai'tela / Doomed
  / auto-inside-run) and the `peakGamespeed` update. Game-time storage now
  applies Ra's `20^level` amplification (previously a dead accessor).
- GUI: EnslavedTab grew the real-time-storage half (store/auto-store buttons,
  efficiency/cap lines) and the auto-release toggle (Ra `autoPulseTime`-gated);
  the Glyphs tab gained the original's `RealityAmplifyButton`. Commands:
  `toggle_store_real_time`, `toggle_auto_store_real`, `toggle_boost_reality`,
  `toggle_auto_release`.

Tests: storing-tick freeze/banking/cap-stop, offline banking remainder,
5-tick auto-release cadence, amplified-Reality reward multiplication +
consumption. Fidelity: 1469/1476 (unchanged). **Cluster 1 complete.**
