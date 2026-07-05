# Documentation

This directory holds the project's documentation. Docs fall into three kinds,
separated by **when** they are written and **how** they are maintained:

| Kind | Where | Written | Maintenance |
|------|-------|---------|-------------|
| **Living reference** | [`ARCHITECTURE.md`](ARCHITECTURE.md), `crates/*/ARCHITECTURE.md`, [`../AGENTS.md`](../AGENTS.md), [`../crates/ad-gui/AGENTS.md`](../crates/ad-gui/AGENTS.md) | continuously | **Kept up to date** — always describes how the code works *now*. |
| **Design docs / RFCs** | [`design/`](design/) | **before** coding | Historical. Not rewritten; only `status` and embedded checkboxes change. |
| **Worklog** | [`worklog/`](worklog/) | **after** coding | Historical, **append-only**. New file per session; existing files are never edited. |
| **Porting guide** | [`PORTING.md`](PORTING.md) | — | Living reference for the fidelity method + the original game. |

Design docs and worklogs are **both historical artifacts** — the "before" and
"after" bookends of a piece of work. The living truth about how the code works
now lives in the `ARCHITECTURE.md` files, never in a dated design doc.

## Design doc conventions

- **File name:** `design/YYYY-MM-DD-<slug>.md` (the date is the day it was
  written; keep it stable when moving/renaming — use `git mv`).
- **Front-matter:** each design doc begins with a YAML block:

  ```yaml
  ---
  title: Normal Challenges
  status: Implemented        # Proposed | Accepted | Implemented | Partial | Superseded | Rejected
  feature: 2.5               # optional — the feature id this doc plans
  superseded_by: 2026-07-05-automator.md   # only when status: Superseded
  ---
  ```

  Status meanings:
  - **Proposed** — drafted, not yet agreed.
  - **Accepted** — agreed, not yet (fully) built.
  - **Implemented** — built; the code matches this doc's intent.
  - **Partial** — partly built; the doc notes deferred pieces.
  - **Superseded** — a later doc replaces it (`superseded_by:`).
  - **Rejected** — considered and not pursued (kept for the reasoning).

  When code diverges from a design doc, change its `status` (and note the delta
  in the implementing worklog entry) rather than rewriting the doc body.

- **The only in-place edits allowed** after a design doc is written are the
  `status` field and ticking checkboxes inside embedded plans (as in
  `feature-decomposition`).

## Cross-linking

Wire the three kinds together so any one of them leads to the others:

- **Worklog → design:** each worklog entry links the design doc(s) it implements
  and records deviations from them.
- **Design → reality:** when a design doc is built, flip its `status`; the
  authoritative "how it works now" is the relevant `ARCHITECTURE.md`.
- **Architecture → design:** `crates/*/ARCHITECTURE.md` file entries link the
  design doc that introduced each system (for the *why*).

## Keeping this index current

This index is maintained by hand. When you add, re-categorize, or change the
status of a design doc, update the table below in the same change. (The codebase
is small enough that hand-maintenance is fine; revisit if it starts to drift.)

## Design doc index

Located in [`design/`](design/). Read these before making architectural
decisions; the architecture doc is the primary reference.

