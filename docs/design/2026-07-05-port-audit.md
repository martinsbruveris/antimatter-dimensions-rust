---
status: Reference
---

# Port Audit — Rust reimplementation vs. original Antimatter Dimensions

*Generated: 2026-07-05.* An independent, code-level audit of what has been ported
from the original game (`../antimatter-dimensions`, Vue 2 + JS) into this project
(`ad-core` Rust engine + Tauri/Vue 3 frontend), and what remains.

This audit was produced by reading both codebases directly and cross-checking the
claims in `2026-06-23-feature-decomposition.md` and the per-feature design docs
against the actual source. Where the code disagrees with a doc, the code wins and
the discrepancy is flagged.

## Legend

- ✅ **Full** — mechanic/UI present and behaves like the original (within the
  project's stated fidelity goals).
- 🟡 **Partial** — present and usable, but with named sub-features deferred or
  simplified.
- 🟨 **Display-only** — the UI/data exists but the mechanic is not wired to
  actually run.
- 🔲 **Not started** — no meaningful implementation.
- ⛔ **Out of scope / out of frontier** — deliberately excluded (celestial-gated,
  real-money, cloud, or explicitly cut).

"Frontier" is the project's own term (see `AGENTS.md`) for the current
implementation boundary: **everything up to and including Reality + the Automator
is built; Celestials (Phase 7) are not started.** That matches the brief.

---

## 1. Executive summary

The port is **substantially complete through Phase 6 (Reality)**. All seven major
pre-celestial phases are present at least "in some form," as expected. The engine
(`ad-core`, ~29k lines) covers Pre-Infinity → Infinity → Infinity
Dimensions/Replicanti → Eternity → Dilation → Reality, including the heavy
late-game systems (Time Study tree, Glyphs with a bit-exact seeded RNG, Perks,
Black Holes, and a full Automator with lexer/parser/compiler/executor + block
editor). The frontend (`crates/ad-gui/frontend`, 114 Vue components) mirrors the
original's tab/subtab structure and vendors its CSS.

The gaps fall into four buckets:

1. **Celestials (Phase 7)** — entirely unstarted, as expected. This is the single
   largest remaining body of work (~40% of the original endgame surface: Teresa,
   Effarig, Enslaved, V, Ra + Alchemy, Lai'tela + Dark Matter Dimensions, Pelle).
2. **Deferred slices inside "done" features** — small, well-documented cuts that
   were parked until a downstream consumer exists (e.g. Infinity Upgrades' bottom
   row, EC6/EC11 cost-growth rewards, RU13/RU25 autobuyer halves, Imaginary
   Machines).
3. **Achievements as a live system** — the full 144-achievement grid *displays*,
   but only ~rows 1–3 plus a handful of feature-specific unlocks are actually
   **wired to fire**. Most achievements are display-only today (see §6.1). This is
   the most impactful "looks done but isn't" gap.
4. **Cross-cutting presentation & QoL** — News ticker, Statistics tab, theme
   selection, 14 of 22 notations, several options toggles, and various
   records/history tabs are missing or stubbed (§5, §6).

---

## 2. Phase-by-phase mechanics audit

### Phase 1 — Pre-Infinity ✅ (complete)

| Feature | Status | Notes |
|---|---|---|
| 1.1 Antimatter Dimensions (8 tiers, production chain) | ✅ | |
| 1.2 Tickspeed | ✅ | galaxy-scaled multiplier incl. distant/remote scaling (needed by TS223/224) |
| 1.3 Buy-10 / bulk buy | 🟡 | buy-max is **repeated single buys**, not the closed-form `ExponentialCostScaling` optimisation. Correct results, but a perf/behaviour divergence at extreme buy counts. |
| 1.4 Dimension Boosts | ✅ | |
| 1.5 Antimatter Galaxies | ✅ | distant + remote scaling present |
| 1.6 Dimensional Sacrifice | ✅ | pre- and post-IC2 formulas |
| 1.7 Buy-10 multiplier per dimension | ✅ | |

### Phase 2 — Infinity ✅ / 🟡

| Feature | Status | Notes |
|---|---|---|
| 2.1 Big Crunch (Infinity prestige) | ✅ | IP, infinities, records, save round-trip |
| 2.2 Infinity Upgrades (16-grid) | 🟡 | 14 grid upgrades wired. **Bottom row deferred** (`ipMult` rebuyable + `ipOffline`, gated by Achievement 41). |
| 2.3 Break Infinity + 12 upgrades | ✅ | post-break IP formula, all 12 break upgrades |
| 2.4 Achievements | 🟨 | See §6.1 — grid displays but mostly unwired. |
| 2.5 Normal Challenges (12) | ✅ | all 12 modifiers + rewards (autobuyer unlocks) |
| 2.6 Autobuyers | ✅ | all 8 AD + tickspeed + dim-boost + galaxy + big-crunch + sacrifice + **eternity + reality** autobuyers, modes, interval upgrades |
| 2.7 Infinity Challenges (8) | ✅ | all 8 restrictions + rewards |

### Phase 3 — Infinity Dimensions & Replicanti ✅

| Feature | Status | Notes |
|---|---|---|
| 3.1 Infinity Dimensions (8) → Infinity Power (^7) | ✅ | |
| 3.2 Replicanti | ✅ | capped growth, Replicanti Galaxies, 3 IP upgrades; uncapped growth arrives with TS192 |

### Phase 4 — Eternity ✅ / 🟡

| Feature | Status | Notes |
|---|---|---|
| 4.1 Eternity prestige | ✅ | EP formula, records, header/button/hotkey |
| 4.2 Eternity Milestones | 🟡 | derived state + grid UI; milestones that unlock **not-yet-built** systems (some autobuyer types, offline generators) display as reached but their effect lands with those systems |
| 4.3 Time Dimensions (8) | ✅ | TD1–4 by milestone, TD5–8 by dilation studies; free-tickspeed curve + softcap |
| 4.4 Time Studies (tree) | 🟡 | 58-study **pre-dilation** catalogue + ~40 in-frontier effects wired; tree UI with SVG. EC study slots render as live nodes. Some effects await Break-Infinity cost knobs. |
| 4.5 Eternity Challenges (12) | 🟡 | all 12 restrictions, **10/12 rewards**; EC6/EC11 cost-growth rewards await Break-Infinity cost-scaling; EC8 budget pending |
| 4.6 Eternity Upgrades | ✅ | 3 ID mults + 3 TD mults + rebuyable `epMult` |

### Phase 5 — Dilation ✅ / 🟡

| Feature | Status | Notes |
|---|---|---|
| 5.1 Time Dilation | ✅ | dilation studies 1–5, dilated run, TP reward, passive DT, Tachyon Galaxies |
| 5.2 Dilation Upgrades | 🟡 | 3 rebuyables + 7 one-time upgrades. **Pelle-only upgrades 11–15 out of frontier.** |

### Phase 6 — Reality ✅ / 🟡

| Feature | Status | Notes |
|---|---|---|
| 6.1 Reality prestige | ✅ | RM formula, full reset, records, glyph-choice modal |
| 6.2 Glyphs | 🟡 | 5 base types, bit-exact seeded RNG, level/rarity/effects, inventory/equip/respec/sacrifice. **Out:** celestial glyph types, cosmetics, the filter, undo, alchemy refining |
| 6.3 Perks | 🟡 | 35-perk tree + effects. **Deferred:** EC-auto-completion perk + autobuyer-speed perks (await their target systems) |
| 6.4 Reality Upgrades (25) | 🟡 | all 25 rebuyable/one-time upgrades. **Deferred:** RU13/RU25 autobuyer halves. **Out:** Imaginary Machines/Upgrades (25 more, RM-hardcap gated) |
| 6.5 Black Holes (2) | 🟡 | unlock, 3 upgrades/hole, phase state machine, pause/unpause. **Out:** inversion + auto-pause (celestial-gated) |
| 6.6 Automator | ✅ | all 5 stages: AP unlock, lexer/parser/compiler, executor, text editor + docs, block editor + templates + import/export |

### Phase 7 — Celestials 🔲 (not started, as expected)

Nothing in `ad-core` implements celestial mechanics (the celestial keyword hits in
the engine are guard flags like "Pelle-disabled" that are always false today). The
full list still to do:

| Celestial | Major systems it brings |
|---|---|
| 7.1 Teresa | Teresa's Reality, IP-storage-for-RM, RM perk shop |
| 7.2 Effarig | 3-stage Reality, Relic Shards, Glyph Forge, Effarig glyph type, glyph-cap increases |
| 7.3 Enslaved (Nameless Ones) | store real/game time, release burst, Enslaved's Reality |
| 7.4 V | 36 V-achievements (6×6) + hard variants, unlock bonuses |
| 7.5 Ra | 4 pets + memories/chunks/levels, **Alchemy** (21 resources tree) |
| 7.6 Lai'tela | **Dark Matter Dimensions** (4 tiers), Dark Energy, Singularities, **Continuum**, Entropy |
| 7.7 Pelle (final) | Dooming, Remnants, Reality Shards, 5 Rifts, Pelle Upgrades, Galaxy Generator, game-ending goal |

Also unstarted and celestial-adjacent: **Imaginary Upgrades** (Phase 6 endgame,
gated behind an RM hardcap) and the celestial-navigation hub + celestial-quote
system. The long-term "endgame mod" support (per `AGENTS.md`) is beyond even this.

---

## 3. UI: tabs & subtabs

The frontend's tab registry (`config/tabs.js`) mirrors the original's structure.
Present and wired: **Dimensions, Automation (Autobuyers), Challenges (Normal /
Infinity / Eternity), Infinity (Upgrades / Infinity Dims / Break Infinity /
Replicanti), Eternity (Time Studies / Time Dims / Eternity Upgrades / Milestones /
Dilation), Reality (Glyphs / Reality Upgrades / Automator / Perks / Black Hole),
Achievements, Options (Saving / Visual / Gameplay).**

Original tabs **missing or stubbed** vs. the port:

| Original tab/subtab | Status here | Bucket |
|---|---|---|
| Statistics | 🔲 placeholder (`component: null`) | to do |
| Secret Achievements | 🔲 | to do (low priority) |
| Past Prestige Runs (records) | 🔲 | to do |
| Challenge Records | 🔲 | to do |
| Glyph Set Records | 🔲 | to do |
| Speedrun Milestones | 🔲 | to do (needs speedrun mode) |
| Shop (real-money / STD) | ⛔ | out of scope |
| Alchemy | ⛔ | Celestial (Ra) |
| Imaginary Upgrades | ⛔ | Phase-6 endgame, out of frontier |
| celestial-teresa/effarig/enslaved/v/ra/laitela/pelle/navigation (8) | 🔲 | Celestials |

---

## 4. UI: modals, headers, components

**Present** (fresh Vue 3 rebuilds): prestige confirm modals (Big Crunch, Sacrifice,
Dim Boost, Antimatter Galaxy, Eternity, Dilation, Reality) with a
"don't-ask-again" `ModalConfirmationCheck`; Big Crunch full-screen animation +
screen; offline-progress + offline-summary modals; Away-progress options; hotkeys
modal; notation modal; animation-options + info-display-options modals; hidden-tabs
("Modify Visible Tabs") modal; backup / import-save / load-game / hard-reset
modals; H2P (How-to-Play) modal; glyph tooltips + sacrifice confirm; automator
import modal + script-template modal; general tooltip; notification container;
sidebar + currency header; header prestige buttons (Big Crunch / Break Infinity /
Eternity / Reality) + challenge display.

**Missing / not rebuilt:**

- **News ticker** (`NewsTicker.vue` + ~hundreds of news lines) — 🔲 not present;
  a placeholder slot is reserved in the Visual options grid.
- **Changelog modal** — 🔲 absent (no `changelog` references in the frontend).
- **Credits** — 🟡 a `CreditsDisplay.vue` exists; the full credits modal is minimal.
- Celestial-quote history, glyph-showcase panel, singularity-milestones, Pelle
  effects, Enslaved hints, speedrun-mode modal — 🔲 (all celestial/endgame).
- Classic-UI mode (`ui-modes/classic`, `s12`) — ⛔ intentionally dropped (Modern
  UI only, per the Visual-options doc).

---

## 5. Options / settings audit

The three Options subtabs exist; several controls are **placeholder slots** kept
empty to preserve the original grid positions.

**Visual — present:** Update-rate slider, Notation picker, Exponent Notation
Options modal, Animation Options modal, Info Display Options modal, Away Progress
Options modal, Modify Visible Tabs, relative-prestige-coloring toggle, Sidebar
resource picker.
**Visual — missing:** 🔲 **Theme picker**, 🔲 **News on/off**, ⛔ Classic-UI toggle
(dropped).

**Gameplay — present:** Hotkeys on/off, Offline-ticks slider.
**Gameplay — missing:** 🔲 Offline-progress on/off toggle, 🔲 "Run suspended time as
offline" (hibernation catch-up), 🔲 Automatic tab switching, 🔲 Automator log-size
slider, 🔲 the **Confirmations** sub-menu (the centralized list of per-action
confirmation toggles — note the per-prestige confirm *modals* themselves exist,
each with a disable checkbox, so the capability is there; only the aggregated
options menu is absent).

**Saving — present (largely ✅):** save-file naming, save now, export to
clipboard/file, import from file, import-save modal, backup window modal, load-game
modal. Cloud-save (Firebase) is ⛔ out of scope for the desktop app.

---

## 6. Cross-cutting systems

### 6.1 Achievements — 🟨 the biggest "looks done, isn't" gap

- **Data & display:** all **144** normal achievements are present in
  `data/achievements.js` and render in the grid.
- **Engine wiring:** only ~**rows 1–3** plus a few feature-specific achievements
  actually **unlock** through gameplay. Distinct achievement IDs with unlock
  triggers wired in `ad-core`: roughly `{1, 8, 11–14, 18–28, 136}` — call sites
  live in `galaxy.rs`, `dimensions.rs`, `crunch.rs`, `dilation.rs`, `tick.rs`.
  There is **no global per-tick condition evaluator**; unlocks are hand-placed at
  trigger sites, and most (rows 4–18) have not been placed yet.
- **Rewards:** the aggregate `achievement_power` (`1.25^completedRows ×
  1.03^count`) is wired, and ~14 achievements' *individual* effects are applied at
  their sites; the rest of the individual reward effects are not.
- **Consequence:** because achievement multipliers feed progression, the live
  balance diverges from the original the further past the early rows you go. The
  roadmap (`2026-06-30-achievements-roadmap.md`) intends these to be added
  *per-feature*; many owning features now exist, so this is ready to close.
- **Secret achievements (24):** 🔲 not implemented (most are frontend-interaction
  triggers with no engine representation). News achievement 22 and the
  time-played achievements (76/155) also 🔲 pending their small subsystems.

### 6.2 Notations — 🟡 8 of 22

Ported: scientific, engineering, letters, standard, logarithm, infinity,
mixed-scientific, mixed-engineering. **Missing (14, the "painful"/cosmetic set):**
emoji, brackets, roman, dots, zalgo, hex, imperial, clock, prime, bar, shi, blind,
blobs, "all". Low gameplay impact; cosmetic completeness only.

### 6.3 Themes — 🔲 not implemented

No theme-selection system. The frontend renders the default dark theme; the
standard themes (Normal/Metro/Dark/Inverted/AMOLED × Metro variants) and the 12
secret themes (S1–S12, incl. the S12 alternate UI) are absent. "theme" in the
frontend only appears in vendored CSS class names and the sidebar dropdown.

### 6.4 News ticker — 🔲 not implemented

No engine module and no `NewsTicker` component; the original's news corpus
(`secret-formula/news.js`) is unported. A placeholder slot is reserved in Visual
options.

### 6.5 Statistics — 🔲 not implemented

Tab exists but renders a placeholder (`component: null`). The underlying records
infrastructure (this/best run times, recent-runs rings, totals) **does** exist in
`records.rs` and per-layer state, so a Statistics view is mostly a presentation
task over data that is already tracked.

### 6.6 Offline / away progress — ✅

Offline-progress and offline-summary modals, away-progress options, offline-ticks
slider, and the catch-up path are implemented (see `2026-06-30-offline-progress.md`).

### 6.7 Save / load — ✅ (with real-save fidelity)

Full save codec with DTO layer that round-trips **real original saves**
(`save/dto.rs` ~2k lines, `encode.rs`, hidden-tab bits, etc.). Backups + import +
export + load-game modals present. Cloud sync ⛔ out of scope.

### 6.8 Hotkeys — 🟡

Hotkeys modal + `data/shortcuts.js` + a shortcuts utility exist and cover the
core prestige/navigation/buy hotkeys. Full parity with the original Mousetrap
keymap (every letter/number/modifier binding) is not verified here; treat as
"core hotkeys present, long tail unverified."

### 6.9 Speedrun mode — 🔲 not implemented

No speedrun mode, speedrun status header, or speedrun-milestones tab.

---

## 7. Consolidated to-do backlog

Ordered roughly by leverage (impact × readiness), not strict dependency.

**Near-term, high leverage (owning features already exist):**
1. **Wire achievement unlock conditions rows 4–18** (+ individual reward effects).
   Highest-impact correctness gap; most owning systems are built. (§6.1)
2. **Statistics tab** — presentation over already-tracked `records.rs` data. (§6.5)
3. **Infinity Upgrades bottom row** (`ipMult` rebuyable + `ipOffline`) + Ach 41
   gate. (§2, Phase 2.2)
4. **EC6/EC11 rewards + EC8 budgets** once Break-Infinity cost-scaling knobs land;
   **RU13/RU25** autobuyer halves; deferred **Perks** (EC-auto, autobuyer-speed).

**Medium-term (self-contained subsystems):**
5. **News ticker** (engine corpus + `NewsTicker` component + Visual toggle).
6. **Theme selection** (standard themes; secret themes optional).
7. **Remaining notations** (14 cosmetic).
8. **Options completeness** — Confirmations sub-menu, offline toggle, hibernation
   catch-up, auto-tab-switching, automator log-size.
9. **Records/history tabs** — Past Prestige Runs, Challenge Records, Glyph Set
   Records.
10. **Secret achievements** (24, mostly frontend triggers) + secret themes.
11. **Speedrun mode** + milestones.

**Large / long-term (new phases):**
12. **Imaginary Machines / Upgrades** (Phase-6 endgame, RM-hardcap gated).
13. **Celestials, Phase 7** — Teresa → Effarig → Enslaved → V → Ra (+Alchemy) →
    Lai'tela (+Dark Matter Dimensions, Continuum, Singularities, Entropy) → Pelle
    (Rifts, Remnants, Galaxy Generator). The bulk of remaining work.
14. **Endgame mod** support (stated long-term goal).

**Explicitly out of scope:** real-money Shop / STD, Firebase cloud saves,
Classic-UI / S12 alternate UI modes.

---

## 8. Fidelity notes / discrepancies found

- `feature-decomposition.md` §4.3 says TD5–8 unlock via time studies 71–74; the
  code (correctly, matching the original) unlocks them via **Dilation studies
  2–5**. The doc already carries this correction inline.
- Several `feature-decomposition.md` example tables (IP/EP costs, EC goals) are
  approximate; the engine follows the real game data. Not defects — the doc says so.
- The Infinity-Upgrade grid prerequisite is **per-column vertical chains**, not
  "previous column"; the code is correct (the doc notes this).
- Achievements are the one place where the top-level status marker (✅ in the
  decomposition) overstates the live state: the *system* is built and rows 1–3
  work, but the **grid is mostly display-only** today (§6.1). Recommend re-marking
  Feature 2.4 as 🟡/partial until row 4+ conditions are wired.
