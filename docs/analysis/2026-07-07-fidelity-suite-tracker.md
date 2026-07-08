# Fidelity suite tracker

Progress of the `ad-fidelity` save-replay suite (design:
[`../design/2026-07-06-fidelity-testing.md`](../design/2026-07-06-fidelity-testing.md)),
measured as the number of passing **grid cells** across the commit history.

A *cell* is one (fixture × horizon) comparison. The current suite uses 4 horizons 
(1, 10, 100, 1000 ticks) and the number of fixtures will grow over time. A cell
passes when every allowlisted `player` field matches the JS oracle within the
log-space tolerance (`1e-6`).

## Progress

Commits that don't change the count are omitted.

| Date | Commit | What changed | Passing cells |
| ---------- | --------- | ------------------------------------------------------------- | ------------- |
| 2026-07-07 | `923c2ea` | Introduced the save-replay harness (capture → JS oracle → Rust diff) — baseline | 20 |
| 2026-07-07 | `fb05716` | Fixed AD production (dimension→dimension feed ran 10× too fast) and the "Buys max" group/bulk buying | 35 (+15) |
| 2026-07-07 | `21d2f2a` | Restored the autobuyer timer phase from the save's `lastTick` on load (was reset to 0) and aligned `advance` with the JS pre-increment interval check | 38 (+3) |
| 2026-07-07 | `8dcda3e` | Gated each interval autobuyer's timer reset on its `canTick` readiness, so phase accrues while waiting to afford instead of restarting each interval | 39 (+1) |
| 2026-07-07 | `650d255` | Rounded the dimension amount after a "Buys max" group purchase (mirroring `buyUntilTen`), dropping lingering fractional production stock | 40 (+1) |
| 2026-07-07 | _(uncommitted)_ | **Coverage widening, not a fidelity regression.** Made the allowlist exhaustive over every engine-relevant field (all Celestials + Imaginary Upgrades + previously-omitted core/records/requirement/autobuyer gaps), including fields `ad-core` does not model yet so the suite *showcases* them. A handful of near-universal gaps (`postC4Tier`, `requirementChecks.*` run-flags, `records.thisReality.maxAM`) are non-default in essentially every save, so every cell now carries at least one known-gap divergence | 0 (−40) |
| 2026-07-08 | _(uncommitted)_ | Modelled the near-universal gaps the widening exposed: round-tripped `postC4Tier` + `records.thisReality.{maxAM,maxIP}` (peak-tracked in `tick`) and modelled the AD-purchase / AM-gain / sacrifice / galaxy `requirementChecks` flags (`eternity.{onlyAD8,onlyAD1,noAD1}`, `reality.noAM`, `infinity.{maxAll,noAD8,noSacrifice}`). Recovers early fixtures 0–7; late fixtures 49/70 still blocked by the IP/infinity **rate records** (`bestInfinitiesPerMs`, `bestIPMsWithoutMaxAll`, `bestIPminEternity`), deferred as they need full crunch-statistics modelling | 32 (+32) |

## Method

Run fidelity suite against fixture set via

```sh
cargo run -q -p ad-fidelity -- crates/ad-fidelity/saves/fixtures
```

Append new count-changing commits to the table as fidelity work continues.
