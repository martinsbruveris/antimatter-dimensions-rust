---
date: 2026-07-06
topic: challenge tab UI fidelity fixes
design_docs:
  - ../design/2026-07-03-normal-challenges.md
  - ../design/2026-07-03-infinity-challenges.md
---

# Challenge tab UI fixes — closing small fidelity gaps

## Summary
An ongoing session of small fixes that bring the Challenges tab closer to the
original game. Each fix is recorded as its own subsection below; more will be
appended to this file as the session continues. No dedicated design doc drives
these — they are UI/behaviour parity touch-ups against the original's
`ChallengeTabHeader.vue` and friends. The challenge mechanics themselves are
covered by the linked design docs.

---

## Fix 1 — "Automatically retry challenges" toggle

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

---

## Fix 2 — Challenge box text fidelity (four small parity gaps)

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

### Correction — the wrap difference is NOT fully fixed (supersedes the C5 claim)
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

### Update — font vendored
Applied the follow-up in the same session: copied `BlobEmoji-Bold.ttf` from the
original's `public/stylesheets/` into ours (it turns out to be the
MonospaceTypewriter face plus a `COLR` color-glyph table — the exact file the
`Typewriter` `@font-face` loads first). Our font resolution now matches the
original's instead of 404-ing to the `MonospaceTypewriter.ttf` fallback, so the
bold challenge text uses the same face the original does. Rebuilt; the file is in
both `public/stylesheets/` and `dist/stylesheets/`. The user confirmed in-app
that the wrapping now matches the original across all boxes — this closes the
Fix 3 wrap issue.
