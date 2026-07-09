---
date: 2026-07-09
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
---

# Fidelity fixes — achievement 28 on bulk buys, and the Big Crunch goal freeze

Working the fidelity suite from the first failing cell,
`00008-0016-42-49-manual @ 100`. That fixture is an early-game (~16 min) save
that reaches the Infinity wall between horizons 10 and 100. Two distinct bugs
surfaced from its divergences; this file covers both.

## Bug 1 — achievement 28 unlocked on the bulk "buy max" path

### Symptom
`achievementBits` row 2 diverged: Rust had bit 8 (achievement 28) set, JS did
not. Achievement 28 ("There's no point in doing that…") gives 1st ADs ×1.1 *and*
bumps the global achievement power (×1.03 per unlock), so the wrong bit also
inflated every dimension's multiplier.

### The bug
The original only calls `Achievement(28).tryUnlock()` inside `buyOneDimension`
(the genuine single-buy) — never in `buyManyDimension`, `buyAsManyAsYouCanBuy`,
or `buyUntilTen`. Its `checkEvent` (`ACHIEVEMENT_EVENT_OTHER`) is a dead event
that is never dispatched, so the only other route is the post-Reality
auto-achiever.

Our engine unlocked 28 inside `on_buy_dimension`, which fires from *every* buy
path. In particular the AD1 autobuyer's "Buys max" mode runs
`buy_max_dimension_bulk` → `buy_until_10_dimension` → `buy_dimension` →
`on_buy_dimension`, so a bulk AD1 purchase over 1e150 wrongly awarded 28.

### The fix
Split `buy_dimension` into the public single-buy (which keeps the tier-0,
≥1e150 achievement-28 check, mirroring `buyOneDimension`) and a private
`buy_one_dimension` core that performs the purchase + `on_buy_dimension` *without*
the 28 check. `buy_until_10_dimension` now loops `buy_one_dimension`, so the bulk
paths no longer touch achievement 28 — matching the original's split exactly.

### Verification
`achievementBits` no longer diverges. On its own this fix leaves the grid at 34
cells (the fixture still fails on the production divergence below), but it is a
genuine correctness fix. `cargo test -p ad-core --features serde` stays green.
