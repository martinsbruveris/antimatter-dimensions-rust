---
status: Reference
---

# UI Framework Analysis

**Date:** 2026-06-24 **Status:** Analysis / Decision Pending

## Context

The original Antimatter Dimensions game is a web app built with Vue.js and extensive CSS
styling (dark theme, CSS grid, flexbox, animations, custom fonts, themed variables). The
current Rust GUI uses egui, which is an immediate-mode GUI designed for tooling — not
styled game UIs. This document analyzes options for more faithfully recreating the
original game's look and feel.

## What the Original Game's UI Requires

- **CSS Grid/Flexbox** — precise 7-column grid layouts for dimension rows
- **CSS transitions/animations** — color-cycling antimatter text (25s keyframes), 0.2s
  hover transitions
- **Custom fonts** — Monospace Typewriter serif
- **Themed CSS variables** — swappable color schemes (Dark, Metro, AMOLED, etc.)
- **Pixel-precise spacing** — rem-based sizing, border-radius, box-shadows
- **Hover/disabled states** — rich interactive feedback per widget
- **Responsive layout** — adapts to window width (narrow/wide dimension text)

## Options

### 1. Tauri (Rust backend + HTML/CSS/JS frontend)

A webview-based approach where Rust handles the game engine via IPC and the frontend is
rendered in a system webview.

**How it works:** Could literally reuse the original game's Vue components and CSS. Rust
`GameState` is exposed via Tauri commands; the frontend calls into Rust for game logic.

**Pros:**
- Pixel-perfect match to the original; can import stylesheets directly
- Mature ecosystem; well-documented
- Fast iteration on UI (standard web dev tools)

**Cons:**
- Adds a webview dependency (~30MB on some platforms)
- Bridges Rust↔JS via IPC commands (serialization overhead per frame)
- Not "pure Rust" — frontend is JS/TS
- Two languages in the project

### 2. Dioxus (React-like, with web or desktop renderer)

Write UI in Rust with RSX syntax. Supports real CSS when targeting the web renderer.
Desktop renderer uses webview (like Tauri) or the experimental Blitz native renderer.

**How it works:** Components are Rust functions returning RSX. In web mode, it compiles
to WASM and renders in a browser/webview with full CSS support. Desktop mode wraps a
webview.

**Pros:**
- Rust-native component model with CSS support in web mode
- Hot reload available
- Single language (Rust) for both logic and UI

**Cons:**
- Desktop native renderer (Blitz) is immature/experimental
- Web mode = webview under the hood (similar tradeoffs to Tauri)
- Smaller ecosystem than Tauri

### 3. Slint (Declarative UI with its own styling language)

A declarative UI toolkit with its own `.slint` markup language. Supports properties,
animations, transitions, states, gradients, border-radius, opacity, custom fonts, and
repeaters.

**How it works:** UI is defined in `.slint` files with a CSS-like property syntax. Rust
code binds data to the UI. Compiles to native (no webview).

**Pros:**
- Purpose-built for styled applications
- Compiles to native with GPU acceleration; no webview
- Visual editor available (Slint Design)
- Built-in animation/transition support

**Cons:**
- Not CSS — requires manual translation of styles
- Commercial license required for closed-source distribution (free for open
  source/evaluation)
- Smaller community than web-based options

### 4. Floem (Xilem-inspired, CSS-like inline styling)

A reactive UI library originating from the Lapce editor project. Uses a composable view
tree with chained style methods that closely mirror CSS properties.

**How it works:** Views are composed functionally. Styling is applied via method chains:
`.border_radius(4.0).background(Color::rgb(30,30,30)).hover(|s| s.background(...))`.
Layout is flexbox-based.

**Pros:**
- Most "CSS-like" API of any pure-native Rust option
- Flexbox layout with responsive sizing
- Hover/active/focus/disabled styles per-view (analogous to CSS pseudo-classes)
- Reactive state model (signals/derived values)
- Pure Rust native rendering (wgpu/vello); no webview

