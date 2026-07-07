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
   npm install                 # pulls Playwright (deps live here)
   npx playwright install chromium
   npm run generate            # reads ../saves/captures, writes ../saves/fixtures
   ```

   The dependencies must be installed here in `oracle/`, but the `generate`
   script can be launched from either directory: here, or from the crate root
   `crates/ad-fidelity` via its launcher `package.json` (`npm run generate`,
   symmetric with `cargo run -p ad-fidelity`). Node resolves Playwright from
   `oracle/node_modules` either way. Paths below are written relative to `oracle/`.

Output: one `fixtures/<save>.json` per input, containing the input save and the
expected savefile after each horizon:

```json
{
  "meta": { "sourceSave": "01_pre_big_crunch.txt", "tickMs": 50, "horizons": [1,10,100,1000], ... },
  "input": "<savefile>",
  "expected": { "1": "<savefile>", "10": "...", "100": "...", "1000": "..." }
}
```

### One save, and dense traces (`--save` / `--trace`)

Pass CLI flags **after `--`** so npm forwards them to the script (bare `--save`
would be eaten by npm itself):

```bash
npm run generate -- --save 1               # just capture id 1 -> ../saves/fixtures
npm run generate -- --save 1 --trace t.json  # dense trace -> ../saves/traces/t.json
```

- `--save <id>` restricts the run to one capture. `<id>` is resolved by the
  shared id convention (see `src/resolve.rs`): a path, `../saves/captures/<id>`,
  or the glob `0*<id>-*.txt` (so `--save 1` matches `00001-…txt`; ambiguous or
  missing ids error). Without `--trace`, this just rebuilds that one fixture.
- `--trace <name>` writes a single **dense** fixture — the same schema, but with
  `expected` carrying *every* tick `1..1000` — to `../saves/traces/<name>`. It
  requires `--save`. This is the input to `ad-fidelity trace <name>`, which scans
  it for the first tick that diverges from Rust. The two sides share
  `saves/traces/`, so `--trace t.json` here pairs with `ad-fidelity trace t.json`.

## Configuration (env vars)

- `GAME_URL` — where the game is served (default `http://localhost:8080`).
- `SAVES_DIR` — input saves (default `../saves/captures`).
- `OUT_DIR` — fixture output (default `../saves/fixtures`, git-ignored).
- `TRACES_DIR` — dense-trace output (default `../saves/traces`, git-ignored).
- `TICK_MS` — tick granularity, default 50 (design §10; a parameter).
- `HORIZONS` — comma list, default `1,10,100,1000` (non-trace runs).
- `TRACE_HORIZON` — max tick for `--trace` runs, default `1000`.

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
