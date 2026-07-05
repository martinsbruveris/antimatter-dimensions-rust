---
status: Rejected
---

# Feasibility: Original JS Frontend + Rust/WASM Engine

Status: analysis. This document evaluates an **alternative** to the current
architecture (Rust-authoritative engine + a fresh Vue 3 frontend). The
alternative: keep the *original* Antimatter Dimensions JS/Vue app and swap out
its game engine for one written in Rust, compiled to WASM.

**Conclusion up front:** not recommended. The approach does not save the work it
appears to save, and it adds two large new costs (a wide JS↔WASM boundary and a
per-component refactor of the original UI). The decisive problem is that the
original game has **no thin waist** between UI and engine — the coupling *is* the
engine we would be replacing. A narrower version of the idea (compile the
existing `ad-core` to WASM and drive *our own* snapshot-based Vue frontend on the
web) is genuinely useful and is treated separately in §7.

---

## 1. The proposal, precisely

Two readings of "replace the JS engine with Rust/WASM":

- **(A) Full engine swap.** Port all of `src/core/` (~62k lines) to Rust, compile
  to WASM, and rewire the original `src/components/` (~52k lines of Vue 2) to read
  from the WASM engine instead of the JS `player` object and core classes.
- **(B) Hot-path swap.** Replace only the numerically heavy inner loop (Decimal
  arithmetic + the multiplier pipeline + production) with WASM, leaving JS to
  orchestrate state, autobuyers, prestige, and UI.

Both are analysed below. (A) is what "replace the game engine" normally means.

---

## 2. The key fact: there is no thin waist

A WASM engine swap is only cheap if the UI talks to the engine through a narrow,
well-defined interface (ideally `engine.tick()` + a snapshot read). The original
game is the opposite. Measured against `../antimatter-dimensions`:

| Coupling measurement | Count |
|----------------------|------:|
| `player.*` reads inside `src/core` (engine built on a global singleton) | 3199 |
| `player.*` references inside `src/components` (UI reads raw state) | 678 |
| `format(...)` calls inside `src/components` | 302 |
| `.vue` files calling `format(...)` | 94 |
| References to `Glyphs` / `Pelle` / `Ra` / `Laitela` in components | 165 / 165 / 73 / 40 |

The Vue components do **not** consume a computed view model. They reach directly
into the live engine: a single dimension row references ~9 distinct engine
globals (`AntimatterDimension`, `DimBoost`, `Laitela`, `Pelle`, …), and reads
`Decimal` objects out of them synchronously during render. The boundary between
"UI" and "engine" is thousands of synchronous property accesses returning live
`break_infinity.js` `Decimal` objects — not a function call.

This is already noted briefly in `2026-06-25-frontend-architecture.md`
("the coupling *is* the engine we are replacing"); this document quantifies it
and follows the consequences through.

---

## 3. What the full swap (A) actually costs

Decompose the work and compare against the **current** plan (port the engine to
Rust + build a fresh thin Vue 3 frontend).

### 3.1 Engine port — identical either way

Both approaches require porting all ~62k lines of `src/core/` to Rust. There is
**no saving** here. So the entire case for the alternative rests on the frontend,
not the engine.

### 3.2 The frontend, compared

- **Current plan:** build a new, thin Vue 3 frontend that renders a serialized
  `GameView` snapshot and dispatches `Action`s. JS contains *no* game logic. The
  cost is rebuilding each tab/modal, but the per-component logic is trivial
  (read fields off a plain snapshot object).
- **Alternative (A):** keep the original components but rewire every data access.
  Because each component reads `player.*` and live engine getters directly, the
  rewire is per-line, per-component, and touches ~all of the 52k lines that
  reference the engine. You keep the *markup and CSS* but rewrite the *script* of
  essentially every component — which is most of the porting value gone.

The seductive framing is "keep the original UI for free." In reality you keep the
**templates and stylesheets** for free (which the current plan *already* reuses —
it vendors the CSS verbatim and mirrors the original templates), but you do
**not** keep the component scripts, because their scripts are glued to the engine
object model.

### 3.3 New cost #1: a wide, marshaling FFI boundary

To feed the original components you must expose, across the WASM boundary, the
union of every getter they read — hundreds of distinct derived values
(per-dimension cost/amount/multiplier/affordability, prestige gains, glyph
effects, celestial state, automator status…). Two ways to do it, both bad:

- **Per-getter FFI calls.** Thousands of boundary crossings per frame. WASM↔JS
  calls are cheap individually but not free, and `Decimal` cannot cross as a
  primitive — see below.
