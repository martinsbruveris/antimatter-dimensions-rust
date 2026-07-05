# Documentation

This directory holds the project's documentation. Docs fall into a few kinds,
separated by **when** they are written and **how** they are maintained:

| Kind | Where | Written | Maintenance |
|------|-------|---------|-------------|
| **Living reference** | [`ARCHITECTURE.md`](ARCHITECTURE.md), `crates/*/ARCHITECTURE.md`, [`../AGENTS.md`](../AGENTS.md), [`../crates/ad-gui/AGENTS.md`](../crates/ad-gui/AGENTS.md) | continuously | **Kept up to date** — always describes how the code works *now*. |
| **Design docs / RFCs** | [`design/`](design/) | **before** coding | Historical. Not rewritten; only `status` and embedded checkboxes change. |
| **Analysis / reference** | [`analysis/`](analysis/) | as needed | Background study of the original JS game. Stable reference (`status: Reference`); not tied to our build state. |
| **Worklog** | [`worklog/`](worklog/) | **after** coding | Historical, **append-only**. New file per session; existing files are never edited. |
| **Porting guide** | [`PORTING.md`](PORTING.md) | — | Living reference for the fidelity method + the original game. |

Design docs and worklogs are **both historical artifacts** — the "before" and
"after" bookends of a piece of work. The living truth about how the code works
now lives in the `ARCHITECTURE.md` files, never in a dated design doc.

## Front-matter convention

Every doc under `design/` and `analysis/` begins with a YAML front-matter block.
The H1 heading is the title; front-matter carries the machine-readable status.

```yaml
---
status: Implemented        # see values below
feature: "2.5"             # optional — the feature id(s) this doc plans (may be a range, e.g. "4.1-4.6")
superseded_by: 2026-07-05-automator.md   # optional — only when status: Superseded
---
```

Status values:

- **Proposed** — drafted, not yet agreed or not yet built.
- **Accepted** — agreed, not yet (fully) built.
- **Implemented** — built; the code matches this doc's intent.
- **Partial** — partly built; named sub-features are deferred (the doc/port-audit
  notes which).
- **Superseded** — a later doc replaces it (set `superseded_by:`).
- **Rejected** — considered and not pursued (kept for the reasoning).
- **Reference** — an analysis, review, or exploration doc with no single feature
  to "complete" (all of `analysis/`, plus code reviews and explorations in
  `design/`). Read for background; not an implementation plan.

When code diverges from a design doc, change its `status` (and note the delta in
the implementing worklog entry) rather than rewriting the doc body. **The only
in-place edits allowed** after a design doc is written are the `status` field and
ticking checkboxes inside embedded plans (as in `feature-decomposition`).

> Statuses were seeded from the code-level
> [port audit](design/2026-07-05-port-audit.md) (2026-07-05) and reviewed.

## Cross-linking

Wire the kinds together so any one of them leads to the others:

- **Worklog → design:** each worklog entry links the design doc(s) it implements
  and records deviations from them.
- **Design → reality:** when a design doc is built, flip its `status`; the
  authoritative "how it works now" is the relevant `ARCHITECTURE.md`.
- **Architecture → design:** `crates/*/ARCHITECTURE.md` file entries link the
  design doc that introduced each system (for the *why*).

## Keeping this index current

This index is maintained by hand. When you add a doc, re-categorize one, or change
a `status`, update the matching table row in the same change. (The codebase is
small enough that hand-maintenance is fine; revisit if it starts to drift.) The
[port audit](design/2026-07-05-port-audit.md) is the authoritative snapshot of
what is actually implemented vs. remaining.

## Design docs (RFCs)

In [`design/`](design/). Written before coding; read before making architectural
decisions.

