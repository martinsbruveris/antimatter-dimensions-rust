---
status: Implemented
feature: statistics-tab
---

# Statistics tab

The Statistics tab: the read-only records pages. In the original it is tab id 2
with seven subtabs; this feature implements the three that our engine already
has the data for, and defers the rest.

Original source: `src/components/tabs/statistics/` (StatisticsTab.vue,
matter-scale.js), `src/components/tabs/challenge-records/`,
`src/components/tabs/past-prestige-runs/`, and the tab definition in
`src/core/secret-formula/tabs.js`.

---

## 1. Scope

Implemented subtabs (original hide-bit ids in brackets):

| Subtab | Original component | Gate (original `condition`) |
|---|---|---|
| **Statistics** [2,0] | `StatisticsTab` | always |
| **Challenge records** [2,1] | `ChallengeRecordsTab` | eternity unlocked ∨ any NC completed (Reality clause folds in) |
| **Past Prestige Runs** [2,2] | `PastPrestigeRunsTab` | infinity unlocked |

**Out of scope** (own features, noted for later):

- **Multiplier Breakdown** [2,3] — a ~3,600-line introspection feature
  (`secret-formula/multiplier-tab/` registry + `MultiplierBreakdownEntry`).
  It enumerates every multiplier source in the game and needs a dedicated
  engine-side "explain this multiplier" API. Separate design when tackled.
- **Glyph Set Records** [2,4] — needs `player.records.recentRealities`
  glyph-set tracking that the engine does not model yet.
- **Speedrun Milestones / Records** [2,5]/[2,6] — the speedrun system is not
  modelled at all.

Omitted lines within the Statistics page (systems not modelled; each would
render as a constant zero):

- News-ticker stats ("news messages seen", paperclips) — no news system.
- Secret Achievements count — secret achievements are not modelled.
- `fullGameCompletions` / `previousRunRealTime` block — endgame; hidden at 0
  in the original anyway.
- "You have been Doomed for X" — `records.realTimeDoomed` is not modelled;
  the Doomed *styling* (title/colors) is kept since `is_doomed` exists.
- The "View Content Summary" button — the catch-up modal is a separate
  feature.

## 2. Data inventory

### Already in the engine (no changes)

| Statistic | Engine source |
|---|---|
| Total antimatter made | `total_antimatter` |
| Real / game time played | `records.real_time_played_ms` / `total_time_played_ms` |
| Infinities (+banked) | `infinities`, `infinities_banked` |
| Fastest Infinity (+sentinel) | `records.best_infinity.time_ms` (`999999999999` = none) |
| Time in current Infinity (game/real) | `records.this_infinity.time_ms` / `real_time_ms` |
| Best IP/min this Eternity | `records.best_infinity.best_ip_min_eternity` |
| Eternities | `eternities` |
| Fastest Eternity | `records.best_eternity.time_ms` |
| Time in current Eternity | `records.this_eternity.time_ms` / `real_time_ms` |
| Best EP/min this Reality | `records.best_eternity.best_ep_min_reality` |
| Realities | `realities` |
| Fastest Reality (game/real) | `records.best_reality.time_ms` / `real_time_ms` |
| Time in current Reality | `records.this_reality.time_ms` / `real_time_ms` |
| Best RM/min | `records.best_reality.rm_min` |
| Best Glyph rarity | `strength_to_rarity(records.best_reality.glyph_strength)` |
| Projected banked Infinities | `floor(infinities × 0.05)` per Achievement 131 / TS191 (both `plusEffectsOf` terms; the same math `eternity_reset` applies) |
| NC best times | `nc_best_times_ms: [f64; 11]` (NC2–NC12) |
| IC best times | `ic_best_times_ms: [f64; 8]` |
| Last-10 prestige runs | `records.recent_infinities` / `recent_eternities` / `recent_realities` |
| Doomed styling | Pelle `is_doomed` |

### Engine additions

1. **`game_created_time_ms: f64` on `GameState`** — wall-clock save-creation
   timestamp (`records.gameCreatedTime`), currently dropped on decode and
   re-emitted as the template constant. Pure passthrough: decode → field →
   encode. `GameState::new()` sets `0.0` (the engine avoids wall clocks); the
   ad-gui backend stamps `Date.now()`-equivalent on new-game/hard-reset, where
   the wall clock already lives (`persistence.rs` conventions). The frontend
   hides the "save created" line when the value is `0`.
