---
date: 2026-07-07
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
---

# Fidelity fix — "Buys max" rounds the dimension amount

## Summary
The next failing fixture after the autobuyer-timer work, `00007-0015-46-07-timed`,
diverged only at tick 1000 with tiny deltas (Δlog10 ~1e-4). A dense trace pinned
the first divergence to tick 534, in a single field — `dimensions.antimatter[3]
.amount` — exactly when the 4th Antimatter Dimension autobuyer completed a group
(bought 10→20). Root cause: the original rounds the dimension amount after a
"Buys max" group purchase; we did not.

## The bug
The original has two group-buy paths that differ by one `Decimal.round`:
- `buyManyDimension` (the manual "buy until 10" / M hotkey): `amount = amount +
  remainingUntil10` — **no** round.
- `buyUntilTen` (used inside `buyMaxDimension`, i.e. "Buys max" and the AD
  autobuyer's BUY_10 mode): `amount = round(amount + remainingUntil10)` —
  **rounds**.

Our `buy_max_dimension_bulk` completed its groups by looping `buy_dimension`
(each `amount += 1`, no round) via the shared `buy_until_10_dimension`, so the
fractional stock a dimension accrues from production between purchases lingered
instead of being rounded off. Only the analytic bulk branch rounded.

Concretely at tick 534: AD4 held `889.4469`; buying the group adds 10.
- JS: `round(899.4469) = 899`, then +1.04 production → `900.0399`.
- Rust: `899.4469`, then +1.04 → `900.4868`.

The 0.447 gap is the rounded-off fraction. It stays a per-purchase ~0.1% wobble
(here it only crept over the 1e-6 eps by tick 1000), and higher tiers show it
more because their amounts are smaller.

## The fix
Round the dimension amount after each group completion in
`buy_max_dimension_bulk` — after the first-group `buy_until_10_dimension` and
after each iteration of the NC9/IC5 loop — mirroring `buyUntilTen`. The shared
`buy_until_10_dimension` (the manual `BuyUntil10Dimension` action = JS
`buyManyDimension`) is left unrounded, matching the original's split.

## Verification
- `trace t.json` (fixture 7): first divergence 534 → **none over 1000 ticks**.
- Fidelity grid: 39 → **40** cells, no regressions.
- `cargo test -p ad-core --features serde`: 507 pass; fmt + clippy clean.
