# Fidelity suite tracker

Progress of the `ad-fidelity` save-replay suite (design:
[`../design/2026-07-06-fidelity-testing.md`](../design/2026-07-06-fidelity-testing.md)),
measured as the number of passing **grid cells** across the commit history.

A *cell* is one (fixture × horizon) comparison. The current suite has **312
cells** = 78 captured savefiles × 4 horizons (1, 10, 100, 1000 ticks). A cell
passes when every allowlisted `player` field matches the JS oracle within the
log-space tolerance (`1e-6`).

## Method

Each commit from the harness's introduction (`923c2ea`) to `HEAD` was checked
out and its own `ad-fidelity` binary run against **the current fixture set**
(passed explicitly, since the default fixtures path moved from `oracle/fixtures`
to `saves/fixtures` in `a1bed75`):

```sh
cargo run -q -p ad-fidelity -- crates/ad-fidelity/saves/fixtures
```

Holding the fixtures fixed makes the count a yardstick for the **engine +
harness** at each commit. Fixtures are git-ignored, so they persist across
checkouts. The suite has grown over time (older comparison logic evolved), so
treat cross-commit numbers as directional, not exact apples-to-apples.

## Progress (only commits that changed the count)

Intervening commits that left the count unchanged are omitted; see the note
below for what they were.

| Date | Commit | What changed | Passing cells |
| ---------- | --------- | ------------------------------------------------------------- | ------------- |
| 2026-07-07 | `923c2ea` | Introduced the save-replay harness (capture → JS oracle → Rust diff) — baseline | 20 / 312 |
| 2026-07-07 | `fb05716` | Fixed AD production (dimension→dimension feed ran 10× too fast) and the "Buys max" group/bulk buying | 35 / 312 (+15) |
| 2026-07-07 | `21d2f2a` | Restored the autobuyer timer phase from the save's `lastTick` on load (was reset to 0) and aligned `advance` with the JS pre-increment interval check | 38 / 312 (+3) |
| 2026-07-07 | `8dcda3e` | Gated each interval autobuyer's timer reset on its `canTick` readiness, so phase accrues while waiting to afford instead of restarting each interval | 39 / 312 (+1) |
| 2026-07-07 | `650d255` | Rounded the dimension amount after a "Buys max" group purchase (mirroring `buyUntilTen`), dropping lingering fractional production stock | 40 / 312 (+1) |

## Omitted (measured, count unchanged at 35)

Between `fb05716` and `21d2f2a` the count held at 35 across six commits, so they
are not in the table above: `09f83bd` (disable ad-sim integration tests),
`5045d47` (Ra), `eb40ba0` (Lai'tela), `0f23385` (Pelle), `a1bed75` (fix saves
default path), `4bac5be` (add trace mode). The three Celestial implementations
add late-game systems the early-game capture fixtures don't exercise, so they
neither help nor hurt this suite. `a558bf4` (`cargo fmt`, between `21d2f2a` and
`8dcda3e`) likewise held at 38, confirming it was behaviour-neutral.

## Reproducing / extending

Re-run the sweep with `scripts`-style loop over `git log --reverse
923c2ea^..HEAD`, checking out each commit and grepping the harness's
`N/total cells passed` summary line. Append new count-changing commits to the
table as fidelity work continues.
