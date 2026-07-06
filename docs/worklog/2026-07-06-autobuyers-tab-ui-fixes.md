---
date: 2026-07-06
topic: autobuyers tab UI fidelity fixes
design_docs:
  - ../design/2026-07-03-autobuyers.md
---

# Autobuyers tab UI fixes — closing small fidelity gaps

## Summary
An ongoing session of small, frontend-only fixes that bring the Autobuyers tab
(as seen right after the first Big Crunch) closer to the original game. Each fix
is recorded as its own subsection below; more may be appended as the session
continues. No dedicated design doc drives these — they are UI/text parity
touch-ups against the original's `AutobuyersTab.vue`, `AutobuyerBox.vue`,
`AutobuyerIntervalLabel.vue`, `AutobuyerIntervalButton.vue`, and
`TickspeedAutobuyerBox.vue`. The autobuyer mechanics themselves are covered by
the linked design doc.

---

## Fix 1 — Hotkey link replaces the invented unlock blurb

### What shipped
The tab header no longer shows the custom line "Complete Normal Challenges to
unlock the prestige autobuyers and to upgrade autobuyer intervals with Infinity
Points." — it has no equivalent in the original. In its place, directly below the
Pause / Disable buttons, the tab now renders "Press ? to open the hotkey list.",
matching the original's `OpenModalHotkeysButton` placement in `AutobuyersTab.vue`.

- `AutobuyersTab.vue` — dropped the invented `<div>` and mounted the existing
  `components/options/OpenHotkeysButton.vue` right after `<AutobuyerToggles />`.

### Decisions & why
- **Reused the existing hotkey-link component** rather than duplicating markup.
  `OpenHotkeysButton.vue` (already used on the options tabs) carries the original's
  `c-options-tab__hotkeys-link` class and opens the same hotkeys modal as the `?`
  shortcut, so the Autobuyers tab now behaves identically.
- **The original's only unlock text here is gated on _not_ having Infinity.**
  `AutobuyersTab.vue` shows "Challenges for upgrading autobuyers are unlocked by
  reaching Infinity." under `v-if="!hasInfinity"`, so after the first Big Crunch
  there is no unlock blurb at all. Removing our invented line matches that.

---

## Fix 2 — Tickspeed "Current bulk: ×1" (and AD "×1.00" → "×1")

### What shipped
The Tickspeed autobuyer's left column now shows the "Current bulk: ×1" line it
was missing, and the shared bulk text is now "×1" rather than "×1.00" (which also
corrects the Antimatter Dimension rows).

- `TickspeedAutobuyerBox.vue` — passes `:show-bulk="entry.mode === 'single'"`.
- `AutobuyerBox.vue` — bulk text changed from "×1.00" to "×1"; the `showBulk`
  prop comment (which was backwards) rewritten to describe the real rule.

### Decisions & why
- **Show-bulk mirrors `AutobuyerIntervalLabel.isShowingBulk`** — the original
  shows the bulk line whenever the bulk is *finite* (`bulk !== 0 && isFinite`).
  For tickspeed, `hasUnlimitedBulk = (mode === BUY_MAX)`, so the line shows in
  single mode and hides in max mode; hence the `entry.mode === 'single'` gate.
  For AD autobuyers, `hasUnlimitedBulk = Achievement(61)` ("Bulked Up", all AD
  bulks at 512) — a late-game achievement, so AD stays finite (×1) here and keeps
  showing the line, which it already did.
- **"×1", not "×1.00".** The original renders bulk with `formatX(bulk, 2)`, whose
  third arg (`placesUnder1000`) defaults to 0, so `formatX(1, 2)` is "×1". Our
  hardcoded "×1.00" was wrong for both tickspeed and AD.
