---
date: 2026-07-10
feature: statistics-tab
design_docs:
  - ../design/2026-07-10-statistics-tab.md
---

# Statistics tab — main page, Challenge records, Past Prestige Runs

## Summary
Implemented the Statistics tab in `ad-gui`: the main Statistics page, the
Challenge-records best-times lists, and the Past Prestige Runs tables — the
three subtabs the engine already had the data for. Design doc written first
(same day) and implemented in full.

## What shipped
- **Engine (`ad-core`)**: `records.game_created_time_ms` passthrough
  (decode/encode; previously dropped and re-emitted as the template constant),
  `Options.stat_tab_resources`, `GameState.shown_runs` (the `player.shownRuns`
  expand flags, capitalized keys in the save), and a shared
  `banked_infinities_gain()` extracted from `eternity_reset`.
- **Backend (`ad-gui`)**: `StatisticsView` in the snapshot
  (`build_statistics_view`) — records, projected banked + rate, NC/IC best
  times, the three recent-run rings, shown-runs flags; commands
  `set_stat_tab_resources` / `toggle_shown_runs`; `fresh_game()` stamps
  `game_created_time_ms` (the engine stays wall-clock-free).
- **Frontend**: `StatisticsTab.vue`, `ChallengeRecordsTab.vue` (+
  `ChallengeRecordsList`), `PastPrestigeRunsTab.vue` (+
  `PastPrestigeRunsContainer`), `util/matterScale.js` (log10-space port of
  `matter-scale.js`), `formatDateTime` in `util/format.js`, and the
  `config/tabs.js` wiring (hide-bits [2,0]–[2,2] with the original
  conditions).

## Decisions & why
- **Scope cut to three subtabs.** Multiplier Breakdown is a ~3,600-line
  introspection feature needing its own engine API; Glyph Set Records and the
  Speedrun pages need systems the engine doesn't model. All deferred
  explicitly in the design doc.
- **Matter scale stays frontend-side** (display-only data convention), working
  in log10 space on the raw `{ m, e }` snapshot numbers since antimatter far
  exceeds f64.
- **`statTabResources` / `shownRuns` are engine-owned** like every other
  option, so they round-trip through real saves.
- **Averages computed frontend-side** (like the original's `averageRun`), with
  a max-exponent-relative mantissa sum for the Decimal columns.

## Deviations from the design doc
- None of substance; implemented as designed. The `formatDecimalAmount`
  sub-1e9 branch routes through the WASM formatter for thousand separators
  (the original's `formatInt`) rather than a plain integer string.

## Surprises & gotchas
- The engine's recent-run rings only parse `[time, realTime, currency,
  count]` from the original's mixed tuples, so the runs tables omit the
  Challenge and per-layer extra columns (noted as an accepted gap; the
  original auto-hides the Challenge column when empty anyway).
- `f64::MAX` survives the Tauri JSON IPC exactly as `Number.MAX_VALUE`, so
  the original's sentinel comparisons port unchanged.
- macOS blocks scripted keystrokes/screen capture for the runtime app, so
  visual verification was limited to a clean 10 s boot (no panics, no log
  output); in-tab rendering should be eyeballed in a manual run.

## Follow-ups
- Multiplier Breakdown subtab (own design; needs an engine-side multiplier
  introspection API).
- Glyph Set Records + the full recent-run tuples (challenge name, TT/glyph
  extras) if fidelity work ever models them.
- The "View Content Summary" button (catch-up modal) on the Statistics page.

## Tests
- `cargo test -p ad-core --features serde` — 580 + 22 + 29 pass (new:
  statistics-fields round-trip, `banked_infinities_gain` stacking).
- `cargo test -p ad-gui` — 10 pass (new: `statistics_view_reflects_records`,
  including full-snapshot serialization).
- Fidelity suite unchanged at 1469/1476 (the encode additions are
  passthroughs).
- Frontend `npm run build` clean; app boots without panics.
