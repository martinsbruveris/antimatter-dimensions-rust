---
status: Reference
---

# Port Audit — Rust reimplementation vs. original Antimatter Dimensions

*Generated: 2026-07-09.* A numbered, code-level audit of what has been ported
from the original game (`../antimatter-dimensions`, Vue 2 + JS) into this project
(`ad-core` Rust engine + Tauri/Vue 3 frontend), and what remains.

This snapshot **supersedes** [`2026-07-05-port-audit.md`](2026-07-05-port-audit.md).
Since that audit, three large bodies of work landed: **all seven Celestials
(Phase 7)**, the **Imaginary Machines + 25 Imaginary Upgrades** (Feature 6.7),
and **normal-achievement wiring for rows 1–17** (Feature 2.4). The 2026-07-05
doc's "Celestials not started / achievements display-only" framing is therefore
out of date; this doc is the authoritative status snapshot.

Every feature carries a stable **ID** (`phase.feature`) so progress can be tracked
by ticking status markers over time. IDs are stable across future snapshots.

## Legend

- ✅ **Full** — present and behaves like the original (within fidelity goals).
- 🟡 **Partial** — present and usable, but with named sub-features deferred.
- 🟨 **Display-only** — UI/data exists but the mechanic is not wired to run.
- 🔲 **Not started** — no meaningful implementation.
- ⛔ **Out of scope** — deliberately excluded (real-money, cloud, cut UI modes).

## At-a-glance phase rollup

| Phase | Area | Overall |
|---|---|---|
| 0 | Foundation / number system | ✅ (break_eternity 🔲, mod-only) |
| 1 | Pre-Infinity | ✅ |
| 2 | Infinity | 🟡 |
| 3 | Infinity Dimensions & Replicanti | ✅ |
| 4 | Eternity | 🟡 |
| 5 | Dilation | ✅ |
| 6 | Reality | 🟡 |
| 7 | Celestials | 🟡 (all shipped, each with documented cuts) |
| 8 | Cross-cutting presentation & QoL | 🟡 |

---

## Phase 0 — Foundation (number system)

| ID | Feature | Status | Notes |
|---|---|---|---|
| 0.1 | `break_infinity` Decimal | ✅ | Faithful match of `break_infinity.js`: `EXP_LIMIT`/`EXP_INF_THRESHOLD` = 9e15, `MAX_VALUE` = e9e15, `NUMBER_MAX_VALUE` = 1.8e308. Uses an `i64` exponent (exact integers) vs JS's f64; same 9e15 ceiling. |
| 0.2 | `break_eternity` (tower numbers) | 🔲 | Design `Proposed` only. **No base-game mechanic requires it** — the base game (through Pelle) stays at/below e9e15. Needed only for the Endgame mod (a "Break Eternity" prestige tier, `1e(1e150)`-scale costs). |

---

## Phase 1 — Pre-Infinity ✅

| ID | Feature | Status | Notes |
|---|---|---|---|
| 1.1 | Antimatter Dimensions (8 tiers) | ✅ | |
| 1.2 | Tickspeed | ✅ | distant + remote scaling |
| 1.3 | Buy-10 / bulk buy | ✅ | AD + Tickspeed buy-max both use the closed-form `ExponentialCostScaling` (`getMaxBought`), with the NC9/IC5 loop branches |
| 1.4 | Dimension Boosts | ✅ | |
| 1.5 | Antimatter Galaxies | ✅ | distant + remote scaling |
| 1.6 | Dimensional Sacrifice | ✅ | pre- and post-IC2 formulas |
| 1.7 | Buy-10 multiplier per dimension | ✅ | |

## Phase 2 — Infinity 🟡

| ID | Feature | Status | Notes |
|---|---|---|---|
| 2.1 | Big Crunch (Infinity prestige) | ✅ | IP, infinities, records, save round-trip |
| 2.2 | Infinity Upgrades (16-grid) | ✅ | all 16 + the Ach-41 bottom row (`ipMult` rebuyable with two-regime costs + buy-max + autobuyer, `ipOffline` offline IP award) |
| 2.3 | Break Infinity + 12 upgrades | ✅ | post-break IP formula + all 12 upgrades with effects (cost-scaling rebuyables feed `dimensionMultDecrease`/`tickSpeedMultDecrease`; passive IP/Infinity generation ticks) |
| 2.4 | Achievements (normal, 144) | 🟡 | **rows 1–18 wired** (conditions + effects), including the former deferred tail (35/61/62/65/74/111/117/156/165/172 + effects 126/133/138/168/171/175/183/187) and row 18 (Pelle). Remaining: 22 (News, needs 8.1); 165's weight-equality and 171's Effarig/Reality sac types await 6.2/7.2; 172's `noTriads` carried but unclearable. Secret achievements → 8.6 |
| 2.5 | Normal Challenges (12) | ✅ | all 12 modifiers + reward→autobuyer wiring |
| 2.6 | Autobuyers | ✅ | all AD/tickspeed + prestige autobuyers, modes, interval upgrades |
| 2.7 | Infinity Challenges (8) | ✅ | all 8 restrictions + rewards |