2. **`Options.stat_tab_resources: u8`** — the Past Prestige Runs resource-pair
   cycle (original `player.options.statTabResources`, values 0–3 =
   ABSOLUTE_GAIN / RATE / CURRENCY / PRESTIGE_COUNT). Engine-owned like every
   other option (round-trips through the save). New command
   `set_stat_tab_resources(value)` (clamped to 0–3).
3. **`shown_runs: { infinity, eternity, reality } : bool`** — the per-layer
   collapse toggles (original `player.shownRuns`, save-persisted). New command
   `toggle_shown_runs(layer)`.
4. **`banked_infinities_gain() -> Decimal` helper** — extract the 5%-per-source
   math from `eternity_reset` so the reset and the statistics view share one
   definition (JS `Achievement(131).effects.bankedInfinitiesGain` + `TS191`).

### Not added (accepted gaps)

- The recent-run rings drop the original tuples' trailing entries (challenge
  name; TT for eternities, glyph level / shards for realities). The engine
  parses only `[time, realTime, currency, count]`. The runs tables therefore
  omit the Challenge column (in the original it auto-hides when no run has
  challenge text) and the layer `extra` columns. Follow-up if fidelity work
  ever models the full tuples.
- `PlayerProgress.seenAlteredSpeed()` (shows the separate Real Time column
  in the runs tables once black holes / dilation touched game speed): the
  tables show the Real Time column once Reality is unlocked instead — a
  simplification consistent with the Statistics page, which gates its
  "(real time)" suffixes on Reality.

## 3. Snapshot (`GameView`)

One new `statistics: StatisticsView` built in `build_game_view` — the page is
read-only, so everything ships as display-ready values (`Num` for decimals,
`f64` ms for times):

```rust
struct StatisticsView {
    total_antimatter: Num,
    real_time_played_ms: f64,
    total_time_played_ms: f64,   // game time; shown once Reality is unlocked
    game_created_time_ms: f64,   // 0 = unknown (line hidden)
    // Infinity block (rendered when infinity_unlocked)
    infinities: Num,
    infinities_banked: Num,
    best_infinity_time_ms: f64,  // 999999999999 sentinel = "no fastest"
    this_infinity_time_ms: f64,
    this_infinity_real_time_ms: f64,
    best_ip_min: Num,
    // Eternity block (rendered when eternity_unlocked)
    eternities: Num,
    projected_banked: Num,
    banked_rate_per_min: Num,    // projected / clampMin(33, thisEternity time) × 60000
    best_eternity_time_ms: f64,
    this_eternity_time_ms: f64,
    this_eternity_real_time_ms: f64,
    best_ep_min: Num,
    // Reality block (rendered when reality unlocked)
    realities: u32,
    best_reality_time_ms: f64,
    best_reality_real_time_ms: f64,
    this_reality_time_ms: f64,
    this_reality_real_time_ms: f64,
    best_rm_min: Num,
    best_glyph_rarity: f64,
    is_doomed: bool,
    // Challenge records
    nc_best_times_ms: Vec<f64>,  // 11, NC2–NC12
    ic_best_times_ms: Vec<f64>,  // 8
    // Past Prestige Runs: [time_ms, real_time_ms, currency, count] × 10
    recent_infinities: Vec<RecentRunView>,
    recent_eternities: Vec<RecentRunView>,
    recent_realities: Vec<RecentRunView>,
    shown_runs: ShownRunsView,   // the 3 collapse flags
}
```

`banked_rate_per_min` follows the original update: `projectedBanked /
clampMin(33, thisEternity.time) × 60000`, using **real** time once Reality is
unlocked (the original recomputes with `realTime` inside the reality branch).
The existing top-level `infinity_unlocked` / `eternity_unlocked` /
`reality.unlocked` gates drive section visibility; `statTabResources` rides in
the existing `options` view.

Averages (the 11th "Average" table row) are computed frontend-side from the
shipped runs, like the original's `averageRun`.

