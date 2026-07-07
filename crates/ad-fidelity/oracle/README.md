# Oracle (Playwright)

The reference-generation side of the save-replay fidelity harness (design:
[`docs/design/2026-07-06-fidelity-testing.md`](../../../docs/design/2026-07-06-fidelity-testing.md)
§6, §10). It runs the **actual** JS game in headless Chromium, ticks each input
save forward deterministically, and writes the resulting savefiles as fixtures.
The Rust replay harness (to come) diffs its own ticks against these.

Why a real browser rather than jsdom: the game's tick loop is welded to its UI
(`gameLoop` → `GameUI.update`, `EventHub`, `Tabs`), so running it headless-in-Node
would mean stubbing the UI — the shimming we explicitly rejected. Chromium runs
the shipped bundle with zero API gaps. See design §10.

## Usage

1. **Make the game reachable** at `GAME_URL` (default `http://localhost:8080`):

   ```bash
   cd ../../../../antimatter-dimensions
   npm install && npm run serve       # dev server at :8080
   ```

2. **Install and run:**

   ```bash
   cd crates/ad-fidelity/oracle
   npm install                 # pulls Playwright
   npx playwright install chromium
   npm run generate            # reads ../saves/captures, writes ../saves/fixtures
   ```

Output: one `fixtures/<save>.json` per input, containing the input save and the
expected savefile after each horizon:

```json
{
  "meta": { "sourceSave": "01_pre_big_crunch.txt", "tickMs": 50, "horizons": [1,10,100,1000], ... },
  "input": "<savefile>",
  "expected": { "1": "<savefile>", "10": "...", "100": "...", "1000": "..." }
}
```

## Configuration (env vars)

- `GAME_URL` — where the game is served (default `http://localhost:8080`).
- `SAVES_DIR` — input saves (default `../saves/captures`).
- `OUT_DIR` — fixture output (default `../saves/fixtures`, git-ignored).
- `TICK_MS` — tick granularity, default 50 (design §10; a parameter).
- `HORIZONS` — comma list, default `1,10,100,1000`.

## Determinism controls (design §10)

Applied so the oracle is reproducible and matches the Rust engine's assumptions:

- **Clock** — `Date.now` is overridden with a fixed, per-tick-advanced counter
  (reset per save), so `realDiff`, records, and timers are deterministic.
- **Ambient loop** — the game's `GameIntervals` are stopped; ticks are driven
  explicitly with `gameLoop(TICK_MS)`.
- **Offline** — `GameStorage.offlineEnabled = false` before import, so loading a
  save does not trigger offline catch-up.
- **Replicanti** — `poissonDistribution` / `binomialDistribution` are mocked to
  their means (expectation mode).
- **RNG** — `Math.random` is a deterministic mulberry32 (reset per save). Glyph
  generation already uses the seeded xorshift in the save; glyph *selection*
  inside a replay window is a deferred concern (rare in short horizons).
- **UA** — a normal Chrome user-agent so `browserCheck()` passes and `init()`
  runs.

## Status

Implemented; **not yet run end-to-end here** (requires the game served +
`playwright install`). Syntax-checked. First run should be verified against the
one save in `saves/captures/` (`01_pre_big_crunch.txt`, an early-game state where
the replicanti/glyph paths are inert). `saves/fixtures/` is git-ignored.