## Phase 3 — Infinity Dimensions & Replicanti ✅

| ID | Feature | Status | Notes |
|---|---|---|---|
| 3.1 | Infinity Dimensions (8) → Infinity Power | ✅ | |
| 3.2 | Replicanti | ✅ | capped growth, RGs, 3 IP upgrades; uncapped growth via TS192 |

## Phase 4 — Eternity 🟡

| ID | Feature | Status | Notes |
|---|---|---|---|
| 4.1 | Eternity prestige | ✅ | EP formula, records, header/hotkey |
| 4.2 | Eternity Milestones (27) | ✅ | all 27 wired: keeps, autoIC/autoUnlockID, the milestone autobuyers (IP-mult, RG, buy-max Galaxies, ID 1–8, Replicanti upgrades), and the offline generators (autoEP/autoEternities/autoInfinities) |
| 4.3 | Time Dimensions (8) | ✅ | TD5–8 via dilation studies; free-tickspeed curve + softcap |
| 4.4 | Time Studies (tree) | ✅ | 58-study pre-dilation catalogue, all effects at their sites, tree UI, presets/import strings (Triad studies remain out of frontier — Ra content) |
| 4.5 | Eternity Challenges (12) | 🟡 | all 12 restrictions + rewards (EC6/EC11 feed the cost-scale knobs; EC8's ID/Replicanti budgets enforced). Remaining: EC1 Enslaved goal-1000 (needs `u16` widening, → 7.3) |
| 4.6 | Eternity Upgrades | ✅ | 3 ID + 3 TD mults + rebuyable `epMult` |

## Phase 5 — Dilation 🟡

| ID | Feature | Status | Notes |
|---|---|---|---|
| 5.1 | Time Dilation | ✅ | dilation studies 1–5, dilated run, TP/DT, Tachyon Galaxies |
| 5.2 | Dilation Upgrades | ✅ | 3 rebuyables + 7 one-time + the Pelle-only 11–15 (Doomed DT formula, TG multiplier, tickspeed power, threshold cube root, EP-based DT) |

## Phase 6 — Reality 🟡

| ID | Feature | Status | Notes |
|---|---|---|---|
| 6.1 | Reality prestige | ✅ | RM formula, full reset, records, glyph-choice modal |
| 6.2 | Glyphs | 🟡 | 5 base + Effarig + Reality types (generation rules, all 11 new effects, sacrifice/refinement), the auto-glyph filter (all 7 modes + 3 rejection modes), Teresa's undo. **Deferred: cosmetics, cursed glyphs (V), glyph sets** |
| 6.3 | Perks (35) | ✅ | tree + all effects, incl. the PEC EC-auto-completion chain (with V's `fastAutoEC` + Ra's `instantEC`) and the autobuyer-speed perks (with Teresa's `autoSpeed`); the dilation-autobuyer speed perk is inert until those autobuyers exist (Ra QoL cut) |
| 6.4 | Reality Upgrades (25) | ✅ | all 25 upgrades incl. RU13's autobuyer half (the 8 TD autobuyers + the EP-mult autobuyer + Eternity modes) and RU25's Reality autobuyer |
| 6.5 | Black Holes (2) | ✅ | unlock, 3 upgrades/hole, phase machine, pause/unpause, inversion (`blackHoleNegative` + `slowestBH` tracking, V's `achievementBH`), and the auto-pause modes (analytic BH1 / 100-step BH2 `timeToNextPause`) |
| 6.6 | Automator | ✅ | all 5 stages: lexer/parser/compiler/executor, text + block editor, templates, import/export |
| 6.7 | Imaginary Machines & Upgrades (25) | ✅ | iM currency (balance + ratcheted `iMCap` now saved), 10 rebuyables + 15 one-time with all requirements (deep ones latch via `imaginaryUpgReqs`; 22's cursed-glyph gate stays unreachable — cursed glyphs are V content) and effects (11 TD pow, 12/23 free Dimboosts, 13 cap mult, 14 `^1.5`, 22 sac fill); Teresa's `1e10000×iM` machine record; Effarig's glyph-weight adjuster landed en route |

## Phase 7 — Celestials 🟡 (all shipped since 2026-07-05, each with cuts)

| ID | Feature | Status | Notes |
|---|---|---|---|
| 7.1 | Teresa | ✅ | pour-RM → `rmMultiplier`, 6 unlocks, Teresa's Reality, 4-entry Perk Shop |
| 7.2 | Effarig | 🟡 | Relic Shards, 3-stage Reality, dilation-like nerfs, glyph-level cap, the Effarig glyph type + `maxRarityBoost` (via 6.2). **Deferred: Replicanti-cap mult / `bonusRG`** |
| 7.3 | Enslaved | 🟡 | game-time storage + release, stored-time unlocks, run restrictions. **Deferred: real-time storage + `boostReality`, auto-release/store, Tesseracts, EC1 goal-1000** |
| 7.4 | V | 🟡 | 6 main-unlock conditions, run modifiers, 9 V-achievements, Space Theorems, `fastAutoEC` (6.3) + `achievementBH` (6.5) effects. **Deferred: Perk-Point goal reduction; `autoAutoClean`; hard achievements 6–8 need Ra's flip (state exists)** |
| 7.5 | Ra + Glyph Alchemy | 🟡 | 4 pets/memories/levels, 28 unlocks, Remembrance, 21-resource Alchemy + refinement. **Deferred: charged-IU effect variants, `uncountability` passive gen (u32 realities), the Reality-resource glyph, `boundless`/`multiversal`** |
| 7.6 | Lai'tela + Dark Matter Dimensions | 🟡 | 4 DMDs, Dark Energy, Singularities (30 milestones), Continuum, entropy run. **Deferred: Continuum super-exp branch, DMD/annihilation/condense autobuyers, deep imaginary reqs, tesseract effects** |
| 7.7 | Pelle (final) | 🟡 | dooming/Armageddon, Remnants → Reality Shards, 5 Rifts, Strikes, Pelle Upgrades, Galaxy Generator, game-end. **Deferred: the full `isDisabled` disable-everything sweep, keep-on-Armageddon gates, deep rift-milestone effects, special Pelle glyph. Cut: credits/song/`zalgo` finale** |

## Phase 8 — Cross-cutting presentation & QoL 🟡

| ID | Feature | Status | Notes |
|---|---|---|---|
| 8.1 | News ticker | 🔲 | no engine corpus, no `NewsTicker` component; placeholder slot reserved |
| 8.2 | Themes | 🔲 | only default dark theme; standard + 12 secret themes absent |
| 8.3 | Statistics tab | 🔲 | placeholder; underlying `records.rs` data exists — presentation only |
| 8.4 | Records/history tabs | 🔲 | Past Prestige Runs, Challenge Records, Glyph Set Records |
| 8.5 | Notations | 🟡 | 8 of 22 (14 cosmetic missing: emoji, roman, hex, clock, …) |
| 8.6 | Secret achievements (24) | 🔲 | mostly frontend-interaction triggers |
| 8.7 | Speedrun mode + milestones | 🔲 | mode, status header, milestones tab |
| 8.8 | Options completeness | 🟡 | missing: Confirmations sub-menu, offline on/off toggle, hibernation catch-up, auto tab switching, automator log-size slider |
| 8.9 | Changelog modal | 🔲 | |
| 8.10 | Credits modal | 🟡 | `CreditsDisplay.vue` exists but minimal |
| 8.11 | Hotkeys | 🟡 | core prestige/nav/buy present; full Mousetrap long tail unverified |
| 8.12 | Tab notifications | ✅ | the yellow `!` badges |
| 8.13 | Tutorial / progressive UI reveal | ✅ | gold-glow highlight state machine |
| 8.14 | Offline / away progress | ✅ | offline + summary modals, catch-up path, away options |
| 8.15 | Save / load | ✅ | full codec; round-trips real original saves; backups/import/export |

---

## Consolidated remaining-work backlog

Ordered roughly by leverage (impact × readiness), not strict dependency.

**Near-term, high leverage (owning systems already exist):**
1. **Achievements tail (2.4)** — the deferred conditions/effects, once their
   substrates exist (recent-infinities ring, NC best-times, offline wall-clock).
2. **Statistics tab (8.3)** — presentation over already-tracked `records.rs` data.
3. **Infinity Upgrades bottom row (2.2)** — `ipMult` rebuyable + `ipOffline` + Ach 41.
4. **Break-Infinity cost-scaling knobs (2.3)** — unblock **EC6/EC11/EC8 (4.5)** and
   the parked **Time-Study effects (4.4)**; then **RU13/RU25 (6.4)** + deferred
   **Perks (6.3)**.

**Medium-term (self-contained subsystems):**
5. **News ticker (8.1)**, **Themes (8.2)**, **remaining notations (8.5)**.
6. **Options completeness (8.8)**, **records/history tabs (8.4)**.
7. **Secret achievements (8.6)** + secret themes.
8. **Speedrun mode (8.7)**.

**Celestial polish (per-celestial cuts, 7.2–7.7):**
9. Effarig/Reality glyph types (unblocks 6.2, 7.2, and Ra's `reality` resource);
   Enslaved real-time storage + `boostReality`; the celestial autobuyers; the
   Pelle `isDisabled` disable-everything sweep.

**Large / long-term:**
10. **`break_eternity` (0.2)** + **Endgame mod** support (stated long-term goal).

**Explicitly out of scope (⛔):** real-money Shop / STD, Firebase cloud saves,
Classic-UI / S12 alternate UI modes.

---

## Fidelity note

The base game reference (`../antimatter-dimensions`) imports `break_infinity.js`
exclusively — no tetration anywhere. Its largest value is the Pelle game-end
(`1e9e15`), which sits at `Decimal::MAX_VALUE`. The `i64` exponent is a faithful
match of that ceiling, not a limitation. See the number-system discussion in the
worklog and Phase 0 above.
