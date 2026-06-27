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
    stores/ui.js            # Pinia: navigation + open modal + dev speed
    data/                   # TEMP frontend data (moves into ad-core later):
                            #   achievements.js, credits.js,
                            #   shortcuts.js (Hotkey List content)
    util/                   # small helpers: dimensionText, responsive,
                            #   shortcuts.js (keyboard handler)
    components/
      Sidebar.vue, SidebarCurrency.vue, GameHeader.vue, InfoButtons.vue
      DimensionRow.vue, TickspeedRow.vue, DimBoostRow.vue, GalaxyRow.vue,
      ProgressBar.vue        # shared building-block components
      Modal.vue, HotkeysModal.vue, CreditsDisplay.vue   # popups
      BigCrunchScreen.vue    # replaces the game view at the Big Crunch cap
      tabs/                  # one component per page (subtab):
        AntimatterDimensionsTab.vue, NormalAchievementsTab.vue,
        AutobuyersTab.vue
        autobuyers/          # AutobuyerBox (shared row/purchase box),
                             #   DimensionAutobuyerBox, TickspeedAutobuyerBox,
                             #   AutobuyerToggles
```

## How it works

- **Backend** (`src/main.rs`): owns `Mutex<GameState>`. `tick_and_get_state`
  ticks the engine and returns a serialized `GameView` snapshot each frame.
  Other commands (`buy_dimension`, `sacrifice`, …) mutate state. Number
  formatting is currently done here (`format_decimal`) — see the formatting doc.
- **Game loop**: `App.vue` runs a `requestAnimationFrame` loop calling
  `game.tick(dt, repeats)`; the `game` store stores the latest snapshot.
  `repeats` is the dev speed multiplier (1x/10x/100x/1000x, in the `ui`
  store): the engine runs N discrete ticks of the real frame `dt` **inside
  Rust** (`GameState::ticks`) — one IPC call and one snapshot per frame,
  rather than a single `dt × N` step. This preserves per-tick precision
  (e.g. autobuyers) and avoids per-tick IPC. The multiplier is dev-only, not
  part of the engine.
- **Stores**: `game` mirrors the Rust snapshot + dispatches actions; `ui` holds
  navigation state, the open-modal id, and dev controls (speed multiplier).
  Components read snapshot fields and call store actions — they never compute
  game logic.

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

**Conditional tabs.** A tab may carry an optional `condition(snapshot)` in
`config/tabs.js` that hides it until the game unlocks it. The `ui` store's
`visibleTabs` getter filters `TABS` through these conditions against the live
`game.snapshot`; `Sidebar.vue` and arrow-key `moveTab` both iterate
`visibleTabs`. The Automation tab uses this (`autobuyers.tab_unlocked`).

## Autobuyers tab

- Pre-Infinity only: the antimatter-dimension (8 tiers) and tickspeed
  autobuyers. The dimboost/galaxy/sacrifice/crunch autobuyers and interval
  upgrades are post-Infinity and not built yet.
- **Unlock model** (engine-owned, see `ad-core`): the Automation tab unlocks at
  all-time `total_antimatter >= 1e40`; each autobuyer's "slow version" is
  unlocked by clicking its purchase box once its antimatter requirement is met
  (AD tiers 1e40…1e110, tickspeed 1e140). Unlocking costs no antimatter. Both
  the tab and unlocked autobuyers persist through a Big Crunch.
- Intervals are fixed (the interval/mode-upgrade buttons show the disabled
  "Complete the challenge to …" state); AD autobuyers can toggle "Buys
  singles"/"Buys max", tickspeed is locked to single.
- **Snapshot:** `GameView.autobuyers` (`build_autobuyers_view`) carries
  `tab_unlocked`, `enabled`, and per-entry `{ name, is_bought, can_unlock,
  requirement, interval_seconds, is_active, mode, can_change_mode }`.
- **Commands:** `unlock_ad_autobuyer`, `toggle_ad_autobuyer`,
  `toggle_ad_autobuyer_mode`, `unlock_tickspeed_autobuyer`,
  `toggle_tickspeed_autobuyer`, `toggle_autobuyers` (global pause/resume, also
  the `A` hotkey), `set_all_autobuyers_active` (the "Enable/Disable all"
  button). Store actions mirror these in `stores/game.js`.

## Keyboard shortcuts & popups

- `util/shortcuts.js` (`handleShortcut`, wired to `App.vue`'s `window` keydown)
  maps keys to game/ui actions, mirroring the original `core/hotkeys.js` for the
  implemented mechanics: `1`-`8` buy-until-10 / `Shift`+`1`-`8` buy-1, `T` /
  `Shift`+`T` tickspeed, `S`/`D`/`G` sacrifice/boost/galaxy, `C` Big Crunch, `M`
  max-all, arrows move tab (Up/Down) / subtab (Left/Right), `H` how-to-play, `?`
  Hotkey List. Letters/digits are matched via `e.code` (robust to Shift), `?` by
  character; Ctrl/Cmd/Alt combos and typing in inputs are ignored.
- Popups are centralised in `stores/ui.js` `openModal`
  (`help`/`info`/`credits`/`hotkeys`, `null` = none) with `showModal` /
  `closeModal` / `toggleModal`. `InfoButtons.vue`'s on-screen buttons and the
  `?`/`H` keys drive the same state, so only one modal is open at a time.
  `Modal.vue` is the shared wrapper (overlay, close button, pinned title; the
  `fitContent` prop sizes it to content, e.g. the Hotkey List's two columns).
  The Hotkey List rows come from `data/shortcuts.js` (the original's
  default-visible bindings).
- **Toast notifications.** Transient top-right popups (the original's
  `GameUI.notify.*` / `core/notify.js`). `stores/ui.js` holds a `notifications`
  list and a `notify(text, type, duration)` action that drives the
  enter/leave animation flags and auto-removal (default 2s; click dismisses
  early); `NotificationContainer.vue` (mounted in `App.vue`) renders them with
  the vendored `o-notification` / `a-notification` CSS. `type` selects the
  colour (`info` = blue). The autobuyer toggle (`A`) fires an info toast
  ("Autobuyers resumed/paused"), matching the original — note the on-screen
  Pause/Resume button does **not** toast, only the hotkey does.

## Big Crunch

- When antimatter reaches `BIG_CRUNCH_THRESHOLD` (capped there in the engine),
  the snapshot's `can_big_crunch` is true and `App.vue` replaces the whole
  `tab-container` with `BigCrunchScreen.vue` — the "world has collapsed" message
  plus the vendored `.btn-big-crunch` button (mirrors ModernUi's `tab-container`
  being hidden while the crunch button shows).
- The button and the `C` key both call `game.bigCrunch()` → the `big_crunch`
  command → `GameState::big_crunch`, which resets all pre-Infinity progress. The
  next snapshot clears `can_big_crunch`, so the normal view returns. Infinity
  Points are **not** awarded yet (planned next).

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
- Big Crunch resets all pre-Infinity progress but awards no Infinity Points
  yet, and shows the first-crunch (non-"small") screen unconditionally; IP and
  the post-`break` header button come next.
