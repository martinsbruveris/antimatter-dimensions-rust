---
date: 2026-07-09
feature: multiple (1.3, 2.2, 2.4, 4.2, 4.4, 5.2, 6.2–6.5, 6.7)
design_docs:
  - ../design/2026-07-09-port-audit.md
---

# Port completion pass — working through the 2026-07-09 audit backlog

## Summary

A multi-feature session driving the deferred items from the 2026-07-09 port
audit to done, one feature at a time. Sections are appended per feature as they
land (each feature is one or more commits).

## 1.3 — Closed-form bulk buyers

**What shipped:** `buy_max_tickspeed` is now a faithful port of the original's
`buyMaxTickSpeed`: outside NC9 it uses the analytic
`CostScale::get_max_bought` (the `ExponentialCostScaling` closed form, charging
only the most expensive purchase), under NC9 it loops purchase-by-purchase so
each buy applies the equal-cost bump and stops at the Big Crunch goal.
`buy_tickspeed` gained the original's full `isAvailableForPurchase` guard
(unlocked + not EC9 + not Continuum + pre-break cost cap).

**Surprises:**
- The AD-side bulk buy (`buy_max_dimension_bulk`) already used the closed form;
  the audit's 1.3 note was stale on that half. Only Tickspeed still looped.
- The closed form is *not* purchase-count-equivalent to a single-buy loop: it
  charges only the top purchase, so with 1e40 AM it buys 38 upgrades where the
  cumulative loop affords 37. That is the original's deliberate simplification,
  and the fidelity suite confirmed it: the change moved the grid from
  **1099/1148 → 1118/1148** (+19 cells).

**Tests:** two new unit tests in `tickspeed.rs` (top-purchase-only charging with
exact counts; the super-exponential branch); the two integration tests that
bought tickspeed from a fresh state now unlock it first (AD2 purchase), since
`buy_tickspeed` carries the original's unlock guard.