- **Whole-state snapshot per frame.** Serialize the entire derived state to JS
  each tick. This is exactly the snapshot model — but the original components
  don't read a snapshot, they read the live object graph, so you'd still rewrite
  them to read the snapshot (back to §3.2), *and* the snapshot for the full game
  (glyphs, celestials, automator, records) is large.

### 3.4 New cost #2: the `Decimal` marshaling problem

The UI displays `Decimal` values everywhere (`format()` is called 302 times).
`break_infinity.js` `Decimal`s are JS objects. If the engine owns numbers on the
Rust side, every value the UI shows must cross the boundary. Options:

- Pass `{mantissa, exponent}` pairs and keep a JS `Decimal`/format layer — i.e.
  you still ship `break_infinity.js` and re-wrap on every read.
- Pre-format to strings in Rust (our `ad-format` already does this) — works, but
  then the JS side can't do its own notation switching/animations without a
  round-trip, and the original components expect `Decimal`s, not strings.

Either way the original components' assumption ("I hold a live `Decimal`") is
violated, forcing edits — again §3.2.

### 3.5 New cost #3: reactivity rebuild

Vue 2's reactivity tracks mutations on the `player` object graph. A WASM engine
mutating Rust-side state produces **no** Vue reactivity. You would have to
maintain a reactive JS mirror of the whole state and reconcile it each tick —
effectively rebuilding a second copy of the state model in JS. This is most of
"reimplement the engine in JS" sneaking back in.

### 3.6 Conflict with project goals

`AGENTS.md` goal #1 is *learn idiomatic Rust, not a line-by-line JS translation*.
To satisfy the original UI, the Rust engine's public surface would have to mirror
the JS object model (same getters, same shapes, same `Decimal` semantics)
verbatim. That pressure pushes the engine toward an un-idiomatic, JS-shaped API —
the opposite of the stated goal.

---

## 4. The hot-path swap (B)

Replace only Decimal arithmetic + the multiplier pipeline + production with WASM;
keep JS orchestration and UI. This avoids the UI refactor but fails on its own
terms:

- The hot path is **interwoven with `player` state access** (3199 reads). The
  multiplier pipeline queries dozens of state fields per dimension per tick. A
  WASM hot path would either pull all that state across the boundary each tick
  (chatty, and `Decimal`-heavy) or duplicate the state in WASM (then JS and WASM
  disagree on the source of truth).
- The JS engine isn't slow because of arithmetic; it runs fine at 30fps. The
  motivation for Rust here is *simulation throughput and safety*, which a partial
  in-browser hot path doesn't deliver (you can't run headless batches of the JS
  orchestration at 1e6 ticks/s).

(B) buys little and complicates the data flow. Not worth it.

---

## 5. Pros and cons of the full swap (A)

### Pros
- **Keeps the original UI's behaviour and polish** — every modal, tooltip,
  automator editor, glyph UI, notation option, theme — *if* the rewire succeeds.
  This is the single biggest remaining cost of the current project.
- The original app stays the visual reference by construction (no transcription
  drift).
- The Rust engine still exists and is reusable for Python/headless/simulation.

### Cons
- **No saving on the 62k-line engine port** — required either way (§3.1).
- **Per-component script rewrite** of ~52k lines of Vue (§3.2): the bulk of the
  apparent "free UI" is not actually free.
- **Wide marshaling FFI boundary** + `Decimal` marshaling + a reactive JS state
  mirror (§3.3–3.5) — much of this is "reimplement the engine in JS" by another
  name.
- **Two engine-shaped surfaces to keep in sync** (Rust internals ↔ the JS-shaped
  WASM API the old UI demands).
- **Pushes the engine API toward un-idiomatic JS shapes** (§3.6).
- Vue 2.6 is EOL; building long-term on it is its own liability.
- Loses the clean Tauri/native packaging path the current design has (WASM-in-
  browser only), unless additionally wrapped.

---

## 6. Side-by-side