## 4. Frontend

New files under `crates/ad-gui/frontend/src/`:

- `components/tabs/StatisticsTab.vue` — port of the original template minus
  the omitted lines (§1). Section gates: Infinity/Eternity/Reality blocks by
  the snapshot's unlock flags; "(real time)" suffixes and the game-time line
  by Reality. Scoped styles copied from the original component (the global
  `.c-stats-tab` chrome is already in the vendored `styles.css`).
- `components/tabs/ChallengeRecordsTab.vue` + `statistics/ChallengeRecordsList.vue`
  — the two best-times lists (`start=2` NC, `start=1` IC; IC list gated on
  `infinity_challenges_unlocked ∨ eternity_unlocked`). Sum line uses the
  `f64::MAX` sentinel check like the original's `Number.MAX_VALUE`.
- `components/tabs/PastPrestigeRunsTab.vue` + `statistics/PastPrestigeRunsContainer.vue`
  — the three layer tables (Infinity / Eternity / Reality, each gated on its
  unlock), the "Showing X" cycle button (`set_stat_tab_resources`), the
  per-layer collapse header (`toggle_shown_runs`), and the frontend
  `averageRun`. Layer config (names, currency labels) lives in the component
  like the original.
- `util/matterScale.js` — port of `matter-scale.js`. Display-only, so it stays
  frontend-side per the display-data convention. Works in **log space** on the
  snapshot's `Num {m, e}` (antimatter can far exceed f64): the `1e100000`
  writing-time branch uses `log10 = log10(m) + e`; the macro/micro scale
  lookups compare log10s and format the quotient via a `Num` rebuilt from
  `10^(frac)` / `floor` of the log10 difference. Same object tables verbatim.
- `util/format.js` — add `formatDateTime(ms)` (the original
  `Time.toDateTimeString`: `new Date(t).toString()` trimmed to the
  `"Mon dd yyyy hh:mm:ss"` slice).
- `config/tabs.js` — point subtab [2,0] at `StatisticsTab` and add the two new
  subtabs with `hideId` [2,1] / [2,2] and their conditions.

The matter-scale line recomputes at most once per second (the original's
jitter guard), keyed off the store clock.

## 5. Commands

| Command | Effect |
|---|---|
| `set_stat_tab_resources(value: u8)` | clamp 0–3, store in `Options` |
| `toggle_shown_runs(layer: String)` | flip `shown_runs.{infinity,eternity,reality}` |

Both mirrored as `stores/game.js` actions.

## 6. Save round-trip

- `records.gameCreatedTime` — new passthrough (decode + encode; encode
  currently emits the template constant).
- `options.statTabResources` — decode into `Options`, encode from it.
- `shownRuns` — decode the three modelled keys (`Infinity`, `Eternity`,
  `Reality`), encode them back; other keys keep template defaults.

None of these are in the fidelity allowlist today; adding the passthroughs
only improves round-trip completeness.

## 7. Testing

- Unit tests in `ad-core` for the new decode/encode passthroughs (round-trip
  a save carrying non-default `gameCreatedTime` / `statTabResources` /
  `shownRuns`) and for `banked_infinities_gain` (0 / ach131 / +TS191).
- `build_game_view` compiles the new view; manual verification of the three
  subtabs against the original game at matching save states (early game,
  post-Infinity, post-Eternity).

## 8. Plan

- [x] Engine: `game_created_time_ms` passthrough + `Options.stat_tab_resources`
      + `shown_runs` + `banked_infinities_gain()` refactor (+ tests)
- [x] Backend: `StatisticsView` in `build_game_view`, the two commands,
      new-game/hard-reset stamps `game_created_time_ms`
- [x] Frontend: `matterScale.js` + `formatDateTime`
- [x] Frontend: `StatisticsTab.vue`
- [x] Frontend: `ChallengeRecordsTab.vue` (+ list)
- [x] Frontend: `PastPrestigeRunsTab.vue` (+ container)
- [x] `config/tabs.js` wiring + conditions
- [x] Living docs (`crates/ad-gui/AGENTS.md`, ARCHITECTURE if touched) + worklog
