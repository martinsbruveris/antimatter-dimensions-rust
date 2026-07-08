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
| 2026-07-07 | `1bff354` | Widening coverage, verifying ~60 more fields per cell. Fixed some small issues involving non-modelled fields. Obtaining new baseline. | 32 (−8) |

## Method

Run fidelity suite against fixture set via

```sh
cargo run -q -p ad-fidelity -- crates/ad-fidelity/saves/fixtures
```

Append new count-changing commits to the table as fidelity work continues.
