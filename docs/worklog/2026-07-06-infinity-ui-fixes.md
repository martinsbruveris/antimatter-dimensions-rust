---
date: 2026-07-06
topic: post-Infinity UI fidelity fixes (Challenges, Autobuyers, Infinity subtabs)
design_docs:
  - ../design/2026-07-03-normal-challenges.md
  - ../design/2026-07-03-infinity-challenges.md
  - ../design/2026-07-03-autobuyers.md
---

# Post-Infinity UI fixes — closing small fidelity gaps

## Summary
A session of small, mostly frontend-only fixes that bring the tabs seen right
after the first Big Crunch closer to the original game: the **Challenges** tab,
the **Autobuyers** tab, and the **Infinity** subtabs (Break Infinity). Grouped by
tab below. No dedicated design doc drives these — they are UI/text parity
touch-ups against the original's Vue components; the mechanics are covered by the
linked design docs. (This file consolidates what were three separate
same-session worklogs.)

---

# Challenges tab

Parity touch-ups against the original's `ChallengeTabHeader.vue`,
`NormalChallengesTab.vue`, and `NormalChallengeBox` and friends.

## Fix C1 — "Automatically retry challenges" toggle

### What shipped
The Challenges (Normal Challenges) subtab now has an always-visible
`Automatically retry challenges: ON/OFF` toggle in its header, mirroring the
original's `retryChallenge` option. When on, crunching — manually or via the Big
Crunch autobuyer — while inside an antimatter challenge (Normal **or** Infinity)
completes and rewards it as usual but **re-enters it fresh** instead of exiting;
when off, the crunch exits the challenge as before.

- `ad-core/options.rs` — new `Options::retry_challenge` field (default `false`,
  `serde(default)` for save compatibility).
- `ad-core/challenges.rs` — `handle_challenge_completion` now keeps the running
  challenge (Normal or Infinity) active when `retry_challenge` is set instead of
  zeroing `current`; added the `in_antimatter_challenge()` helper (mirrors
  `Player.isInAntimatterChallenge`).
- `ad-core/crunch.rs` — `big_crunch()` derives `entering_challenge =
  retry_challenge && in_antimatter_challenge()` and passes it to
  `big_crunch_reset`, so the reset starts the challenge fresh (suppressing
  skip-resets), matching the original.
- `ad-core/save/` — `retryChallenge` is encoded and decoded (`encode.rs`,
  `dto.rs`) so the option round-trips through a save.
- `ad-gui` — `retry_challenge` surfaced in `OptionsView`/snapshot, a
  `set_retry_challenge` Tauri command, a `setRetryChallenge` store action, and
  the header toggle button in `ChallengesTab.vue`.

### Decisions & why
- **Behaviour lives in the engine, not the frontend.** The original re-enters the
  challenge as a side effect of `bigCrunchReset`'s default argument
  (`enteringAntimatterChallenge = isInAntimatterChallenge && retryChallenge`) plus
  `handleChallengeCompletion` skipping the `current = 0` clear. We reproduced both
  halves in `ad-core` so the manual crunch, the crunch autobuyer, and any future
  caller all honour the option through the single `big_crunch()` path.
