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
src/persistence.rs          # SaveManager: on-disk saves, slots, backups (§12)
tauri.conf.json             # frontendDist = ./frontend/dist (no devUrl)
frontend/
  index.html                # <body class="t-normal s-base--dark"> + vendored <link>s
  public/stylesheets/       # VENDORED VERBATIM from ../../../antimatter-dimensions
  src/
    main.js                 # createApp + Pinia + app-shell.css
    app-shell.css           # minimal layout shell ONLY (ours, not vendored)
    config/tabs.js          # tab/subtab structure -> page components
    stores/game.js          # Pinia: Rust snapshot + action dispatchers
    stores/ui.js            # Pinia: navigation + open modal + dev speed +
                            #   h2pEmphasisShown (suppresses the How-To-Play glow)
    data/                   # frontend display data:
                            #   achievements.js (id/name/description/reward —
                            #     strings live frontend-side by design; the
                            #     engine owns only unlock state + effects),
                            #   credits.js, shortcuts.js (Hotkey List content)
    util/                   # small helpers: dimensionText, responsive,
                            #   shortcuts.js (keyboard handler),
                            #   tutorial.js (step ids + hasTutorial/emphasizeH2P)
    components/
      Sidebar.vue, SidebarCurrency.vue, GameHeader.vue, InfoButtons.vue
      DimensionRow.vue, TickspeedRow.vue, DimBoostRow.vue, GalaxyRow.vue,
      ProgressBar.vue        # shared building-block components
      Modal.vue, HotkeysModal.vue, CreditsDisplay.vue   # popups
      ImportSaveModal.vue, HardResetModal.vue            # save/load modals
      LoadGameModal.vue, BackupWindowModal.vue            #   (wired to engine)
      ConfirmModal.vue, ModalConfirmationCheck.vue       # prestige-confirm shell
      DimensionBoostConfirmModal.vue, AntimatterGalaxyConfirmModal.vue,
      SacrificeConfirmModal.vue, BigCrunchConfirmModal.vue  #   confirm bodies
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
  navigation state, the open-modal id, and dev controls (speed multiplier,
  offline mode, absolute pause). Components read snapshot fields and call store
  actions — they never compute game logic.
- **Offline mode + absolute pause** (dev controls under the speed row, in `ui`):
  while *offline mode* is on the loop stops ticking the engine and instead
  accumulates speed-scaled game-time (`accumulatedGameMs`, shown as a live
  readout); switching it off replays that interval via the `simulate_offline`
  command at the player's `offline_ticks` resolution and, above 10 s, opens the
  `OfflineSummaryModal` catch-up (before→after, original `AwayProgressModal`
  formatting). *Absolute pause* freezes both live ticks and offline accumulation.
  See `design-docs/2026-06-30-offline-progress.md`.

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
  (button → modal), **Gameplay → Hotkeys** (enable/disable toggle) and
  **Gameplay → Offline ticks** (slider). The Classic-UI toggle is intentionally
  dropped (Modern UI only); themes will be a reduced set. Full plan + per-option
  checklist: `design-docs/2026-06-27-options-tabs.md`.
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
- **Offline ticks** (`OptionsGameplayTab.vue`, Gameplay row 2 middle): the offline
  replay-resolution slider. Values follow the original's per-decade spacing
  `(1 + v%9) × 10^floor(v/9)` over slider indices 36..=63 → 10K…10M (default
  100K), a deliberately wider range than the original's 500…1M (the faster engine
  affords it). Calls `set_offline_ticks`; consumed by the Offline-mode replay.
- **Commands:** `set_hotkeys(enabled)`, `set_update_rate(rate)` (engine clamps
  to the 33–200 ms slider range), `set_notation(name)` (ignores names outside
  the known set), `set_notation_digits(comma, notation)` (clamps to 3–15, keeps
  notation `>=` comma), `set_offline_ticks(ticks)` (accepts any positive value —
  the slider range diverges from the original, so imported saves are not clamped).
  Mirrored by `stores/game.js` `setHotkeys` / `setUpdateRate` / `setNotation` /
  `setNotationDigits` / `setOfflineTicks`.
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
  `import_save*`/`hard_reset` also persist the root to disk immediately.
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

