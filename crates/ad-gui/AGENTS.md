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
npm --prefix frontend run build   # build wasm + Vue app -> frontend/dist
cargo run -p ad-gui               # Tauri serves frontend/dist (dev)
cargo tauri build                 # release build (.app/.dmg with icon)
```

`npm run build` first runs `build:wasm` (`wasm-pack build ../../ad-format
--target web ... --features wasm`, output to `frontend/src/wasm/`), then the
Vite build. **`wasm-pack` must be installed** (`cargo install wasm-pack`); the
generated `src/wasm/` is regenerable and git-ignored like `dist/`.

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
      ImportSaveModal.vue, HardResetModal.vue            # save/load modals
      LoadGameModal.vue, BackupWindowModal.vue            #   (wired to engine)
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
  Other commands (`buy_dimension`, `sacrifice`, …) mutate state. Numbers ship
  **raw** in the snapshot as `Num { m, e }` (mantissa × 10^exponent); the
  webview formats them via the `ad-format` WASM module (Option C in the
  formatting doc), so no formatting crosses IPC.
- **Number formatting** (`frontend/src/util/format.js`): `formatDecimal(num,
  places, placesUnder1000)` calls the WASM `format` synchronously in-process,
  reading the active notation from `snapshot.options.notation`. The WASM module
  (`frontend/src/wasm/`, built by `npm run build:wasm`) is initialised once in
  `main.js` before the app mounts. `formatMultiplier` is the `×N` variant
  (1 decimal under 1000).
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

## Options tabs

- **Storage is engine-owned.** Player options live in `ad-core`'s `Options`
  struct (`GameState.options`), not in a frontend store — so a save written
  from a fresh game is valid and options round-trip unchanged through
  load → run → save. They are preserved across a Big Crunch. The snapshot
  exposes them as `GameView.options`; the frontend reads that and writes via
  dedicated commands (it never mutates options locally).
- **Scope so far.** The implemented options each sit in their original grid
  position with the rest of the grid as invisible placeholders
  (`l-options-grid__button--hidden`): **Visual → Update rate** (slider),
  **Visual → Notation** (dropdown), **Visual → Exponent Notation Options**
  (button → modal) and **Gameplay → Hotkeys** (enable/disable toggle). The
  Classic-UI toggle is intentionally dropped (Modern UI only); themes will be a
  reduced set. Full plan + per-option checklist:
  `design-docs/2026-06-27-options-tabs.md`.
- **Notation** (`SelectNotationDropdown.vue`, Visual row 2 middle): a simplified
  port of the original's ExpandingControlBox — a header button expanding an
  inline list of the four `ad-format` notations (Scientific, Engineering,
  Standard, Letters; default **Standard**). Selecting one calls `set_notation`;
  the next snapshot's `options.notation` re-renders every number via the WASM
  formatter. Only implemented notations are listed (no dead entries).
- **Exponent Notation Options** (`NotationModal.vue`, opened from the Visual-tab
  button via `ui.openModal === 'notation'`): two 3–15 sliders for the
  `notationDigits` comma / in-notation thresholds, with the original's verbatim
  text and a live sample preview. The thresholds set `ExponentDisplay { min:
  10^comma, max: 10^notation }` in the WASM formatter (`util/format.js` threads
  them through every `formatDecimal` call); the preview (`formatExponentSample`)
  uses the in-flight slider values so it updates while dragging. The engine keeps
  the notation threshold `>= comma` (original NotationModal invariant), mirrored
  locally in the modal.
- **Commands:** `set_hotkeys(enabled)`, `set_update_rate(rate)` (engine clamps
  to the 33–200 ms slider range), `set_notation(name)` (ignores names outside
  the known set), `set_notation_digits(comma, notation)` (clamps to 3–15, keeps
  notation `>=` comma). Mirrored by `stores/game.js` `setHotkeys` /
  `setUpdateRate` / `setNotation` / `setNotationDigits`.
- **Update rate** drives the game loop: `App.vue`'s rAF loop only ticks once
  `update_rate` ms of wall-clock time have elapsed, then processes the whole
  interval — matching the original's `interval(gameLoop, updateRate)` (larger
  = coarser, less frequent updates) rather than ticking every frame.
- **Hotkeys toggle** gates `util/shortcuts.js`: gameplay keys (digits, T, A, M,
  S, D, G, C) are skipped when disabled, while modal keys (`?`, `H`, `Esc`) and
  arrow navigation stay active — mirroring the original's `bindHotkey`
  (gated by `player.options.hotkeys`) vs `bind` (always active) split.
- **Reusable widgets** (`components/options/`): `PrimaryToggleButton.vue`
  (labelled on/off button) and `OptionsSlider.vue` — a minimal, visually
  faithful single-handle slider using the vendored `ad-slider-component.css`
  (newly added to `public/stylesheets/` + `index.html`), not the original's
  heavy `vue-slider-component` port.

## Save / Load

- **Engine codec** (`ad-core::save`): `encode_save(&GameState, now_ms) ->
  String` and `decode_save(&str) -> Result<GameState, SaveError>`. Pure,
  deterministic, no IO. The codec implements the original's `AAB` format
  (zlib + base64 + character-safe cleanup + magic markers). Saves are
  wire-compatible with the real game. See
  `design-docs/2026-06-28-save-load-analysis.md`.
- **Commands:** `export_save` (returns the save string for clipboard copy),
  `import_save(text)` (decodes + swaps `Mutex<GameState>` + returns
  `GameView`), `export_save_to_file(save_file_name)` (native Save As dialog
  via `tauri-plugin-dialog`, writes `.txt`), `import_save_from_file` (native
  Open dialog, reads + decodes + swaps state), `hard_reset` (replaces state
  with `GameState::new()`). Mirrored by `stores/game.js` `exportSave` /
  `importSave` / `exportSaveToFile` / `importSaveFromFile` / `hardReset`.
- **File dialogs** use `tauri-plugin-dialog` (`blocking_save_file` /
  `blocking_pick_file`), registered in `main.rs` and permitted via
  `dialog:default` in `capabilities/default.json`. This replaced the original's
  `<input type="file">` / `.c-file-import` CSS hack for the main save tab,
  avoiding the WebKit overflow issue.
- **ImportSaveModal** (opened by the "Import save" button): text input +
  Import/Cancel buttons. On Import, calls `importSave`, shows errors inline
  (red text), closes + toasts on success. Enter key submits.
- **HardResetModal** (opened by "RESET THE GAME"): the original's
  confirmation-phrase gate ("Shrek is love, Shrek is life") controls whether
  the HARD RESET button appears; clicking it calls `hardReset`, toasts, and
  closes the modal.
- **Not yet wired:** Save game button, Choose save / save slots, backup slots,
  autosave, on-disk persistence, "time since last save" display, the `S`
  keyboard shortcut (save).

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
- **File-import buttons need `overflow: hidden`.** The vendored `.c-file-import`
  hack (the Backup "Import from file" buttons) balloons an invisible `::before`
  (`font-size: 100rem; padding: 10rem 20rem`) so the whole button opens the file
  dialog. The **Tauri webview is WebKit**, which paints that overflow *outside*
  the button instead of clipping it (Chrome clips). Left unclipped it silently
  covers and steals clicks from nearby controls, and inside a scrollable modal it
  inflates the scroll height to far past the content. Always clip the file-import
  button's container with `overflow: hidden` in the component's `<style scoped>`.
  See `BackupWindowModal.vue`. The main "Import save from file" button on the
  Saving tab no longer uses this hack — it uses a native dialog via
  `tauri-plugin-dialog` instead. General rule: prefer testing webview-bound UI
  against **WebKit** (Playwright's `webkit`), not just Chrome — they differ on
  form-control rendering and flexbox `min-width: auto` overflow.

## Known follow-ups

- Formatting WASM done (snapshot sends raw `Num { m, e }`, webview formats via
  `ad-format`); **PyO3** exposure of `format` is the remaining part of Option C
  (see `design-docs/2026-06-25-number-formatting.md`).
- Notation options: only the four implemented notations are listed; the
  remaining ~18 notations are not ported yet (the `notationDigits` thresholds
  modal is done — see Options tabs above). `inf_threshold` is left at its default
  (never "Infinite") — fine pre-Infinity since antimatter caps before
  `NUMBER_MAX_VALUE`.
- Achievements live in `data/achievements.js` for display only; unlock state and
  real tiles come once `ad-core` owns achievements.
- Responsive dimension rows use the "narrow" stacked layout unconditionally
  below 1573px (matches the original at the default window size).
- Big Crunch resets all pre-Infinity progress but awards no Infinity Points
  yet, and shows the first-crunch (non-"small") screen unconditionally; IP and
  the post-`break` header button come next.
- Save/load: export/import (clipboard + file) and hard reset are wired; save
  slots, autosave, on-disk persistence, "time since last save", and the `S`
  keyboard shortcut remain. See `design-docs/2026-06-28-save-load-analysis.md`
  §9.
