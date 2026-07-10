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