| Status | Document | Summary |
|--------|----------|---------|
| Implemented | [`2026-06-19-architecture.md`](design/2026-06-19-architecture.md) | Rust project architecture, workspace layout, design decisions |
| Reference | [`2026-06-19-break-infinity-review.md`](design/2026-06-19-break-infinity-review.md) | Code review of the vendored `break_infinity` crate |
| Proposed | [`2026-06-21-break-eternity-representation.md`](design/2026-06-21-break-eternity-representation.md) | Design for extending `Decimal` to `break_eternity` (tower numbers) |
| Reference | [`2026-06-23-fidelity-analysis.md`](design/2026-06-23-fidelity-analysis.md) | Fidelity analysis: Rust implementation vs the original JS |
| Proposed | [`2026-06-23-fidelity-test-plan.md`](design/2026-06-23-fidelity-test-plan.md) | Fidelity test plan for the pre-Infinity & Infinity stages |
| Implemented | [`2026-06-24-experiment-architecture.md`](design/2026-06-24-experiment-architecture.md) | Strategy-based simulation experiment architecture (`ad-sim`) |
| Reference | [`2026-06-24-ui-framework-analysis.md`](design/2026-06-24-ui-framework-analysis.md) | Comparison of GUI framework options for the playable frontend |
| Implemented | [`2026-06-25-frontend-architecture.md`](design/2026-06-25-frontend-architecture.md) | `ad-gui` design: Tauri + Vue 3/Vite/Pinia, vendored CSS, Rust-authoritative |
| Implemented | [`2026-06-25-number-formatting.md`](design/2026-06-25-number-formatting.md) | Where number formatting lives (Rust + WASM) and why |
| Implemented | [`2026-06-27-how-to-play-modal.md`](design/2026-06-27-how-to-play-modal.md) | How-To-Play (H2P) modal design notes |
| Partial | [`2026-06-27-options-tabs.md`](design/2026-06-27-options-tabs.md) | Visual & Gameplay options tabs — analysis + iterative port plan |
| Implemented | [`2026-06-27-simulation-architecture.md`](design/2026-06-27-simulation-architecture.md) | End-to-end simulation driver (Action IR + Controller trait), kept separate from game logic |
| Partial | [`2026-06-28-ad-format-test-plan.md`](design/2026-06-28-ad-format-test-plan.md) | Test plan for the `ad-format` crate |
| Rejected | [`2026-06-28-js-frontend-rust-wasm-engine.md`](design/2026-06-28-js-frontend-rust-wasm-engine.md) | Feasibility of keeping the JS frontend + a Rust/WASM engine (rejected) |
| Implemented | [`2026-06-28-save-load-analysis.md`](design/2026-06-28-save-load-analysis.md) | Save/load analysis + codec design (round-trips real original saves) |
| Partial | [`2026-06-30-achievements-roadmap.md`](design/2026-06-30-achievements-roadmap.md) | Achievements feature-correlation & secret-achievement analysis; per-feature rollout |
| Partial | [`2026-06-30-achievements.md`](design/2026-06-30-achievements.md) | Normal achievements: bitmask state, inline unlock hooks, effects & tab |
| Implemented | [`2026-06-30-offline-progress.md`](design/2026-06-30-offline-progress.md) | Offline progress & the manual Offline-mode button |
| Implemented | [`2026-06-30-ui-reveal-and-tutorial.md`](design/2026-06-30-ui-reveal-and-tutorial.md) | Progressive UI reveal, first-time confirmations, and the tutorial highlight |
| Proposed | [`2026-07-01-config-driven-engine.md`](design/2026-07-01-config-driven-engine.md) | Design exploration of a config-driven game engine |
| Implemented | [`2026-07-02-infinity-points-and-records.md`](design/2026-07-02-infinity-points-and-records.md) | Feature 2.1: Infinity Points / Infinities, the `Records` struct, Big Crunch reward+reset |
| Implemented | [`2026-07-03-autobuyers.md`](design/2026-07-03-autobuyers.md) | Feature 2.6: the automation system (AD/Tickspeed + prestige autobuyers, interval upgrades) |
| Implemented | [`2026-07-03-break-infinity.md`](design/2026-07-03-break-infinity.md) | Feature 2.3: Break Infinity + its 12 upgrades |
| Implemented | [`2026-07-03-infinity-challenges.md`](design/2026-07-03-infinity-challenges.md) | Feature 2.7: the 8 Infinity Challenges (restrictions + rewards) |
| Implemented | [`2026-07-03-infinity-dimensions.md`](design/2026-07-03-infinity-dimensions.md) | Feature 3.1: Infinity Dimensions → Infinity Power |
| Partial | [`2026-07-03-infinity-upgrades.md`](design/2026-07-03-infinity-upgrades.md) | Feature 2.2: the 16-upgrade Infinity grid (bottom row `ipMult`/`ipOffline` deferred) |
| Implemented | [`2026-07-03-normal-challenges.md`](design/2026-07-03-normal-challenges.md) | Feature 2.5: the 12 Normal Challenges (all modifiers + reward→autobuyer wiring) |
| Implemented | [`2026-07-03-replicanti.md`](design/2026-07-03-replicanti.md) | Feature 3.2: Replicanti + Replicanti Galaxies + the 3 IP upgrades |
| Partial | [`2026-07-04-dilation.md`](design/2026-07-04-dilation.md) | Phase 5 (5.1–5.2): Time Dilation + Dilation Upgrades (Pelle-only upgrades out of frontier) |
| Partial | [`2026-07-04-eternity.md`](design/2026-07-04-eternity.md) | Phase 4 (4.1–4.6): Eternity, Milestones, Time Dimensions, Time Studies, Eternity Challenges, Eternity Upgrades |
| Implemented | [`2026-07-04-tab-notifications.md`](design/2026-07-04-tab-notifications.md) | Tab notification badges (the yellow `!` on tabs) |
| Implemented | [`2026-07-05-automator.md`](design/2026-07-05-automator.md) | Feature 6.6: the Automator (all five stages) |
| Reference | [`2026-07-05-port-audit.md`](design/2026-07-05-port-audit.md) | Code-level audit of what is ported vs remaining (2026-07-05) — the current status snapshot |
| Partial | [`2026-07-05-reality.md`](design/2026-07-05-reality.md) | Phase 6 (6.1–6.5): Reality, Glyphs, Perks, Reality Upgrades, Black Holes |

## Analysis (study of the original JS game)

In [`analysis/`](analysis/). Background reference, not tied to our build state.

| Status | Document | Summary |
|--------|----------|---------|
| Reference | [`2026-06-11-codebase-analysis.md`](analysis/2026-06-11-codebase-analysis.md) | Full analysis of the original JS game's architecture |
| Reference | [`2026-06-11-endgame-analysis.md`](analysis/2026-06-11-endgame-analysis.md) | Analysis of the endgame mod's additions |
| Reference | [`2026-06-23-feature-decomposition.md`](analysis/2026-06-23-feature-decomposition.md) | Feature-by-feature decomposition of the original (with port-progress checkboxes) |
| Reference | [`2026-06-24-vis-analysis.md`](analysis/2026-06-24-vis-analysis.md) | Analysis of the AD "Vis" variant codebase |
| Reference | [`2026-06-25-redemption-analysis.md`](analysis/2026-06-25-redemption-analysis.md) | Analysis of the AD "Redemption" variant codebase |
| Reference | [`2026-06-28-endgame-1.0-analysis.md`](analysis/2026-06-28-endgame-1.0-analysis.md) | Content analysis of Endgame v1.0 |

Raw research material lives in [`research/`](research/).
