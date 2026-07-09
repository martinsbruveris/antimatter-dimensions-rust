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

## 2.2 — Infinity Upgrades bottom row (`ipMult` + `ipOffline`)

**What shipped:** the Achievement-41 bottom row is now fully modelled. Engine:
the `ipMult` rebuyable's two-regime cost curve (×10 steps to 1e3M, ×1e10 steps
to the 1e6M cap), single purchase + the original's two-phase geometric-series
`buyMax` (via new `Decimal::afford/sum_geometric_series` helpers ported into
`break_infinity`), the Big-Crunch-autobuyer dynamic-amount ×2 bump per purchase
(TS181 suppresses it), the `ipOffline` one-time purchase, and the offline
catch-up award (`offline_currency_gain`, called engine-side by
`simulate_offline` and once by the GUI's chunked replay). The IP-mult autobuyer
(1-Eternity milestone) ticks `buy_max_ip_mult` every update. GUI: the bottom
row on the Infinity Upgrades tab (multiplier tile + spoon buttons + ipOffline
tile, vendored classes), new Tauri commands, and the cost-cap footer.

**Fidelity fixes found along the way:**
- `playerInfinityUpgradesOnReset` is now a shared faithful port: it honours
  Reality Upgrade 10 (previously the Reality reset always cleared the upgrade
  bitmasks and `apply_rupg10` never restored them) and grants/clears
  `ipOffline` with the milestone keeps (the original's keep *sets* include it).
- The `ipMult` effect in `total_ip_mult` now carries Effarig's Eternity-stage
  cap (the E1E6 default cap is above the natural e993k max, so only Effarig's
  can bind).

**Deviations:** the original gates the `ipOffline` award on
`player.options.offlineProgress`; that toggle is an 8.8 gap and offline
progress is always on here, so the award is ungated (noted in the code).

**Tests:** eight new unit tests (gating, cost curve, geometric-series buyMax,
threshold crossing, cap, autobuyer bump, ipOffline award, milestone keeps).
Fidelity grid unchanged (1118/1148).
