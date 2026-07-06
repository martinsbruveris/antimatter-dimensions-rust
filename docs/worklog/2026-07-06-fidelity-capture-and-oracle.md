---
date: 2026-07-06
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
---

# Save-replay fidelity harness — capture rig + Playwright oracle

## Summary
Built the first two stages of the save-replay fidelity harness from the design
doc: the in-browser **capture rig** (userscript + local save server) and the
**Node oracle** (Playwright) that deterministically ticks real saves in headless
Chromium and writes reference fixtures. Both live in `crates/ad-fidelity/`.

## What shipped
- `crates/ad-fidelity/capture/save-server.js` — tiny CORS-enabled HTTP server
  that receives POSTed saves and writes them sequenced
  (`NNNNN-HHH-MM-SS-<tag>.txt`, where `HHH-MM-SS` is the game time elapsed at
  capture) plus an `index.jsonl` of metadata; sequence continues across restarts.
- `crates/ad-fidelity/capture/userscript.js` — Tampermonkey/Violentmonkey script
  injected into the original game: a bottom-right panel with speed buttons
  (1×/5×/25×/100×/1000×) and time-based capture. Only **time-based** capture is
  wired; event-driven is deferred to phase 2 per the design.
- `crates/ad-fidelity/oracle/generate-replay-fixtures.js` — Playwright oracle:
  boots the real game headless, applies the determinism controls, ticks each
  save in `saves/` to horizons 1/10/100/1000 at 50 ms, exports the resulting
  save at each horizon, and writes `oracle/fixtures/<save>.json`
  (`{ meta, input, expected }`).
- `package.json` + `README.md` + `.gitignore` for both subdirs; updated the
  crate `README.md` with the new layout and a "Save-replay harness" section.

## Decisions & why
- **Speed control replaces the loop, not `window.gameLoop`.** The game's
  `GameIntervals.gameLoop` calls a *module-local* `gameLoop` (`intervals.js:56`),
  so wrapping `window.gameLoop` would not intercept it. Instead the userscript
  stops that interval and drives `window.gameLoop(updateRate)` m times per real
  tick — more normal-sized ticks, not one giant tick, keeping granularity
  faithful. (`window.gameLoop` exists via `merge-globals.js`.)
- **Game-time capture cadence**, polled in real time against
  `records.totalTimePlayed`, so coverage stays even regardless of the chosen
  speed.
- **Offline disabled on import** via `GameStorage.offlineEnabled = false`
  (`storage.js:495`) so loading a save doesn't trigger offline catch-up before
  the controlled ticks. `GameIntervals.stop()` again after import (import calls
  `postLoadStuff` → `GameIntervals.restart`).
- **Deterministic clock + PRNG** injected via `addInitScript` (before page
  scripts), reset per save; replicanti samplers mocked to their means after load
  (design §7). Ticks pass an explicit `diff = 50`, so `realDiff = passDiff`
  (`game.js:438`) and the hibernation/`simulateTime` branch (needs
  `passDiff === undefined`) never fires.
- **Normal Chrome user-agent** on the Playwright context: `browserCheck()` is
  just `supportedBrowsers.test(navigator.userAgent)` (`game.js:1081`) and
  `init()` only runs when it passes (`main.js:8`); headless Chrome's default UA
  would fail it and skip init entirely.

## Deviations from the design doc
- None material. The design already recorded these decisions (time-based first,
  Playwright, 50 ms, autobuyers run during replay, offline disabled). The
  implementation matches; the doc's status is `Accepted`.

## Surprises & gotchas
- `GameStorage.export()` shows a clipboard toast; the actual string builder is
  `GameStorage.exportModifiedSave()` — used directly in both the userscript and
  oracle.
- The one available save (`saves/01_pre_big_crunch.txt`) is early-game, so the
  replicanti and glyph RNG paths are inert for it — good for a first smoke run,
  but it does not exercise the sampler mock or glyph comparison.

## Follow-ups
- Run the oracle end-to-end against `saves/01_pre_big_crunch.txt` once the game
  is served + `playwright install chromium` is done; sanity-check the fixtures.
- Build the Rust replay/comparison harness (the third stage): decode input →
  tick N → diff `PlayerDTO` against the fixture per the §5 allowlist, with the
  §6 round-trip identity guard.
- Generate the curated save set (needs a real capture run).
- Empirically fix the tolerance constants (§10).

## Tests
- `node --check` passes on all three scripts.
- Save server tested end-to-end (local, no game): GET status, POST valid
  (sequenced files written), sequence increments, POST-without-`save` → 400,
  `OPTIONS` preflight → 204, `index.jsonl` metadata correct.
- The Playwright oracle is **syntax-checked only** — not yet run end-to-end here
  (needs the game served and `playwright install`). Noted in `oracle/README.md`.