**Cons:**
- Young project; smaller community; thinner documentation
- No visual editor
- No CSS grid (flexbox only)
- No CSS parsing — still requires manual translation

### 5. Bevy (Game engine with bevy_ui)

A data-driven game engine with an ECS architecture. Its built-in `bevy_ui` module uses a
CSS flexbox-inspired layout system.

**How it works:** UI nodes are entities with `Style` components (flex_direction,
justify_content, padding, border, etc.). Text nodes support custom fonts. Buttons use
interaction systems.

**Pros:**
- Pure Rust, GPU-accelerated native rendering
- Flexbox layout model closer to CSS than egui
- First-class animation/interpolation support
- If visual effects are added later (particles, shaders), the full engine is available

**Cons:**
- Massive dependency (~300 crates); 30-60s+ clean compile times
- Overkill — uses 5% of the engine for a text-heavy UI
- `bevy_ui` is considered immature (limited scrolling, no grid, verbose)
- Boilerplate-heavy (~15 lines to spawn a button)
- Conflicts with architecture — Bevy wants ECS ownership of state; `GameState` needs
  bridging

### 6. Iced (Elm architecture, custom styling traits)

An Elm-inspired GUI library where each widget has a `Style` struct you implement.
Supports themes, custom fonts, colors, border-radius, and shadows.

**How it works:** Application state updates via messages. The `view()` function builds
the widget tree. Custom `Theme` implementations control all widget styling.

**Pros:**
- Pure Rust; clean Elm-like architecture
- Good theming support for dark-themed apps
- Active development; growing community

**Cons:**
- No CSS parsing; all styles are Rust code
- No grid layout (Row/Column/Container only)
- No keyframe animations
- High manual effort to translate CSS

### 7. egui (Current approach, with improvements)

Continue with egui but invest in custom `Widget` implementations using the `Painter` API
for borders, rounded rects, and time-based color animations. Use `egui::Grid` for
dimension rows.

**Pros:**
- No migration effort; incremental improvement
- Fast compile times; minimal dependencies
- Good for rapid prototyping

**Cons:**
- Hard ceiling on fidelity — text layout, spacing, and animation support are limited
- Immediate-mode paradigm fights against stateful styling (hover transitions, etc.)
- No CSS grid; limited custom font control

## Comparison Table

| Approach | Fidelity | Compile Time | Complexity | Best For |
|----------|----------|-------------|-----------|----------|
| Tauri | ★★★★★ | Fast (Rust side) | Low | Exact CSS match |
| Dioxus-web | ★★★★☆ | Medium | Medium | Rust + CSS hybrid |
| Slint | ★★★★☆ | Medium | Medium | Native + rich styling |
| Floem | ★★★½☆ | Medium | Medium | CSS-like native Rust |
| Bevy | ★★★☆☆ | Slow | High | Future visual effects |
| Iced | ★★★☆☆ | Medium | High | Pure Rust minimalist |
| egui | ★★☆☆☆ | Fast | Low | Quick iteration |

## Key Tradeoff

The fundamental choice is:

- **Web-based (Tauri/Dioxus-web):** Highest fidelity with lowest effort, since the
  original is already HTML/CSS. But adds a webview and splits the project across
  languages.
- **Native with rich styling (Slint/Floem):** Pure Rust, no webview, but requires manual
  CSS-to-native translation and loses some capabilities (animations, grid).
- **Native minimal (egui/Iced/Bevy):** Simplest dependency story but furthest from the
  original look.

## Recommendation

If the primary goal is visual fidelity to the original game:
- **Tauri** is the pragmatic choice — reuse existing CSS, minimal UI code.
- **Slint** or **Floem** if "pure Rust, no webview" is a hard requirement.

If the primary goal is learning Rust and the UI is secondary:
- **egui** (current) or **Iced** keeps things simple.
- Upgrade to Slint/Floem when the game systems are more complete.
