# Options Tabs — Visual & Gameplay (analysis + implementation plan)

Date: 2026-06-27

This document analyses the **Visual** and **Gameplay** options subtabs of the
original Antimatter Dimensions game and proposes how to port them to `ad-gui`.
The **Saving** subtab is explicitly out of scope here — it will be handled
together with the save/load system. A per-option tracking checklist is at the
end (§7) so implementation can proceed iteratively.

Original source consulted:
- `src/components/tabs/options-visual/OptionsVisualTab.vue`
- `src/components/tabs/options-gameplay/OptionsGameplayTab.vue`
- `src/components/modals/options/*` (Animation, Confirmation, Info Display,
  Away Progress, Notation, News, hidden-tabs)
- `src/core/player.js` (`player.options` schema), `src/core/options.js`
  (`GameOptions`), `src/core/notations.js`, `src/core/themes.js`,
  `src/core/secret-formula/confirmation-types.js`


## 1. How options work in the original

### 1.1 Storage

All options live under a single `player.options` object (player.js:767), which
is part of the save blob. There is **no separate options store** — options
round-trip through the save like any other player state. A handful of nested
groups exist: `news`, `notationDigits`, `animations`, `confirmations`,
`awayProgress`, `showHintText`, `automatorEvents`, `multiplierTab`. The full
default schema is reproduced in §6.

This matters for us: because the user wants to import **external saves**, the
options we expose must eventually map back onto these exact keys. See §3.1.

### 1.2 UI structure

Each subtab (`OptionsVisualTab`, `OptionsGameplayTab`) is a flat
`l-options-grid` of rows; each row holds up to three controls. Controls are one
of a small set of reusable widgets:

| Widget | Role | Used by |
|--------|------|---------|
| `OptionsButton` | a plain button that runs an `onclick` (usually opens a modal) | "Open … Options", "Modify Visible Tabs", UI toggle |
| `PrimaryToggleButton` | labelled on/off toggle bound via `v-model` | hotkeys, offline progress, header coloring, … |
| `ExpandingControlBox` | a button that expands an inline dropdown panel | Theme, Notation, Sidebar pickers |
| `SliderComponent` | a numeric slider | update rate, offline ticks, notation digits |
| `ModalOptionsToggleButton` | toggle styled for use inside a modal | every modal toggle |

Deeper / grouped settings are pushed into **modals** (Animation, Confirmation,
Info Display, Away Progress, News, Notation). The tabs themselves only hold the
most common toggles plus buttons that open those modals.

### 1.3 Read/write pattern