- **The save DTO field is required (no `serde(default)` on the DTO side).**
  `retryChallenge` is a real key in the original `player.options`, so every
  genuine save carries it; the project's convention is that modelled fields are
  required on load to surface format drift. (The `Options` struct field keeps
  `serde(default)` for the separate internal-serde path, where a missing value
  correctly defaults to the original's `false`.)

### Deviations from the design doc
- None. The linked challenge design docs don't cover the retry option; this is a
  pure parity addition against the original's `ChallengeTabHeader.vue`.

### Surprises & gotchas
- `Player.isInAntimatterChallenge` covers **both** Normal and Infinity challenges
  (but not Eternity Challenges, which use the separate `retryCelestial` option we
  don't model), so the helper checks both `challenge.current` and
  `infinity_challenge.current`.
- Exiting via the "Exit Challenge" button still exits regardless of the toggle:
  the original's `exit()` clears `current` before the reset and passes
  `enteringAntimatterChallenge = false`, which our `exit_challenge` already does.

### Tests
- `ad-core`: `retry_challenge_keeps_it_running_after_crunch` (normal),
  `retry_challenge_re_enters_ic_after_crunch` (infinity), plus a `retryChallenge`
  round-trip assertion in `valid_in_range_options_are_applied`.
- `cargo test -p ad-core --features serde` → 404 + 22 + 29 pass; clippy clean.
- `cargo build -p ad-gui` and the Vite frontend build both succeed.

## Fix C2 — Challenge box text fidelity (four small parity gaps)

Frontend-only (`ChallengesTab.vue`, `data/normalChallenges.js`); no engine change.

### What shipped
- **Restored the missing Big Crunch Autobuyer hint.** The Normal Challenges tab
  now shows the second intro line from the original's `NormalChallengesTab.vue`:
  "If you have an active Big Crunch Autobuyer, it will attempt to Crunch as soon
  as possible when reaching Infinite antimatter." (a separate `<div>`, as in the
  original).
- **Corrected Challenge 5's multiplier.** The description read `×1.1245`; the
  original renders it via `formatX(1.1245, 0, 3)`, which rounds to **`×1.125`**.
  Fixed the static string to match.
- **Locked challenges hide their description.** C10–C12 (locked until 16
  Infinities) now show `Infinity 16 times to unlock.` in place of the mechanic
  description, mirroring `NormalChallengeBox.descriptionDisplayConfig` (the
  `!isUnlocked` branch). Implemented as a per-box `description` field that swaps
  to the unlock text when `is_unlocked` is false.

### Decisions & why
- **The "different line breaks" report (the user's 3rd point) had the same root
  cause as the C5 number.** The challenge CSS and fonts are vendored verbatim from
  the original (`public/stylesheets/*`), and the box font resolves to the
  monospace `MonospaceTypewriter.ttf`. In a fixed-width box with a monospace font,
  identical text wraps identically — so a wrap difference must come from differing
  text. Auditing all 12 descriptions against the original's rendered output, the
  **only** content difference was C5's `×1.1245` (7 chars) vs `×1.125` (6 chars),
  which shifts the wrap in that one box. Fixing the number fixes the wrap; no CSS
  change was needed.
- **Descriptions stay pre-capitalized in our data** (the working tree already
  capitalized them). The original stores them lowercase and capitalizes at render
  via `DescriptionDisplay`; since our port renders the raw string, capitalizing in
  the data reproduces the same displayed text without porting that component.

### Surprises & gotchas
- The original's `formatX(value, places, placesUnder1000)` runs the *current*
  notation; for sub-1000 values that is effectively `toFixed(placesUnder1000)`.
  `(1.1245).toFixed(3)` is `"1.125"` in JS (the stored double rounds up), which is
  why the original shows `×1.125` and not `×1.124`.
- `DescriptionDisplay` renders `{{ title }} {{ description }}`, i.e. a leading
  space when `title` is empty; it collapses at the start of the line box, so it
  has no visual effect and our bare `<span>` matches.

### Tests
- `npx vite build` succeeds. No unit tests (static display strings / template).
  Left visual confirmation to the user, since reaching the Challenges tab in-app
  requires a post-Infinity save.

#### Correction — the wrap difference is NOT fully fixed (supersedes the C5 claim)
The user, looking at the running app, reported that the line breaks differ in
**C3, C4, and C7 as well** as C5 — each off by ~one word. So the "only C5's text
differs, fixing it fixes the wrap" reasoning above was wrong: those three boxes
have text identical to the original, so the cause is **font metrics, not text**.

Likely root cause found (not yet fixed, deferred at the user's request): the
original's `Typewriter` `@font-face` is
`src: url("BlobEmoji-Bold.ttf"), url("MonospaceTypewriter.ttf")`, and we are
**missing `BlobEmoji-Bold.ttf`** — it exists in the original's
`public/stylesheets/` but was never vendored into ours. Challenge boxes set
`font-weight: bold`, so the original renders them in the true-bold
`BlobEmoji-Bold.ttf`, while ours falls through to `MonospaceTypewriter.ttf` and
the browser **synthesizes bold** (faux-bold runs slightly wider), shifting the
wrap by about a word on every multi-line box. This is consistent with the
difference appearing across several boxes rather than only C5.

Follow-up: vendor `BlobEmoji-Bold.ttf` into
`crates/ad-gui/frontend/public/stylesheets/` (a one-file copy from the original)
and re-check the wrapping. Left open for a later session.

#### Update — font vendored
Applied the follow-up in the same session: copied `BlobEmoji-Bold.ttf` from the
original's `public/stylesheets/` into ours (it turns out to be the
MonospaceTypewriter face plus a `COLR` color-glyph table — the exact file the
`Typewriter` `@font-face` loads first). Our font resolution now matches the
original's instead of 404-ing to the `MonospaceTypewriter.ttf` fallback, so the
bold challenge text uses the same face the original does. Rebuilt; the file is in
both `public/stylesheets/` and `dist/stylesheets/`. The user confirmed in-app
that the wrapping now matches the original across all boxes — this closes the
Fix C2 wrap issue.

---

# Autobuyers tab

Parity touch-ups against the original's `AutobuyersTab.vue`, `AutobuyerBox.vue`,
`AutobuyerIntervalLabel.vue`, `AutobuyerIntervalButton.vue`, and
`TickspeedAutobuyerBox.vue`.

## Fix A1 — Hotkey link replaces the invented unlock blurb

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

## Fix A2 — Tickspeed "Current bulk: ×1" (and AD "×1.00" → "×1")

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

## Fix A3 — Interval-upgrade button text parity

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

### Tests (for Fixes A1–A3)
- `npm --prefix crates/ad-gui/frontend run build` succeeds (wasm + Vite). No unit
  tests — these are static display strings / template changes; left visual
  confirmation to the user in-app.

## Fix A4 — Hide the prestige autobuyers until their challenge is done

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

# Infinity subtabs

## Fix I1 — Break Infinity tab visible from the first Big Crunch

### What shipped
The Break Infinity subtab now appears from the first Big Crunch, showing the
unlock hint and a disabled BREAK INFINITY button, instead of only appearing after
Infinity is broken. Three separate gaps against the original's
`BreakInfinityTab.vue` / `BreakInfinityButton.vue` / tab config:

- `config/tabs.js` — the subtab condition was `break_infinity.unlocked`
  (= `broke_infinity`); changed to `infinity_unlocked`, matching the original's
  `condition: () => …infinityUnlocked()` (the same gate as the parent Infinity
  tab and the Replicanti subtab).
- `BreakInfinityButton.vue` — was `v-if="isVisible"` (`isUnlocked || isBroken`),
  so it didn't render pre-unlock; the original always renders it, styled
  `--unavailable` until unlockable. Removed the `v-if` and aligned the class
  logic to the original (`--available: isUnlocked`, `--unavailable: !isUnlocked`,
  `--unclickable: isBroken`).
- `BreakInfinityTab.vue` — added the "Reduce the interval of Automatic Big Crunch
  Autobuyer to 0.1 seconds to unlock Break Infinity." hint (shown while
  `!break_infinity_unlockable`), mounted `BreakInfinityButton` with the original's
  `l-break-infinity-tab__break-btn` positioning class, and gated the upgrade grid
  on `break_infinity_unlockable`.

### Decisions & why
- **Gate everything on the flags the original uses.** The original tab's
  `isUnlocked = Autobuyer.bigCrunch.hasMaxedInterval`, which is our
  `break_infinity_unlockable`; the grid appears (and the button becomes clickable)
  once the Big Crunch autobuyer's interval hits the 0.1 s floor — *not* on
  `player.break`. The tab's *visibility* is the separate `infinityUnlocked()`
  gate.
- **No backend change needed.** `break_infinity` is always present in the
  snapshot, and `break_infinity_unlockable` / `broke_infinity` / `infinity_unlocked`
  already exist — only the frontend gates were wrong.
- **Kept the tab's own IP header.** Our port has no shared "before" chrome; each
  Infinity subtab renders its own `c-infinity-tab__header` (as `InfinityDimensionsTab`
  does), so keeping it here is consistent and matches the original showing the IP
  header on every Infinity subtab via `before: "InfinityPointsHeader"`.

### Surprises & gotchas
- The 0.1 s in the hint is hardcoded (the original's `format(0.1, 1, 1)` = "0.1");
  it's the fixed 100 ms interval floor, so no notation formatting is needed.
- Enslaved's "FEEL ETERNITY" button state isn't modelled, so the button's class
  set drops the `--feel-eternity` / `!isEnslaved` cases; otherwise it matches the
  original.

### Tests
- `npm --prefix crates/ad-gui/frontend run build` succeeds. Template/config-only
  change; visual confirmation left to the user in-app.

## Fix I2 — Replicanti unlock button encompasses both lines

### What shipped
Before Replicanti is unlocked (right after the first Big Crunch), the "Unlock
Replicanti / Cost: XXX IP" button is now one big button containing both lines,
instead of a small one-line button with the cost spilling out below it.

- `ReplicantiTab.vue` — the unlock button now uses the original's
  `o-primary-btn--replicanti-unlock` class (vendored, `20rem × 8rem`) instead of
  our custom `l-replicanti-tab__unlock` (which set only `min-width`); dropped the
  now-unused `.l-replicanti-tab__unlock` scoped selector (kept `.l-replicanti-tab__galaxy`).

### Decisions & why
- **Root cause was the missing height override.** The base `.o-primary-btn` is a
  fixed `height: 2.5rem` (one line). Our custom class set only `min-width`, so the
  button stayed 2.5 rem tall and the second line (`Cost:`) overflowed *below* the
  button box. The original's `o-primary-btn--replicanti-unlock` sets an explicit
  `height: 8rem`, so both lines sit inside the button — matching it fixes the
  problem exactly.

### Tests
- `npm --prefix crates/ad-gui/frontend run build` succeeds. CSS/template-only
  change; visual confirmation left to the user in-app.

## Fix I3 — Prestige tab coloring reaches the sidebar subtab symbols

A sidebar-global fix (not an Infinity-subtab-body change), surfaced by the
Infinity tab but benefiting Eternity and Reality too.

### What shipped
The Infinity tab's subtab **symbols** in the sidebar are now orange, matching the
tab name (previously the symbols were white). The same fix colors the Eternity
subtab symbols purple and the Reality subtab symbols green.

- `Sidebar.vue` — the subtab `:class` now includes `tab.uiClass` (alongside the
  `o-subtab-btn--active` toggle), mirroring the original `ModernTabButton.vue`,
  which applies `tab.config.UIClass` to both the tab **and** each subtab.

### Decisions & why
- **Each subtab is its own `.o-tab-btn`, so it re-declares the base `color`
  (white) rather than inheriting the parent tab's prestige color.** The original
  works around this by putting the prestige `UIClass` (`o-tab-btn--infinity`
  etc.) on the subtab element itself, where `.o-tab-btn--infinity { color:
  var(--color-infinity) }` wins. Our sidebar only put it on the tab, so the tab
  *name* was already colored but the subtab symbols weren't. The fix is generic —
  it reads `tab.uiClass` — so it covers every prestige tab.

### Scope — which tabs carry a custom color
`.o-tab-btn--{infinity,eternity,reality,celestial}` set `color` (+ background,
border, hover, flyout-arrow `::after`, and tooltip color) in the vendored CSS.
Our `config/tabs.js` wires `uiClass` for **Infinity** (orange `--color-infinity`),
**Eternity** (purple `--color-eternity`), and **Reality** (green
`--color-reality`) — matching the original's `UIClass`. The original also has
`o-tab-btn--celestial` (celestials tab, not ported yet) and a separate
`newUIClass: "shop"` (the IAP shop tab, not ported); neither exists in our config,
so no other tab is custom-colored today.

### Known remaining gap
The original's `ModernTabButton` scoped style also colors the **active-tab accent
bar** per prestige (`.o-tab-btn--infinity::before { background-color:
var(--color-infinity) }`, and eternity/reality/celestial variants). We did not
port those `::before` variants into `Sidebar.vue`'s scoped style, so the active
accent bar stays the default `--color-accent` for these tabs. Not addressed here
(the request was text + symbols); noted for a later touch-up.

### Tests
- `npm --prefix crates/ad-gui/frontend run build` succeeds. Template-only change;
  visual confirmation left to the user in-app.

### Update — accent-bar `::before` variants added
Closed the "known remaining gap" above in the same session at the user's request:
added the four prestige `.o-tab-btn--{infinity,eternity,reality,celestial}::before`
`background-color` rules to `Sidebar.vue`'s scoped style (after
`.o-tab-btn--active::before`), matching the original `ModernTabButton.vue`. The
active-tab accent bar now takes the prestige colour (orange / purple / green;
celestial included for when that tab is ported) instead of the default
`--color-accent`. Rebuilt; the frontend build succeeds and the scoped CSS grew as
expected.
