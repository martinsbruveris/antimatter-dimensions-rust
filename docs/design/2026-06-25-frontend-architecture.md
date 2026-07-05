---
status: Implemented
---

# Frontend Architecture: `ad-gui` (Vue 3 + Tauri)

Status: active. This crate was originally created as `ad-gui-3` and later renamed
to `ad-gui`; the two earlier prototypes described below (egui and vanilla-JS)
have since been deleted. The current layout is documented in
`crates/ad-gui/AGENTS.md`.

## Context

The GUI is one of the project's deliverables, and the plan is to reimplement the
*whole* game, not just pre-infinity. Two earlier prototypes (now removed)
informed this:

- an egui/eframe native frontend (the original `ad-gui`).
- a Tauri app with a hand-written vanilla-JS frontend (`ad-gui-2`) and
  **hand-transcribed** CSS. The hand-transcription was a direct source of visual
  bugs (wrong button color `#455a64`, wrong button text size `1.1rem` — both
  were typos against the original stylesheet).

The original game (`../antimatter-dimensions`) is **Vue 2.6 + vue-cli**, and its
components are tightly coupled to the full JS engine: a single dimension row
references 9 distinct engine globals (`AntimatterDimension`, `DimBoost`,
`Laitela`, `Pelle`, …) backed by ~238 `.js` files. So the original components
**cannot be lifted wholesale** onto the Rust engine — the coupling *is* the
engine we are replacing. See `2026-06-24-ui-framework-analysis.md`.

## Decision

A fresh crate (created as `ad-gui-3`, now `crates/ad-gui`):

- **Tauri + Vue 3 + Vite + Pinia.** Vite is the standard Vue 3 build; Pinia is
  the official store. The Rust backend (Tauri commands) was carried over from the
  vanilla-JS prototype essentially verbatim.
- **Rust-authoritative.** `GameState` in `ad-core` is the single source of truth.
  Each frame the frontend calls `tick_and_get_state` and receives a serialized
  snapshot (`GameView`). The JS side never computes game logic — it renders the
  snapshot and dispatches actions (`buy_dimension`, `sacrifice`, …) over IPC.
- **Vendored stylesheets, verbatim.** The original `public/stylesheets/*.css`
  and fonts are copied unmodified into `frontend/public/stylesheets/`. We never
  re-transcribe CSS again. The Modern-UI default theme is selected with the body
  class `t-normal s-base--dark` (from the original `themes.js`: the default
  theme `isDark()` follows `newUI`).
- **Reuse template markup + component scoped styles.** Components mirror the
  original Modern components' `<template>` and class names. Classes that live in
  the original components' `<style scoped>` (not the global CSS — e.g.
  `.l-tickspeed-container`, `.c-dim-row__large`, `.l-modern-buy-ad-text`) are
  replicated in the corresponding `ad-gui` component's scoped style. Only the
  `<script>` is rewritten to read the Pinia snapshot instead of engine globals.
- **A minimal app-shell** (`src/app-shell.css`) supplies just the centered,
  scrollable container; the original's large layout shell (`#ui`, sidebar,
  `tab-container`) is not reproduced. All game-component styling comes from the
  vendored CSS.

### Why not run the original Vue frontend against a Rust shim
The components expect a synchronous, richly-structured in-process object model
and `break_infinity.js`. Tauri IPC is async and serialized; mirroring the whole
model in JS and reimplementing every derived getter would rebuild most of the
engine in JS. And a single component pulls in many unimplemented systems
(Continuum, Pelle, challenges, tutorials). Selective template reuse + a
Rust-authoritative snapshot captures the value without the coupling.

## Structure

This is the initial M1 layout; see `crates/ad-gui/AGENTS.md` for the current,
authoritative file tree (stores, config, `components/tabs/`, …).

```
crates/ad-gui/
  src/main.rs            # Tauri commands + GameView snapshot
  tauri.conf.json        # frontendDist = ./frontend/dist (serves built dist/)
  frontend/
    index.html           # <body class="t-normal s-base--dark"> + vendored <link>s
    vite.config.js
    public/stylesheets/   # VENDORED verbatim from the original game
    src/
      main.js             # createApp + Pinia + app-shell.css
      app-shell.css       # minimal centered/scrollable container (ours)
      stores/game.js      # Pinia: snapshot + buyUntil10 + action dispatchers
      util/dimensionText.js
      components/          # GameHeader, AntimatterDimensionsTab, DimensionRow,
                          # TickspeedRow, DimBoostRow, GalaxyRow, ProgressBar
```

## Number formatting

Milestone 1 keeps formatting in Rust, pre-baked into the snapshot. The eventual
move to a shared `ad-format` crate (PyO3 + WASM) is tracked in
`2026-06-25-number-formatting.md`.

## Milestones

- **M1 (done): pre-infinity parity** with the vanilla-JS prototype — antimatter
  dimensions, tickspeed, dim boosts, galaxies, sacrifice, the progress bar. The
  multi-page shell (sidebar nav, achievements page) followed.
- Later: infinity and beyond (new tabs as Pinia stores/components); `ad-format`
  + WASM; notation options; save/load; non-default themes.