### On-disk persistence, save slots & backups (§12)

- **`persistence.rs` / `SaveManager`** (a managed `Mutex<SaveManager>`) owns all
  filesystem + wall-clock work `ad-core` deliberately avoids: the app-data-dir
  layout, the 3 save slots, and 8 automatic backup slots per save slot. Layout:
  `{app_data_dir}/saves.dat` (encoded `{ current, saves }` root) and
  `backups/{slot}/{1..8}.dat` (encoded single players). Writes are atomic (temp
  file + rename); backup ages come from file mtime (no separate `times.dat`).
- **Startup** (`.setup` in `main.rs`): resolves `app.path().app_data_dir()`,
  loads the root into the live `Mutex<GameState>`, and fires the longest
  applicable **offline** backup from the load gap. A missing/corrupt save starts
  fresh.
- **Cadence lives in the frontend** (`App.vue` rAF loop, `maybePersist`):
  **autosave** every `options.autosave_interval` ms and **online** backups
  (slots 1–4, wall-clock intervals) call `save_game` / `trigger_backup`. The
  **reserve** slot (8) is written by the backend before any backup load.
- **Commands:** `save_game`, `switch_save_slot(index) -> GameView`,
  `get_save_slots -> [{id,exists,antimatter,is_current}]`, `trigger_backup(slot)`,
  `get_backups -> [{id,exists,antimatter,last_backup_ms}]` (absolute mtime so the
  modal's "Last saved … ago" ticks in real time off the store clock),
  `load_backup(slot) ->
  GameView`, `export_backups_to_file` / `import_backups_from_file` (§2.4 bundle,
  native dialogs), `set_autosave_interval` / `set_show_time_since_save`. Mirrored
  by the like-named `stores/game.js` actions.
- **Engine-owned Saving options.** `autosave_interval`, `show_time_since_save`,
  and `save_file_name` live in `ad-core`'s `Options` (round-tripped through the
  save), surfaced in the snapshot's `options`. The Saving tab's autosave slider,
  "Display time since save" toggle, and "Save file name" input read/write them
  (`set_save_file_name` sanitizes to alphanumerics/space/hyphen, ≤16 chars).
  `save_file_name` is stored **per save slot** (it's a `player.options` field), so
  each slot carries its own — the "Choose save" modal shows it per slot
  (`get_save_slots` returns it), and it's the default filename for export-to-file.
- **SaveTimer.vue** — the bottom-left "Time since last save: HH:MM:SS" overlay,
  replicating the original's `SaveTimer.vue` (fixed `o-save-timer` at
  `bottom:0;left:0`, click to save, gated by `show_time_since_save`). It formats
  with `util/format.js` `timeDisplayShort` (the original's `TimeSpan.toStringShort`)
  and reads the store's `msSinceSave` (`nowMs - lastSaveTime`), where `nowMs` is
  refreshed every frame by `App.vue`'s loop so it advances even while
  paused/offline.
- **LoadGameModal / BackupWindowModal** fetch summaries on open (`getSaveSlots` /
  `getBackups`) and Load via `switchSaveSlot` / `loadBackup`. The backup modal's
  Export/Import-as-file use native dialogs (the last `<input type=file>` /
  `.c-file-import` WebKit hack is gone).
- **`Ctrl/Cmd+S` saves** (`util/shortcuts.js`, original `mod+s`): handled before
  the Ctrl/Cmd guard and independent of the Hotkeys option (a `bind`, not
  `bindHotkey`), and calls `preventDefault` to suppress the browser Save dialog.
- **Not yet wired:** real offline *progress* on startup (only the offline
  *backup* fires; the game doesn't replay the gap — the Offline-mode dev control
  is still the only replay path), so the Backup modal's "Load with offline
  progress disabled" toggle is currently inert.

## Keyboard shortcuts & popups

- `util/shortcuts.js` (`handleShortcut`, wired to `App.vue`'s `window` keydown)
  maps keys to game/ui actions, mirroring the original `core/hotkeys.js` for the
  implemented mechanics: `1`-`8` buy-until-10 / `Shift`+`1`-`8` buy-1, `T` /
  `Shift`+`T` tickspeed, `S`/`D`/`G` sacrifice/boost/galaxy, `C` Big Crunch, `M`
  max-all, arrows move tab (Up/Down) / subtab (Left/Right), `H` how-to-play, `?`
  Hotkey List. `S`/`D`/`G`/`C` route through the confirmation gate (the
  `request*` store actions), so they pop the explanatory modal just like the
  on-screen buttons. Letters/digits are matched via `e.code` (robust to Shift),
  `?` by character; Ctrl/Cmd/Alt combos and typing in inputs are ignored.
- Popups are centralised in `stores/ui.js` `openModal`
  (`help`/`info`/`credits`/`hotkeys`, the prestige-confirm ids
  `dimboostConfirm`/`galaxyConfirm`/`sacrificeConfirm`/`bigCrunchConfirm`,
  `null` = none) with `showModal` /
  `closeModal` / `toggleModal`. `InfoButtons.vue`'s on-screen buttons and the
  `?`/`H` keys drive the same state, so only one modal is open at a time.
  `Modal.vue` is the shared wrapper (overlay, close button, pinned title; the
  `fitContent` prop sizes it to content, e.g. the Hotkey List's two columns; the
  `centered` prop gives confirmation/choice modals a stable-width (`min-width:
  50rem`) centred column matching the original's `ModalWrapperChoice`, so
  width-capped content stays centred and they need no per-modal `text-align`
  rules — see *Confirmation modals* below).
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
- The button and the `C` key both call `game.requestBigCrunch()`. Mirroring the
  original `manualBigCrunchResetRequest`, the confirm modal opens only when the
  `bigCrunch` confirmation is on **and** it is the first infinity (`||
  player.break` once Break Infinity lands) — so pre-break the *first* crunch pops
  `BigCrunchConfirmModal.vue` (the first-infinity explanation, no disable
  checkbox) and every later crunch goes through directly. Confirm invokes the
  `big_crunch` command → `GameState::big_crunch`, which resets all pre-Infinity
  progress **and awards Infinity Points + an Infinity** (`gained_infinity_points`
  / `gained_infinities`, both 1 pre-break; IP/infinities persist). The next
  snapshot clears `can_big_crunch`, so the normal view returns.
- On the **first** crunch the store's `bigCrunch` action navigates to the new
  **Infinity** tab (mirrors `Tab.infinity.upgrades.show()`); the tab is
  conditional on `snapshot.infinity_unlocked` in `config/tabs.js` and shows the
  `InfinityUpgradesTab.vue` — the Infinity-Points header plus the **4×4 Infinity
  Upgrades grid** (Feature 2.2). `GameView` surfaces `infinity_points`; the
  post-break "IP/infinities gained" modal + disable checkbox and the Statistics
  `infinities` display come later. See
  `design-docs/2026-07-02-infinity-points-and-records.md` and
  `design-docs/2026-07-03-infinity-upgrades.md`.
- **Infinity Upgrades grid** (`InfinityUpgradesTab.vue` + `data/infinityUpgrades.js`):
  the vendored `o-infinity-upgrade-btn` tiles in four column chains, with the
  per-column lit-band background. Layout + descriptions live frontend-side
  (`data/infinityUpgrades.js`); the engine owns owned-state / affordability / cost
  / effect value, shipped per tile in `GameView.infinity_upgrades[]` (`{ id,
  is_bought, can_be_bought, cost, effect }`, grid/column-major order). Clicking a
  buyable tile calls the `buy_infinity_upgrade(id)` command (store `buyInfinityUpgrade`).
  The bottom row (`ipMult`/`ipOffline`, Achievement 41) is not built yet.

## Progressive reveal, tutorial highlights & confirmations

Three early-game presentation features, all engine-driven; the frontend only
renders the result. See `design-docs/2026-06-30-ui-reveal-and-tutorial.md`.

- **Progressive reveal.** The engine derives per-dimension `available_for_purchase`
  (band + "own the tier below") and `shown` (reveal/lookahead) flags, shipped in
  each `GameView.dimensions[]` entry. `DimensionRow.vue` uses `v-show="dim.shown"`
  and dims not-yet-purchasable rows via `c-dim-row--not-reached`;
  `TickspeedRow.vue` hides itself with `visibility:hidden` (reserving space)
  until `tickspeed_unlocked`; the sacrifice button's visibility follows
  `sacrifice_unlocked` (= achievement 18, *not* the boost count, which gates only
  `can_sacrifice`).
- **Tutorial highlights.** The snapshot carries raw `tutorial_state` /
  `tutorial_active`; `util/tutorial.js` exposes the step ids and
  `hasTutorial(snapshot, step)` / `emphasizeH2P(snapshot)`. The DIM1/DIM2/
  TICKSPEED/DIMBOOST/GALAXY targets add the vendored `tutorial--glow` (when the
  action is affordable) and a `fa-circle-exclamation l-notification-icon` `!`.
  `emphasizeH2P` (the pulsing How-To-Play `?`) is currently suppressed by the
  `ui.h2pEmphasisShown` flag (always true for now) because it would overlay the
  dev speed controls; when those become a toggleable option, drive the flag from
  their visibility.
- **Confirmation modals.** `ConfirmModal.vue` is the shared shell (built on
  `Modal.vue` with the `centered` variant) + `ModalConfirmationCheck.vue` (the
  "Don't show this message again" checkbox, which flips the engine flag via
  `set_confirmation`). The four bodies map to `ui.openModal` ids
  `dimboostConfirm` / `galaxyConfirm` / `sacrificeConfirm` / `bigCrunchConfirm`.
  Click handlers and the `S`/`D`/`G`/`C` hotkeys route through the store's
  `requestDimBoost` / `requestGalaxy` / `requestSacrifice` / `requestBigCrunch`,
  which no-op when the action isn't possible and otherwise either open the modal
  (when the matching `confirmations.*` flag is on) or perform the action
  directly — mirroring the original's `manualRequest*` / `sacrificeBtnClick`.
  The toggles live in the engine's `Options.confirmations`, surfaced as
  `GameView.options.confirmations` and round-tripped through the save.

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
- Achievements: `data/achievements.js` holds display strings (name/description/
  reward) only; the engine owns unlock state and effects. The tab renders real
  tiles (sprite + grey/green) from the snapshot `unlocked_achievements` list, and
  the `game` store fires an unlock toast by diffing that list between snapshots
  (seeded silently on load/import/reset). Rows 1–2 are implemented; later rows
  show as locked placeholders. See `design-docs/2026-06-30-achievements.md`.
- Responsive dimension rows use the "narrow" stacked layout unconditionally
  below 1573px (matches the original at the default window size).
- Big Crunch awards Infinity Points + an Infinity and opens the Infinity tab on
  the first crunch (Feature 2.1); the Infinity Upgrades grid is built (Feature
  2.2). Still shows the first-crunch (non-"small") screen unconditionally; the
  post-`break` header button, the <60 s "small crunch" flow, and the crunch
  animation come later. Break Infinity (Feature 2.3) — including the `ipMult`/
  `ipOffline` bottom-row upgrades and the post-break crunch modal — is next.
- Save/load: fully wired — clipboard/file export-import, hard reset, on-disk
  persistence, 3 save slots, autosave, 8 backup slots/slot (+ bundle
  export/import), "time since last save", and `Ctrl/Cmd+S`. The one gap is real
  offline *progress* on startup (only the offline *backup* fires). See
  `design-docs/2026-06-28-save-load-analysis.md` §9/§12.