Every tab/modal component has an `update()` method (called each frame by the
original's global update loop) that copies `player.options.*` into local
`data`, and a `watch` block that writes local changes back to
`player.options.*`. A few options have side effects beyond the assignment,
funnelled through `GameOptions` (options.js):

- `toggleUI()` — flips `newUI`, re-applies the theme, re-zooms, saves.
- `toggleNews()` — flips `news.enabled`, saves.
- `refreshUpdateRate()` — restarts the game-loop interval so a new
  `updateRate` takes effect (the loop is `interval(gameLoop, updateRate)`,
  intervals.js:56).
- `refreshAutosaveInterval()` — restarts the autosave timer (Saving tab).

Theme application (`themes.js`) swaps body CSS classes; notation selection
(`notations.js`) sets `ui.notation` and the active `@antimatter-dimensions/
notations` strategy.

### 1.4 Unlock gating

Many options are hidden until a prestige layer is reached. Pre-Infinity (our
current milestone) only a subset is ever visible. Gating predicates we care
about:

- Confirmations: each type has `isUnlocked()`; pre-Infinity-visible ones are
  **Dimension Boost** (`galaxies>0 || dimensionBoosts>0`), **Antimatter
  Galaxy** (`galaxies>0`), **Sacrifice** (`Sacrifice.isVisible`), **Big
  Crunch** (`player.break || eternityUnlocked`).
- Animations: `bigCrunch` shown once Infinity is unlocked; everything else is
  later. (Note: the Big-Crunch *animation* itself is gated on infinity, but the
  first crunch is our milestone endpoint — see §4.5.)
- Info Display: "Show % gain", "Achievement IDs", "Achievement unlock state"
  are always available; the rest are gated.
- Sidebar picker and UI toggle: Modern UI only.


## 2. Complete option inventory

### 2.1 Visual tab

| # | Control | `player.options` key | Type / range | Notes |
|---|---------|----------------------|--------------|-------|
| V1 | UI: Modern/Classic | `newUI` | toggle | `GameOptions.toggleUI()` |
| V2 | Update rate | `updateRate` | slider 33–200 ms | restarts game loop |
| V3 | Open News Options | (modal) `news.*` | button → modal | News system not ported |
| V4 | Theme | `themeModern` / `themeClassic` | dropdown | many themes; secret themes gated |
| V5 | Notation | `notation` | dropdown (22) | drives `ad-format` |
| V6 | Exponent Notation Options | `notationDigits.comma`, `.notation` | modal, 2 sliders 3–15 | comma ≤ notation invariant |
| V7 | Animation Options | `animations.*` | modal | mostly prestige-gated |
| V8 | Info Display Options | `showHintText.*` | modal | partly gated |
| V9 | Away Progress Options | `awayProgress.*` | modal | tied to offline progress |
| V10 | Modify Visible Tabs | `hiddenTabBits`, `hiddenSubtabBits` | modal | already on todo.md |
| V11 | Relative prestige gain text coloring | `headerTextColored` | toggle | header gain text color |
| V12 | Sidebar (Modern UI) | `sidebarResourceID` | dropdown | Modern UI only |
| V13 | Hotkey List button | — | button → modal | already implemented |

### 2.2 Gameplay tab

| # | Control | `player.options` key | Type / range | Notes |
|---|---------|----------------------|--------------|-------|
| G1 | Confirmation Options | `confirmations.*` | modal | per-type unlock gating |
| G2 | Hotkeys | `hotkeys` | toggle | master enable for shortcuts |
| G3 | Switch tabs on some events | `automaticTabSwitching` | toggle | auto-navigate on events |
| G4 | Offline progress | `offlineProgress` | toggle | engine-relevant |
| G5 | Offline ticks | `offlineTicks` | slider → 500…1e6 | non-linear mapping |
| G6 | Run suspended time as offline | `hibernationCatchup` | toggle | engine-relevant |
| G7 | Automator Log Max | `automatorEvents.maxEntries` | slider 50–500 | post-Reality; out of scope |

The offline-ticks slider uses a custom mapping (OptionsGameplayTab.vue:87):
slider value `x ∈ [22,54]` → `(1 + x%9) · 10^floor(x/9)` ticks, i.e. 500, 600,
…, 900, 1000, 2000, …, 1e6.

### 2.3 Modal contents (for completeness)

- **Animation** (`animations`): `bigCrunch`, `eternity`, `dilation`,
  `tachyonParticles`, `reality`, `background`, `blobSnowflakes` (S11 slider),
  `blobHole`. Each gated by the matching prestige.
- **Confirmation** (`confirmations`): 21 boolean types, each shown only when
  `isUnlocked()`. Pre-Infinity: Dimension Boost, Antimatter Galaxy, Sacrifice,
  Big Crunch.
- **Info Display** (`showHintText`): `showPercentage`, `achievements`,
  `achievementUnlockStates`, `challenges`, `studies`, `glyphEffectDots`,
  `realityUpgrades`, `perks`, `alchemy`. First three ungated.
- **Away Progress** (`awayProgress`): ~23 booleans choosing which resources
  appear in the offline-progress summary. All ungated structurally but only
  show if the resource increased.
- **Notation** (`notationDigits`): `comma` and `notation` digit thresholds
  (sliders 3–15), with `comma ≤ notation` enforced; live sample preview.
- **News** (`news`): `enabled`, `repeatBuffer` (0–80), `AIChance` (0–1),
  `speed` (0.5–2), `includeAnimated`.


## 3. Implementation considerations

### 3.1 Where do options live?

The architecture principle is "Rust authoritative; frontend reads `GameState`
for display and never computes game logic." Options are a grey area: most are
**presentation/UX config**, not game logic, but in the original they are part of
the persisted player state, and external-save import requires that mapping.

Two engine-relevant options exist in the Visual/Gameplay set — `offlineProgress`
and `hibernationCatchup` (and `offlineTicks`) — but offline progress is **not
implemented yet** (no offline simulation pre-milestone), so nothing in the
engine consumes them today.

**Recommendation (pragmatic, milestone-first):** introduce a dedicated
frontend `options` Pinia store, keyed to mirror `player.options` exactly, and
persist it to `localStorage` for now. Rationale:

1. Every Visual/Gameplay option in scope for the current milestone is purely
   presentational (theme, notation, confirmations, hotkeys-enable, update rate,
   header coloring, info-display hints). None needs the Rust engine.
2. Mirroring the original key names means that when save/load lands, the store
   can be hydrated from / serialised into the save's `options` block with a
   near-identity mapping — no schema churn.
3. It keeps option-writes off the IPC hot path (no Tauri round-trip per toggle).

When offline progress is implemented, the few engine-consumed keys
(`offlineProgress`, `offlineTicks`, `hibernationCatchup`) get passed into the
relevant Tauri command (e.g. the catch-up call) as parameters rather than being
mirrored into `GameState`. Keep the store the single source of truth; pass
values down when the engine needs them.

This deliberately departs from "all persisted state lives in `GameState`,"
which is justified because options are UI config, the original itself keeps them
in a separate `options` sub-object, and saving is being designed separately.
Flag this for the save/load design so the two agree on the boundary.

### 3.2 New frontend structure

```
stores/options.js          # Pinia store mirroring player.options (persisted)
config/options.js          # static metadata: notation list, theme list,
                           #   confirmation types + unlock predicates,
                           #   info-display rows + unlock predicates
components/tabs/
  OptionsVisualTab.vue
  OptionsGameplayTab.vue
  options/
    OptionsButton.vue          # thin wrapper (vendored o-primary-btn--option)
    PrimaryToggleButton.vue
    ExpandingControlBox.vue
    OptionsSlider.vue          # wraps an <input type=range>; see §3.6
    ModalOptionsToggleButton.vue
    SelectThemeDropdown.vue
    SelectNotationDropdown.vue
    SelectSidebarDropdown.vue
components/modals/options/
    NotationModal.vue
    AnimationOptionsModal.vue
    ConfirmationOptionsModal.vue
    InfoDisplayOptionsModal.vue
    AwayProgressOptionsModal.vue
    NewsOptionsModal.vue
```

Wire the two tabs into `config/tabs.js` (replace the two `component: null`
entries for the `visual` and `gameplay` subtabs).

### 3.3 Modal system reuse

The existing `ui` store has a single `openModal` string
(`help`/`info`/`credits`/`hotkeys`) and `Modal.vue` wrapper. The options modals
fit this model: add ids (`notation`, `animations`, `confirmations`,
`infoDisplay`, `awayProgress`, `news`, `hiddenTabs`) and render the matching
component in `App.vue`'s modal switch. The original's `ModalWrapperOptions` adds
options-specific chrome (`c-modal-options__large`, a button container); replicate
that as a variant prop on `Modal.vue` or a small `ModalOptions.vue` wrapper using
the vendored `c-modal-options*` classes. Keep the "one modal at a time" rule.

### 3.4 Notation → `ad-format`

Notation is the only Visual option that touches the number pipeline. Today
formatting happens in `main.rs` (`format_decimal`); per the number-formatting
doc it will move to the `ad-format` crate (PyO3 + WASM). The notation choice and
`notationDigits` thresholds are inputs to `FormatOptions`.

- Short term: the snapshot still sends pre-formatted strings, so changing
  notation must reach the formatter. Either (a) send the selected notation to
  Rust via a `set_notation` command and have `build_game_view` format with it,
  or (b) defer until WASM formatting lands and the frontend formats raw numbers
  with the store's notation directly. **(b) is cleaner** and aligns with the
  known follow-up; until then, a single `set_notation`/`set_notation_digits`
  command is acceptable.
- Only a subset of the 22 notations need exist initially — at minimum **Mixed
  scientific** (default), **Scientific**, **Engineering**. The "painful"
  notations (Roman, Emoji, Zalgo, …) can come later; the dropdown should list
  only implemented ones to avoid dead entries.
- Enforce the `comma ≤ notation` invariant in the store setters exactly as
  NotationModal does.

### 3.5 Theme system

Themes swap a body CSS class (`t-<name>`, plus `s-base--dark`/`--metro`). The
frontend already pins `t-normal s-base--dark`. To support theme switching:

- Apply the theme by setting `document.body.className` from the store on change
  and on load (mirror `themes.js`'s `set()` minus the engine bits).
- Vendor the additional theme stylesheets as needed; the default **Normal**
  theme is already in place. Start with Normal-only and add themes
  incrementally — the dropdown lists vendored themes only.
- Secret themes are unlocked via `player.secretUnlocks.themes`; out of scope.
- The Modern/Classic UI toggle (V1): **recommend not porting Classic UI.** Hide
  the toggle and assume Modern throughout (matches our vendored CSS and the
  AGENTS note that we target the Modern default). Document as an intentional
  deviation.

### 3.6 Sliders

The original `SliderComponent` is a styled wrapper around `vue-slider-component`.
We don't need that dependency — a native `<input type="range">` styled with the
vendored `o-primary-btn--slider` classes is enough. Three sliders are in scope:
update rate (33–200, step 1), notation digits (two, 3–15, step 1), offline ticks
(22–54 → mapped). Replicate the offline-ticks mapping function verbatim (§2.2).

### 3.7 Update rate

`updateRate` controls the game-loop cadence. Our loop is a `requestAnimationFrame`
loop in `App.vue` (not a fixed `setInterval`), so the semantics differ: rAF runs
~per frame and the engine advances by real `dt`. Options:

- Honour `updateRate` as a **minimum interval between ticks**: accumulate `dt`
  and only call `game.tick` when ≥ `updateRate` ms have elapsed. This reproduces
  the original's "lower = smoother, higher = less CPU" behaviour without changing
  the engine.
- The original's secret achievement at `updateRate === 200` is out of scope.

### 3.8 Confirmations & Info-display gating

Confirmation types and info-display rows carry unlock predicates that reference
game state (galaxies, dimension boosts, `Sacrifice.isVisible`, prestige flags).
Encode these in `config/options.js` as `condition(snapshot)` functions (same
pattern as `tabs.js`), and have the modals filter rows through the live
`game.snapshot`. Pre-Infinity only the four confirmation types and three
info-display rows from §1.4 appear. The confirmation flags themselves must be
read by the dimboost/galaxy/sacrifice/Big-Crunch actions once confirm dialogs
exist (those dialogs are already on todo.md as a separate item) — i.e. the
options store is the source the confirm-dialog code checks.

### 3.9 Away Progress & News — defer

`awayProgress.*` only matters once offline progress exists; the News ticker is
unimplemented (already a "later" item on todo.md). Build the **buttons** and
**modals** for structural parity but treat their effects as no-ops until those
systems land, or omit the buttons until then. Recommend: **omit** the News and
Away Progress buttons initially (less dead UI), add when the backing systems
arrive.


## 4. Scope for the current milestone (pre-Big-Crunch)

The milestone is "play until the first Big Crunch." The minimal, genuinely
functional options set:

**Visual:**
- V5 Notation dropdown (limited list) + V6 Exponent Notation modal — real,
  visible effect on every number once formatting is wired.
- V4 Theme dropdown — start Normal-only (or a small vendored set).
- V2 Update rate slider — real effect on loop cadence.
- V11 Header text coloring — small, self-contained.
- V13 Hotkey List — already done.
- V10 Modify Visible Tabs — already tracked separately on todo.md.

**Gameplay:**
- G2 Hotkeys enable/disable — gates the existing `util/shortcuts.js` handler.
- G1 Confirmation Options modal — the four pre-Infinity types; pairs with the
  planned confirm dialogs.
- G3 Switch-tabs-on-events — low value pre-Infinity (few events); optional.

**Defer:** V1 (Classic UI — recommend never), V3/News, V7/Animations (only
bigCrunch is near-term and ties into the Big-Crunch animation todo item), V8/Info
Display (achievement-ID toggles — minor), V9/Away Progress, V12 Sidebar picker,
G4–G6 offline (no offline sim yet), G7 Automator (post-Reality).

### 4.1 Interaction with existing todo items

`todo.md` already lists, in the current milestone: "First version of the option
screen with selected options", and separately "Big crunch animation + option
setting to disable it", "Modify visible tabs modal + shortcut (Tab)", and
"Confirm dialogs for dimension boost, galaxy, sacrifice and big crunch". This
doc's V7 (bigCrunch animation toggle), V10 (visible tabs), and G1
(confirmations) are the option-screen side of those items — implement the toggle
storage here, the dialog/animation behaviour with those items.


## 5. Suggested implementation order

1. `stores/options.js` (localStorage-persisted, mirrors `player.options` keys)
   + `config/options.js` metadata. No UI yet.
2. Reusable widgets: `OptionsButton`, `PrimaryToggleButton`, `OptionsSlider`,
   `ExpandingControlBox`, options modal wrapper.
3. `OptionsGameplayTab.vue` with G2 (hotkeys) + G3, wired into `tabs.js`.
   Hook G2 into `util/shortcuts.js` (early-return when disabled).
4. `OptionsVisualTab.vue` with V11 + V2 (update rate, with the rAF
   accumulator) + V4 theme (Normal-only) , wired into `tabs.js`.
5. Notation: V5 dropdown + V6 modal, integrated with formatting (prefer the
   WASM/`ad-format` path; `set_notation` command as interim).
6. G1 Confirmation modal (+ gating), shared with the confirm-dialog work.
7. V7 Animation modal (bigCrunch toggle), shared with the Big-Crunch animation.
8. Remaining gated/deferred options as their systems land.


## 6. Reference: `player.options` default schema

```js
options: {
  news: { enabled, repeatBuffer: 40, AIChance: 0, speed: 1, includeAnimated },
  notation: "Mixed scientific",
  notationDigits: { comma: 5, notation: 9 },
  sidebarResourceID: 0,
  retryChallenge, retryCelestial, showAllChallenges,        // challenge tab
  cloudEnabled, hideGoogleName, showCloudModal,             // saving / cloud
  forceCloudOverwrite, syncSaveIntervals,                   // saving / cloud
  hotkeys: true,
  themeClassic: "Normal", themeModern: "Normal",
  updateRate: 33,
  newUI: true,
  offlineProgress: true, loadBackupWithoutOffline: false,
  automaticTabSwitching: true,
  respecIntoProtected: false,
  offlineTicks: 1e5, hibernationCatchup: true,
  statTabResources: 0,
  multiplierTab: { currTab: 0, showAltGroup, replacePowers },
  autosaveInterval: 30000, showTimeSinceSave: true,         // saving tab
  saveFileName: "", exportedFileCount: 0,                   // saving tab
  hideCompletedAchievementRows: false,
  glyphTextColors, headerTextColored: false, ...glyph flags,
  showHintText: { showPercentage, achievements, achievementUnlockStates,
                  challenges, studies, glyphEffectDots, realityUpgrades,
                  perks, alchemy, glyphInfoType, showGlyphInfoByDefault },
  animations: { bigCrunch, eternity, dilation, tachyonParticles, reality,
                background, blobSnowflakes: 16, blobHole },
  confirmations: { /* 21 booleans, all default true */ },
  awayProgress: { /* ~23 booleans, all default true */ },
  hiddenTabBits: 0, hiddenSubtabBits: [...11], lastOpenTab, lastOpenSubtab,
  perkLayout, perkPhysicsEnabled,
  automatorEvents: { newestFirst, timestampType, maxEntries: 200,
                     clearOnReality, clearOnRestart },
  invertTTgenDisplay, autoRealityForFilter,
}
```
(Keys not relevant to Visual/Gameplay — cloud, challenge, glyph, multiplier-tab,
saving — elided/grouped. Full schema: player.js:767.)


## 7. Implementation tracking checklist

Status: ☐ not started · ◐ in progress · ☑ done. "Milestone" = needed for the
first-Big-Crunch milestone; "Later" = post-Infinity or depends on an unported
system.

> **Storage decision (made 2026-06-27):** options live in `ad-core`
> (`GameState.options`, the `Options` struct), **not** a frontend store — so a
> save from a fresh game is valid and options round-trip unchanged through
> load → run → save. This supersedes the frontend-store recommendation in §3.1.
> The snapshot carries `GameView.options`; the frontend writes via commands
> (`set_hotkeys`, `set_update_rate`). Classic UI is dropped; themes/notations
> will be a reduced set.

### Infrastructure
- ☑ `ad-core` `Options` struct in `GameState` (serializable, crunch-preserved) — Milestone
- ☐ `config/options.js` static metadata (notations, themes, confirmation/info rows + gating) — Milestone
- ◐ Reusable widgets: PrimaryToggleButton ☑, OptionsSlider ☑; OptionsButton / ExpandingControlBox pending — Milestone
- ☐ Options modal wrapper (`c-modal-options*` chrome) + `ui.openModal` ids — Milestone
- ☑ Wire `visual` / `gameplay` subtabs in `config/tabs.js` — Milestone

### Visual tab
- ☑ V2 Update rate slider (rAF loop honours updateRate) — Milestone
- ☐ V4 Theme dropdown (Normal-only first; body-class swap) — Milestone
- ☑ V5 Notation dropdown (limited list; → ad-format WASM) — Milestone
- ☑ V6 Exponent Notation modal (comma/notation sliders, comma ≤ notation) — Milestone
- ☐ V11 Relative prestige gain text coloring toggle — Milestone
- ☑ V13 Hotkey List button (already implemented)
- ◐ V10 Modify Visible Tabs modal (tracked on todo.md) — Milestone
- ☐ V7 Animation Options modal (bigCrunch toggle; ties to crunch animation) — Milestone/Later
- ☐ V8 Info Display Options modal (showPercentage, achievement IDs/states) — Later
- ☐ V12 Sidebar resource picker (Modern UI) — Later
- ☐ V3 News Options modal — Later (News unported)
- ✗ V1 UI Modern/Classic toggle — **not porting** (Modern-only); document deviation

### Gameplay tab
- ☑ G2 Hotkeys enable/disable (gates `util/shortcuts.js`) — Milestone
- ☐ G1 Confirmation Options modal (4 pre-Infinity types; pairs w/ confirm dialogs) — Milestone
- ☐ G3 Switch-tabs-on-events toggle — Milestone (optional / low value)
- ☐ G4 Offline progress toggle — Later (no offline sim)
- ☐ G5 Offline ticks slider (mapped 500…1e6) — Later
- ☐ G6 Run suspended time as offline toggle — Later
- ☐ G7 Automator Log Max slider — Later (post-Reality)

### Modals backing later systems
- ☐ V9 Away Progress Options modal — Later (offline progress)
- ☐ News Options modal — Later (News ticker)


## 8. Recommended deviations from the original

1. **Modern UI only** — drop the Classic UI and its toggle (V1). Our vendored
   CSS targets Modern; supporting Classic doubles styling work for no gameplay
   value.
2. **Options in a frontend store, not `GameState`** — for the milestone; revisit
   when save/load defines the persisted-options boundary (§3.1).
3. **Native range inputs** instead of the `vue-slider-component` dependency.
4. **Notation dropdown lists only implemented notations** — no dead entries;
   grow the list as `ad-format` gains strategies.
5. **Omit News / Away Progress buttons until their systems exist** — avoid dead
   UI rather than rendering inert modals.

These should be reflected in `crates/ad-gui/AGENTS.md` once the tabs land.
