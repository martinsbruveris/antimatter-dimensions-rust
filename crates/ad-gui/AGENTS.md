# ad-gui — Primary frontend (Tauri + Vue 3)

The playable frontend. A Tauri **Rust backend** exposes the `ad-core` engine to
a **Vue 3 + Vite + Pinia** web frontend. The Rust engine is authoritative; the
frontend renders a per-tick snapshot and dispatches actions over Tauri IPC.

Background and rationale: `design-docs/2026-06-25-frontend-architecture.md` and
`design-docs/2026-06-25-number-formatting.md`. (Earlier prototypes — an
egui frontend and a vanilla-JS Tauri frontend — have been removed.)

## Run

```bash
npm --prefix frontend install     # once
npm --prefix frontend run build   # build Vue app -> frontend/dist
cargo run -p ad-gui               # Tauri serves frontend/dist (dev)
cargo tauri build                 # release build (.app/.dmg with icon)
```

`cargo run` is dev mode and serves `frontend/dist` from disk, so after a
frontend change just re-run `npm run build` (no Rust rebuild needed). There is
no `devUrl`/Vite-dev-server wired up — a plain `cargo run` with `devUrl` set
would show a blank window. `cargo tauri build` (release) bundles the frontend
via `beforeBuildCommand`. Run from `crates/ad-gui/` (requires
`cargo install tauri-cli`).

## Layout

```
src/main.rs                 # Tauri commands + GameView snapshot (build_game_view)
tauri.conf.json             # frontendDist = ./frontend/dist (no devUrl)
frontend/
  index.html                # <body class="t-normal s-base--dark"> + vendored <link>s
  public/stylesheets/       # VENDORED VERBATIM from ../../../antimatter-dimensions
  src/
    main.js                 # createApp + Pinia + app-shell.css
    app-shell.css           # minimal layout shell ONLY (ours, not vendored)
    config/tabs.js          # tab/subtab structure -> page components
    stores/game.js          # Pinia: Rust snapshot + action dispatchers
    stores/ui.js            # Pinia: navigation (current tab/subtab)
    data/achievements.js    # TEMP frontend data (moves into ad-core later)
    util/                   # small helpers (dimensionText, responsive)
    components/
      Sidebar.vue, SidebarCurrency.vue, GameHeader.vue
      DimensionRow.vue, TickspeedRow.vue, DimBoostRow.vue, GalaxyRow.vue,
      ProgressBar.vue        # shared building-block components
      tabs/                  # one component per page (subtab):
        AntimatterDimensionsTab.vue, NormalAchievementsTab.vue
```

## How it works

- **Backend** (`src/main.rs`): owns `Mutex<GameState>`. `tick_and_get_state`
  ticks the engine and returns a serialized `GameView` snapshot each frame.
  Other commands (`buy_dimension`, `sacrifice`, …) mutate state. Number
  formatting is currently done here (`format_decimal`) — see the formatting doc.
- **Game loop**: `App.vue` runs a `requestAnimationFrame` loop calling
  `game.tick(dt * speedMultiplier)`; the `game` store stores the latest
  snapshot. The speed multiplier (1x/10x/60x) is a dev-only UI feature in
  the `ui` store — it is not part of the game engine.
- **Stores**: `game` mirrors the Rust snapshot + dispatches actions; `ui` holds
  navigation state and dev controls (speed multiplier). Components read
  snapshot fields and call store actions — they never compute game logic.

## Multi-page navigation

- `config/tabs.js` is the single source of truth: each tab has subtabs with a
  `name`, `symbol`, and a Vue `component` (`null` = not built yet → placeholder).
- `stores/ui.js` tracks the open tab/subtab and exposes `currentComponent`.
- `App.vue` renders persistent chrome (sidebar, `GameHeader`, the green
  `information-header` separator) + `<component :is="ui.currentComponent">`.
- `Sidebar.vue` is driven by `TABS`, highlights the active tab/subtab from the
  `ui` store, and navigates via `setTab` / `setSubtab`.

**To add a page:** create `components/tabs/XTab.vue`, then point a subtab's
`component:` at it in `config/tabs.js`. Nothing else to wire.

## Conventions

- **Vendored CSS, verbatim.** All game-component styling comes from the
  original stylesheets copied unchanged into `public/stylesheets/`. Do not
  hand-transcribe CSS (that caused colour/size bugs in an earlier prototype).
  Select the Modern default theme via the `t-normal s-base--dark` body classes.
- **Scoped styles.** Some Modern classes live only in the original components'
  `<style scoped>` (e.g. `.l-tickspeed-container`, `.c-dim-row__large`,
  `.l-modern-buy-ad-text`, sidebar active states). Replicate those in the
  corresponding `ad-gui` component's scoped block; everything else comes from
  the global vendored CSS.
- **`app-shell.css`** holds layout-shell rules only (the centered/scrollable
  container beside the sidebar). Keep game styling out of it.
- **Reuse original markup**: mirror the original Modern components' `<template>`
  and class names; rewrite only the `<script>` to read the snapshot instead of
  the JS engine globals. Original source: `../../../antimatter-dimensions/src`.
- **Tooltips.** Use the vendored `c-tooltip-content`, `c-tooltip-arrow`, and
  `c-tooltip--left` (or `--top`/`--right`/`--bottom`) classes for tooltips.
  Place them as siblings of the trigger element inside a `position: relative`
  wrapper and toggle visibility on `:hover`. See `DimensionRow.vue` for an
  example.
- **External links.** A plain `<a target="_blank">` does not reliably open in
  the system browser inside the Tauri webview. Open external URLs via the
  opener plugin instead: `openUrl(url)` from `@tauri-apps/plugin-opener`
  (registered in `main.rs` as `tauri_plugin_opener::init()`, permission
  `opener:default` in `capabilities/default.json`). Fall back to `window.open`
  for plain-browser dev mode. See `InfoButtons.vue` for an example.

## Known follow-ups

- Formatting → a shared `ad-format` crate (PyO3 + WASM); snapshot then sends raw
  numbers (see `design-docs/2026-06-25-number-formatting.md`).
- Achievements live in `data/achievements.js` for display only; unlock state and
  real tiles come once `ad-core` owns achievements.
- Responsive dimension rows use the "narrow" stacked layout unconditionally
  below 1573px (matches the original at the default window size).