- **Left the text literal.** Bulk is always 1 in the current model (bulk upgrades
  and Achievement 61 aren't ported), so the value is hardcoded rather than
  threaded through the snapshot; a `bulk` field can replace it when bulk upgrades
  land. The bulk cap is 512 (< 1000), so even future values render as a plain
  "×N" with no notation dependency.

---

## Fix 3 — Interval-upgrade button text parity

### What shipped
`IntervalUpgradeButton.vue` (shared by tickspeed, all eight AD tiers, and the
prestige autobuyers) now matches the original `AutobuyerIntervalButton.vue`
exactly:

- **Upgradeable** (challenge complete, not yet minimized): a two-line button
  "40% smaller interval" / "Cost: N IP" — e.g. the 1st Dimension autobuyer now
  reads "40% smaller interval / Cost: 1 IP" instead of the single-line "Reduce
  interval: 1 IP".
- **Locked** (challenge not complete): "Complete the challenge to upgrade
  interval" — replacing the invented "Complete Normal Challenge N to upgrade
  interval".
- **Minimized** (interval at the 100 ms floor): renders nothing, matching the
  original's `v-if` / `v-else-if` (no third branch).

### Decisions & why
- **Cost uses `formatDecimal(cost, 2)`** to mirror the original's
  `format(cost, 2)` (places 2, `placesUnder1000` 0); for the current 1 IP cost
  both yield "1", but this keeps larger costs faithful.
- **"40% smaller interval" is hardcoded.** The original computes
  `formatPercents(0.4)`; the factor is the fixed `INTERVAL_UPGRADE_FACTOR = 0.6`
  (→ 40% smaller), and 40 renders as "40" in every notation, so the constant
  string is safe.

### Surprises & gotchas
- The old code showed "Interval minimized" in the maxed state; the original shows
  **nothing** there (the interval button disappears and, in the original, a
  separate bulk button can take its place once interval < 100 ms — not ported).
  The new behaviour matches the original; it isn't reachable this early, so it has
  no visible effect yet. Flagged to the user in case they'd prefer an explicit
  label.

### Tests (for Fixes 1-3)
- `npm --prefix crates/ad-gui/frontend run build` succeeds (wasm + Vite). No unit
  tests — these are static display strings / template changes; left visual
  confirmation to the user in-app.

---

## Fix 4 — Hide the prestige autobuyers until their challenge is done

### What shipped
The Dim Boost, Galaxy, and Big Crunch autobuyers are now hidden entirely until
their reveal condition is met, instead of showing an invented "Complete Normal
Challenge N to unlock" box. After the first Big Crunch (NC10–12 not yet
completed) all three are simply absent from the tab.

- `PrestigeAutobuyerBox.vue` (Dim Boost / Galaxy) — dropped the `v-else` locked
  buy-box branch; the row's `v-if="entry.is_unlocked"` is now the only output.
- `BigCrunchAutobuyerBox.vue` — same: removed the `v-else` locked buy-box.
- Both components' doc comments updated to describe the hide-when-locked rule.

### Decisions & why
- **Traced the exact reveal condition through the original.** All three boxes
  (`DimensionBoostAutobuyerBox`, `GalaxyAutobuyerBox`, `BigCrunchAutobuyerBox`)
  wrap the original `AutobuyerBox`, which renders the row on `isUnlocked ||
  isBought`, the buy-box on `canBeBought`, and **nothing** otherwise. These three
  extend `UpgradeableAutobuyerState`, which defines neither `isBought` nor
  `canBeBought` (both `undefined` → falsy) and gives them no antimatter "slow
  version" — so while locked they render nothing. Their only gate is
  `isUnlocked`, which is the relevant Normal Challenge being completed: NC10
  (Dim Boost), NC11 (Galaxy), NC12 (Big Crunch).
- **No backend change needed.** Our snapshot's `autobuyer_is_unlocked =
  is_bought || can_be_upgraded` already matches the original's `isUnlocked`: these
  prestige autobuyers keep `is_bought = false`, and `can_be_upgraded` is exactly
  the NC-completed check. Only the invented `v-else` locked display had to go.

### Surprises & gotchas
- The locked "Complete Normal Challenge N to unlock" box was **entirely our
  invention** — the original never shows a purchase/hint box for these three,
  because (unlike AD/Tickspeed) they have no antimatter unlock path. The
  `unlock_challenge` snapshot field is now unused by these boxes but kept
  (harmless, and still described in the view).

### Tests
- `npm --prefix crates/ad-gui/frontend run build` succeeds. Template-only change;
  visual confirmation left to the user in-app.

---