| Dimension | Current (Rust engine + fresh Vue 3) | Alternative A (orig UI + WASM engine) |
|-----------|--------------------------------------|----------------------------------------|
| Engine port (62k lines) | Required | Required (no saving) |
| Frontend work | Rebuild thin, snapshot-driven components | Rewrite the script of ~every original component |
| JS↔engine boundary | One narrow snapshot + `Action`s | Wide getter surface + `Decimal` marshaling |
| Reactivity | Trivial (snapshot is plain data) | Must mirror full state reactively in JS |
| Number display | Pre-formatted in Rust (`ad-format`) | Must re-supply `Decimal`s or strings to old UI |
| Idiomatic Rust (goal #1) | Preserved | Compromised (API mirrors JS model) |
| Native/Tauri + Python + headless | Yes (same `ad-core`) | Web/WASM only unless extra wrapping |
| UI fidelity | High via vendored CSS + mirrored templates | Highest, *if* rewire succeeds |
| Net effort vs current | — | Strictly greater |

---

## 7. The version of this idea that *is* worth doing

The genuinely valuable kernel: **`ad-core` can compile to WASM and power a web
build of our own snapshot-driven frontend.** Our architecture is already
Rust-authoritative — the Tauri frontend calls `tick_and_get_state` and renders a
`GameView`. Swapping the transport from Tauri IPC to a WASM module export changes
the boundary, not the model:

```
Tauri today:   Vue 3  ──IPC──►  ad-core (native)   ──► GameView snapshot
Web variant:   Vue 3  ──wasm──► ad-core (wasm32)    ──► GameView snapshot
```

This keeps the narrow snapshot boundary, reuses the same engine crate, and adds a
web deployment target — without adopting the original game's coupling. It is the
right way to "run the Rust engine behind a web UI." It uses *our* thin Vue 3
frontend, not the original coupled one.

### 7.1 Does this give a deployable web version of the game?

Yes — a **fully client-side** web build, shippable as static files. `ad-core`
compiled to `wasm32-unknown-unknown` (via `wasm-bindgen`) becomes a JS-importable
module that runs **in the browser**: the engine executes on the player's machine,
exactly as the original game does, with **no server** required. It can be hosted
on any static host (GitHub Pages, Netlify, Cloudflare Pages, an object-storage
bucket).

The reason it is cheap is that only the *transport* is platform-specific. The
frontend already treats the engine as "call `tick_and_get_state`, render the
`GameView` snapshot, dispatch `Action`s" — it does not care how that call is
delivered. Native uses Tauri IPC; web uses a direct WASM call. The Vue components
are unchanged.

### 7.2 What you would build

- **A transport abstraction in the frontend.** Today the store calls Tauri's
  `invoke()`. Introduce one interface with two backends — Tauri `invoke` (native)
  and the WASM exports (web) — selected at build time. Components keep calling the
  store, not the transport.
- **WASM bindings for `ad-core`.** A thin `#[wasm_bindgen]` wrapper exposing
  `tick_and_get_state` and the action-dispatch entry points, mirroring the Tauri
  commands in `ad-gui/src/main.rs`. This is the same pattern `ad-format` is
  already slated for, one layer up.
- **A web save/load path.** The one genuinely platform-specific piece. The native
  Tauri build writes save files to disk; the web build persists to `localStorage`
  or `IndexedDB` instead. Small and self-contained.

The payoff is **both** targets from one codebase — the native Tauri desktop app
*and* a browser build — sharing the same `ad-core` engine and the same Vue 3
frontend. That is a property the original game does not have: it is web-only, and
its engine is JS.

### 7.3 Caveats

- **Snapshot size/cadence.** The web path serializes a `GameView` each tick across
  the WASM boundary. For full late-game state (glyphs, celestials, records) that
  snapshot grows; if it ever becomes a per-frame cost, trim it to dirty fields or
  a coarser cadence. Not a blocker, just a thing to watch as the game grows.
- **A deployment target, not a near-term deliverable.** It only pays off once
  `ad-core` covers enough of the game to be worth shipping. Don't build the WASM
  target now — but the architecture keeps the door open at low cost, which is the
  point.
- **No threads/SIMD by default in basic WASM**, and none are needed: the engine
  runs comfortably at ~30fps single-threaded (the JS original does). The headless
  *simulation-throughput* goal stays on the native/Python side, where it belongs.

`ad-format` is already slated for a WASM target (see
`2026-06-25-number-formatting.md`); extending that to `ad-core` is a natural,
low-risk follow-on once the engine is further along.

---

## 8. Recommendation

1. **Do not** adopt alternative (A) or (B). Neither saves the engine-port cost,
   and (A) adds more frontend work than it removes plus a heavy boundary.
2. **Keep** the current Rust-authoritative design and the fresh, snapshot-driven
   Vue 3 frontend, continuing to vendor the original CSS and mirror templates for
   fidelity.
3. **When useful**, add a **WASM compile target for `ad-core`** to ship the
   existing snapshot frontend on the web (§7). This captures the "Rust engine,
   web UI" goal cleanly.

---

*Document created: 2026-06-28.*
</content>
</invoke>