| Document | Summary |
|----------|---------|
| [`2026-06-11-codebase-analysis.md`](design/2026-06-11-codebase-analysis.md) | Full analysis of the original JS game's architecture |
| [`2026-06-11-endgame-analysis.md`](design/2026-06-11-endgame-analysis.md) | Analysis of the endgame mod's additions |
| [`2026-06-19-architecture.md`](design/2026-06-19-architecture.md) | Rust project architecture, workspace layout, design decisions |
| [`2026-06-19-break-infinity-review.md`](design/2026-06-19-break-infinity-review.md) | Code review of the vendored break_infinity crate |
| [`2026-06-21-break-eternity-representation.md`](design/2026-06-21-break-eternity-representation.md) | Design for extending Decimal to support break_eternity (tower numbers) |
| [`2026-06-24-ui-framework-analysis.md`](design/2026-06-24-ui-framework-analysis.md) | Comparison of GUI framework options for the playable frontend |
| [`2026-06-25-frontend-architecture.md`](design/2026-06-25-frontend-architecture.md) | `ad-gui` design: Tauri + Vue 3/Vite/Pinia, vendored CSS, Rust-authoritative snapshot |
| [`2026-06-25-number-formatting.md`](design/2026-06-25-number-formatting.md) | Where number formatting lives (Rust now; PyO3 + WASM later) and why |
| [`2026-06-27-options-tabs.md`](design/2026-06-27-options-tabs.md) | Analysis of the Visual & Gameplay options tabs + iterative port plan |
| [`2026-06-27-simulation-architecture.md`](design/2026-06-27-simulation-architecture.md) | Options for a full end-to-end simulation driver (Action IR + Controller trait) kept cleanly separate from game logic |
| [`2026-06-28-js-frontend-rust-wasm-engine.md`](design/2026-06-28-js-frontend-rust-wasm-engine.md) | Feasibility analysis of keeping the original JS/Vue app and swapping its engine for Rust/WASM (rejected; recommends a WASM target for `ad-core` instead) |
| [`2026-06-30-offline-progress.md`](design/2026-06-30-offline-progress.md) | How the original simulates offline progress, how it maps onto our `simulate`/`ticks` primitives, the game-speed/timestamp implications, and a design for a manual Offline-mode button |
| [`2026-06-30-ui-reveal-and-tutorial.md`](design/2026-06-30-ui-reveal-and-tutorial.md) | Progressive UI reveal (hiding/showing AD rows, tickspeed, sacrifice), first-time/disable-able confirmation modals (boost/galaxy/sacrifice/crunch), and the tutorial glow + exclamation highlight; how the original implements each and a phased plan |
| [`2026-06-30-achievements.md`](design/2026-06-30-achievements.md) | Normal achievements: bitmask state on `GameState`, unlock hooks inline in the buy/galaxy/boost/crunch/tick methods (rows 1–2 minus News), per-achievement effects + the global achievement-power multiplier, `achievementBits` save round-trip, the sprite-driven tab, and the unlock toast; phased plan |
| [`2026-07-02-infinity-points-and-records.md`](design/2026-07-02-infinity-points-and-records.md) | Completing Feature 2.1: Infinity Points / Infinities currency, the `Records` struct (time played, this/best infinity), the IP gain formula (pre-break = 1), Big Crunch reward+reset semantics, save/load round-trip, and the Infinity tab + IP header |
| [`2026-07-03-infinity-upgrades.md`](design/2026-07-03-infinity-upgrades.md) | Feature 2.2: the 16-upgrade Infinity grid — data table, bitmask state, purchase/column prereqs, every effect and its engine application site, passive `ipGen`, save/load, and the grid UI; bottom row (`ipMult`/`ipOffline`) deferred |
| [`2026-07-03-normal-challenges.md`](design/2026-07-03-normal-challenges.md) | Feature 2.5: the 12 Normal Challenges — run state machine (start/complete/exit, forced Big-Crunch reset, unlock chain), all 12 modifiers mapped to their engine sites, reward→autobuyer wiring, save/load, the Challenges tab UI, and an incremental plan (NC1 slice first) |
| [`2026-07-03-break-infinity.md`](design/2026-07-03-break-infinity.md) | Feature 2.3: Break Infinity + its 12 upgrades — lifting the 1e308 cap, the scaling IP formula, and the one-time/rebuyable upgrade effects |
| [`2026-07-03-replicanti.md`](design/2026-07-03-replicanti.md) | Feature 3.2: Replicanti — growth approximation, Replicanti Galaxies, the three IP upgrades, and the tickspeed/ID interactions |
| [`2026-07-03-autobuyers.md`](design/2026-07-03-autobuyers.md) | Feature 2.6: the automation system — AD/Tickspeed autobuyers, the challenge-only prestige autobuyers, and the IP-cost interval-upgrade machinery |
| [`2026-07-04-dilation.md`](design/2026-07-04-dilation.md) | Phase 5 (Time Dilation): design for Features 5.1–5.2 — dilation studies, the dilated run + TP/DT/Tachyon-Galaxy mechanics, and the Dilation Upgrades |
| [`2026-07-04-eternity.md`](design/2026-07-04-eternity.md) | Phase 4 (Eternity): design for Features 4.1–4.6 — EP formula + reset semantics, milestones, Time Dimensions/free tickspeed, the Time Studies tree + effect map, Eternity Challenges, Eternity Upgrades; frontier corrections (TD5–8 are Dilation-gated; Big Crunch resets Replicanti) |
| [`2026-07-04-tab-notifications.md`](design/2026-07-04-tab-notifications.md) | Tab notification badges (the yellow `!` on tabs): the original's two-field state + trigger/clear semantics, the 5 in-frontier notifications, and the engine-owned port (trigger hooks, save round-trip, sidebar rendering + seen-acknowledgement) |
| [`2026-07-05-reality.md`](design/2026-07-05-reality.md) | Phase 6 (Reality): design for Features 6.1–6.5 — RM formula + reality reset, the seeded glyph generator/effects/sacrifice, the perk tree, Reality Upgrades, Black Holes; frontier cuts (celestial content, automator) and the save mapping |
| [`2026-07-05-automator.md`](design/2026-07-05-automator.md) | Feature 6.6 (Automator): mechanics, frontier cuts, the five-stage porting plan (engine prerequisites / language core / execution engine / text-editor UI / block editor + templates + import-export) and per-stage implementation notes (§12–§16) |

The table lists key documents; see the [`design/`](design/) folder for the full,
date-prefixed set (including the fidelity/test-plan and research material).
