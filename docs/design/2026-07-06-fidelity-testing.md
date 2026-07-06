---
status: Accepted
---

# Fidelity Testing: Save-Replay Differential Harness

**Date:** 2026-07-06

> **Status: design agreed** (2026-07-06), not yet built. The architecture and
> decisions below are settled. What remains is implementation, the empirical
> tolerance constants (§10), and the initial curated save set — which is produced
> once time-based capture is implemented (one seed save exists in `saves/`).
> Event-driven capture is a deferred phase 2 (§4).

## 1. Motivation

Today's fidelity coverage (`ad-fidelity`) is two kinds of test, both of which
compare against *values*, not against a *running engine*:

- **Analytical tests** (`tests/section*.rs`) — hand-computed expected values for
  individual formulas.
- **Fixture tests** (`tests/fixture_tests.rs` + `fixtures/pre-infinity.json`) —
  reference values produced by loading isolated JS formula files under Node's
  `vm` with everything else stubbed. This works *only* because pre-infinity lets
  ~90% of the game be stubbed to identity.

Neither exercises the whole engine ticking forward from a real game state. This
doc proposes a **record-and-replay differential test**: capture real save states
across a playthrough, load each into both engines, tick a fixed number of steps,
and diff the results. It gives broad baseline assurance that the two engines
agree; it deliberately does **not** cover UI or every edge case (targeted
scenario tests remain the tool for those).

## 2. Approach

For each captured save and each horizon `N ∈ {1, 10, 100, 1000}` ticks:

1. Load the save into the JS engine (the oracle) and into Rust.
2. Tick both forward `N` steps with an identical, injected time delta.
3. Compare a defined subset of persisted state, in log space with per-field
   tolerance.

Short horizons localize divergence; the 1/10/100/1000 ladder is chosen so that a
mismatch can be bisected to the tick where it started.

## 3. Confirmed foundations (from source)

Checked against `../antimatter-dimensions` and this repo:

- **Deterministic time-stepping works.** `gameLoop(passDiff, options)`
  (`src/game.js:422`) takes an explicit diff; only when `passDiff === undefined`
  does it derive `realDiff` from `Date.now() - player.lastUpdate`
  (`src/game.js:438`). So N fixed ticks are drivable without the wall clock.
  Caveat: the *real-time* mechanics (Ra memory, black-hole real time,
  `records.*.realTime`) still read `Date.now()` — inert for the phases ported so
  far, but late-game will additionally need `Date.now` mocked.
- **RNG is mixed, and we remove it from the comparison (see §7).** The only
  *seeded* generator is `xorshift32` (`src/core/math.js:549`) for glyph
  *generation* (`src/core/glyphs/glyph-generator.js:21`) — reproducible, and Rust
  already replicates it bit-for-bit (`GlyphRng`, `ad-core/src/glyphs.rs:192`,
  seeded from `player.reality.seed`). Replicanti growth is **unseeded and
  complex**: `replicantiLoop` (`src/core/replicanti.js:174`) dispatches across
  five sampler variants (`binomialDistribution` / `…SmallExpected` / `…BTRD`;
  `poissonDistribution` / `…SmallExpected` / `…ViaNormal` / `…PTRD`, in
  `src/core/math.js`) over **two independent RNG streams** — `Math.random` and
  the `fastRandom` IIFE (`src/core/math.js:558`). All are rejection samplers, so
  the draw *count* per sample is data-dependent. Rust does **not** reproduce
  this: `tick_replicanti` (`ad-core/src/replicanti.rs:171`) already drops the
  randomness for the closed-form expectation. So there is no seed to match and no
  reason to; §7 records the decision to make JS deterministic to match Rust.
- **Save format.** `GameSaveSerializer.serialize`
  (`src/core/storage/serializer.js:6`) is `JSON.stringify(player, jsonConverter)`
  then pako-deflate + encode. The converter maps `Infinity → "Infinity"` and
  `Set → array`. So the save *is* the whole `player` tree as JSON; Decimal fields
  serialize to strings.
- **Rust round-trips the format.** `ad-core` has a full `save/` module: import
  (`decode_save`, `save/mod.rs:42`), export (`encode_save`, `save/encode.rs:45`),
  the pako/base64 codec (`save/codec.rs`), and a **`PlayerDTO` mirroring the JS
  `player` tree** (`from_save_dto`, `save/dto.rs:843`). Save-game compatibility
  with the original is a stated project goal (see
  [`2026-06-28-save-load-analysis.md`](./2026-06-28-save-load-analysis.md)), and
  the JS→Rust importer already exists.

## 4. Decision 1 — the JS engine owns the reference saves

Saves are captured from the **original JS game**, not the Rust port. Feeding a
Rust-origin save into JS would require reconstructing a fully-valid JS `player`
(every field JS expects, many not yet populated by the port); capturing from JS
yields guaranteed-valid players, and Rust already imports them.

Capture is **manual play**, not automated. The endgame Automator does not drive
early-game progression (first Big Crunch, Normal Challenges, …), and there is no
scripted full-playthrough sequence yet (a long-term goal of `ad-sim` / the Python
bindings, but far off). So a human plays the real game and states are captured
along the way.

Practical capture mechanism (no game rebuild required):

- `window.player` and `GameStorage` are globals (`src/core/player.js:11`,
  `src/core/storage/storage.js:66`), so a **userscript / console snippet** can
  drive capture: on a timer *and* on key events, call `GameStorage.export()` and
  POST it to a tiny local server that writes sequenced files. This beats the File
  System Access API (no per-session re-grant) and auto-download (no clutter), and
  it sequences files server-side.
- **Speedup must be added** — `devtools.js` exposes a `dev` cheat object
  (`giveGlyph`, `eternify`, `dilate`, `buyAllPerks`, …), gated by
  `isDevEnvironment()`/`isLocalEnvironment()`, but **no game-speed multiplier**.
  Cheapest path without rebuilding: scale the `diff` fed to `gameLoop` (watch for
  double-ticking against the game's own loop).
- **Event-triggered capture** (first crunch, each challenge entry/completion,
  first eternity, dimboost/galaxy milestones) is more valuable than a pure time
  cadence — it guarantees the early-game boundary states we specifically want,
  which a time cadence samples poorly.

Note: the speedup method shapes *which* states are sampled (big-diff ticks differ
from many small ticks), but not the validity of the comparison — the fidelity
comparison re-ticks from each save at small diffs in both engines.

### Capture rig

The rig has two parts, both injected by a userscript (no game rebuild):
**speed-control buttons** that scale the `diff` fed to `gameLoop` so a human can
fast-forward the manual playthrough, and **save capture**.

**Start with time-based capture only** (event-driven is deferred to phase 2,
below). Export a save on a game-time cadence and POST it to a small local server.
Rationale for starting here: the discrete transitions that event-driven capture
would target are *rare early but ubiquitous later* — a galaxy happens
once-in-a-while at first but several times a second later. So once those mechanics
matter, a plain time cadence samples them densely for free; the only real gap is
the sparse *early-game* boundaries, which phase 2 fills.

Timed captures already exercise the §7 discrete-event divergence: whenever a
replay window crosses a threshold (galaxy / crunch / eternity), each engine
reaches it on its own tick, and the short horizons surface an off-by-one crossing
— the worst-case noise mode.

**Transport.** On each cadence tick, snapshot `GameStorage.export()` (synchronous
and cheap) and POST it with a light tag (timestamp + a magnitude or two). Use
`fetch(url, { keepalive: true })` rather than `sendBeacon` (whose ~64 KB cap can
be exceeded by late-game saves). The server writes sequenced files; it (or Rust)
decodes each save for the magnitudes used to pick the curated ~10–30-save subset
(§6 tiering).

#### Deferred — event-driven capture (phase 2)

Complements the timed layer by guaranteeing the sparse early-game boundaries a
time cadence samples poorly. Kept for later; design recorded here.

**Hook.** `window.EventHub` and `window.GAME_EVENT` are global
(`src/core/event-hub.js:1`, `:50`). Subscribe on the logic bus:
`EventHub.logic.on(GAME_EVENT.X, fn, TARGET)`. Handlers fire synchronously inside
each reset/milestone; the `EventHub.logic` singleton is created once
(`event-hub.js:47`) and survives in-game resets and save imports, so subscribe
once. Pass a unique `TARGET` so handlers can be removed via `offAll(TARGET)`.

**Which events** (`GAME_EVENT`, `event-hub.js:50`–`119`): first-occurrence
milestones (capture once) — `BREAK_INFINITY`, `INFINITY_DIMENSION_UNLOCKED`,
`INFINITY_CHALLENGE_COMPLETED`, `REALITY_FIRST_UNLOCKED`, `BLACK_HOLE_UNLOCKED`,
`GAME_LOAD`; recurring transitions (throttled) — `BIG_CRUNCH_*`,
`ETERNITY_RESET_*`, `REALITY_RESET_*`, pre-infinity `DIMBOOST_*` /
`GALAXY_RESET_*` / `SACRIFICE_RESET_*`, `CHALLENGE_FAILED`.

**`_BEFORE` vs `_AFTER`.** Every reset dispatches `_BEFORE` then `_AFTER` from
*inside* the transition — `bigCrunchReset` fires `BIG_CRUNCH_BEFORE`
(`big-crunch.js:56`) only after confirming `Player.canCrunch` (`:53`) and before
applying rewards/reset. So `_BEFORE` is not prediction: the transition is already
committed, and we snapshot the pre-reset `player`. `_AFTER` → post-reset state and
dynamics; `_BEFORE` → the transition/reward math in isolation (a 1-tick replay
compares the reset cleanly), with the caveat that replay only re-fires the
transition if the save's autobuyer is active — else the oracle must force it
(`bigCrunchReset(forced = true)`, `big-crunch.js:50`). Trigger *timing* is not a
`_BEFORE` concern; the timed layer already covers it (above).

**Throttle by log-magnitude**, not wall-clock or game-time: gate a recurring
capture on the driving resource having grown ≥1 order of magnitude since the last
capture of that type (crunch on `log10(antimatter)` / IP, eternity on EP). This
yields log-uniform coverage and caps volume regardless of speed.

## 5. Decision 2 — compare persisted player-tree fields

The comparison boundary is the **persisted save (the `player` tree)** — we do not
compare computed state that is not persisted. This is a deliberate, self-defining
scope. A subset of fields is selected with per-field tolerances.

Why this is sufficient for formula bugs: ticks *integrate rates into persisted
amounts*, so a rate/formula error lands in `player.dimensions.*.amount` (etc.) at
tick 1 — persisted-only comparison is not a lagging indicator for those. What it
will not see is purely-derived display/cache values, which are intentionally out
of scope.

Fields need per-type handling:

- **Decimals** serialize to strings → parse and compare in log space.
- **Set-like fields aren't uniform:** bitmasks (`achievementBits`, challenge
  completion) → exact int/bitwise; id-arrays (time studies) → order-insensitive
  set compare; glyph object-arrays → element-wise tolerant match (level/strength/
  effects are RNG-origin, ordering may differ).
- **Time fields → skip.** `Date.now`/real-time fields (`records.*.realTime`,
  `realTimePlayed`, `lastUpdate`, `backupTimer`) are skipped; both engines are
  driven with an identical injected diff instead. Game-time fields
  (`totalTimePlayed`, `this*.time`) are **also skipped** (decided): they would
  match under a fixed diff, but they are bookkeeping, not mechanics.
- **Coverage asymmetry:** Rust only populates fields for ported systems;
  unported-system fields go on the ignore-list. In practice this argues for an
  **allowlist** (compare fields we understand; fail loudly on unknown new fields)
  over a blocklist (which fails open).

### Field allowlist (first pass — for review)

The comparison allowlist over `PlayerDTO` (`ad-core/src/save/dto.rs`), grouped by
system and reviewed. **Include** = compared; **Skip** = ignored. Field names are
the JS/save keys (the comparison runs on the serialized form). Comparison mode
(§8) noted where non-obvious. General rules:

- Decimals → `log-tolerance`; ints/bools/bitmasks → `exact`; id-arrays / Sets →
  `set`; id-keyed maps → keyed `exact`.
- **Skip inputs a tick cannot mutate** (options, confirmations, editor config):
  both engines read them identically from the save, so they can't diverge —
  comparing them adds noise, not coverage.
- **Skip `Date.now`/real-time fields and values derived from a primary** (costs
  recomputed from purchase counts, rate/stat records).

**Core AM economy** — Include: `antimatter`, `dimensions.antimatter[].{amount,
bought, costBumps}`, `sacrificed`, `dimensionBoosts`, `galaxies`,
`totalTickBought`, `chall9TickspeedCostBumps`; challenge run state
`chall8TotalSacrifice`, `chall2Pow`, `chall3Pow`, `matter`.

**Infinity** — Include: `break`, `infinities`, `infinityPoints`, `infinityPower`,
`infinitiesBanked`, `partInfinityPoint` (live IP accumulator),
`dimensions.infinity[].{amount, baseAmount, isUnlocked}`, `infinityUpgrades`
[set], `infinityRebuyables` [counts], `challenge.infinity.{current,
completedBits}`. Skip: infinity-dim `cost` (derived from `baseAmount`),
`challenge.infinity.bestTimes` (real-time).

**Eternity** — Include: `eternityPoints`, `eternities`, `timeShards`,
`totalTickGained`, `timestudy.{theorem, maxTheorem, amBought, ipBought, epBought,
studies[set]}`, `dimensions.time[].{amount, bought}`, `eternityUpgrades` [set],
`epmultUpgrades`, `eternityChalls` [map], `eterc8ids`, `eterc8repl`,
`challenge.eternity.{current, unlocked, requirementBits}`. Skip: `timestudy.
presets` (config), time-dim `cost` (derived), `respec` (pending toggle).

**Replicanti** — Include: `replicanti.{unl, amount, chance, interval, galaxies,
boughtGalaxyCap}`, compared in expectation mode (§7). Skip: `chanceCost` /
`intervalCost` (derived from purchase counts).

**Dilation** — Include: `dilation.{studies[set], active, tachyonParticles,
dilatedTime, nextThreshold, baseTachyonGalaxies, totalTachyonGalaxies,
upgrades[set], rebuyables[map], lastEP}`.

**Reality (partial)** — Include: `realities`, `reality.{realityMachines, maxRM,
perkPoints, perks[set], rebuyables[map], upgradeBits, upgReqs}`,
`reality.glyphs.sac` [map]. **Glyphs are compared** — the glyph RNG is a faithful
port (`GlyphRng` reproduces `xorshift32Update` bit-for-bit, `glyphs.rs:192`; glyph
*selection* is pinned, §7), so `glyphs.{active, inventory}[].{type, level,
strength, rawLevel, effects}` should match (type/effects `exact`; level/strength/
rawLevel `log-tolerance`; matched by slot `idx`, or as a set). The glyph-RNG
cursor `reality.{seed, initialSeed, secondGaussian}` is compared too — same
xorshift/gaussian state (`secondGaussian` is the glyph RNG's Marsaglia spare,
`glyph-generator.js:69`), so a mismatch flags an RNG-fidelity bug; mostly static,
since generation only fires on an in-window Reality. Skip: glyph `id` (a
`maxID + 1` counter — deterministic but fragile bookkeeping; use `idx`/content to
match), `reality.automator.*` (program + run state), `reality.{reqLock, respec,
achTimer, autoAchieve, gainedAutoAchievements}` and `glyphs.protectedRows`
(config/timer).

**Black holes (partial)** — Include: `blackHole[].{unlocked, active,
intervalUpgrades, powerUpgrades, durationUpgrades}`. Skip: `blackHole[].{phase,
activations}` (timers), `blackHolePause`, `blackHolePauseTime`.

**Achievements** — Include: `achievementBits` [bitmask/exact].

**Requirement checks** — Include: `requirementChecks.eternity.noRG`,
`requirementChecks.reality.{noInfinities, noEternities, maxGlyphs}` — they gate
prestige unlocks and evolve during a run.

**Records** — Include the peaks that gate unlocks/formulas: `records.
totalAntimatter`, `thisInfinity.maxAM`, `thisEternity.{maxAM, maxIP}`,
`thisReality.{maxEP, maxReplicanti, maxDT}`; and the rate records
`thisInfinity.{bestIPmin, bestIPminVal}` and `thisEternity.{bestEPmin,
bestEPminVal}` (the "X-highest" auto-crunch/eternity modes read them, so they are
state). Skip: every `realTime`, all `best*` *times* (`Number.MAX_VALUE`
sentinels), game-time fields (`totalTimePlayed`, `this*.time` — decided),
`recentEternities` / `recentRealities` (bookkeeping rings), `timePlayedAtBHUnlock`,
`bestReality.{glyphLevel, glyphStrength}` (best-loadout stat).

**Options / UI (skip entirely)** — all of `options.*` (`notation`, `updateRate`,
`hiddenTabBits`, `autosaveInterval`, `awayProgress`, `showHintText`,
`confirmations`, `animations`, `automatorEvents`, `saveFileName`,
`retryChallenge`, …), plus `tutorialState` / `tutorialActive`, `tabNotifications`,
`triggeredTabNotificationBits`: inputs or pure UI a tick does not mutate.
(`retryChallenge` is gameplay-affecting but a non-diverging input, so it is
skipped like the rest.)

**Autobuyers** — Include: the `auto.*` settings — `autobuyersOn`, and per
autobuyer `isActive`, `isBought`, `mode`, `interval`, and the goal fields
(`amount`, `time`, `xHighest`, `increaseWithMult`, plus the reality autobuyer's
`rm`/`glyph`). The **Automator can change these at runtime** (mode/amount/
interval), so they are mutable game state, not fixed input, and can diverge. Skip:
autobuyer `cost` (derived from `interval`, i.e. the interval-upgrade level). Note:
whether autobuyers *run* during replay is a separate §10 decision; if they run,
their effects also land in the economy fields above.

## 6. Architecture

Two stages, decoupled — mirroring the existing `pre-infinity.json` pattern so
`cargo test` stays fast and Node-free (see the "builds and tests are fast"
project norm):

1. **Oracle generation (Node, slow, occasional).** Boot the JS engine, import a
   save, tick `N`, export. Fixtures are **JS save strings** (compact, checked in):
   an input save plus the JS-produced save after `N` ticks. Snapshot all four
   horizons from a single run rather than four runs.
2. **Rust replay (fast, no Node).** Load the input save, tick `N`, and diff
   against the expected fixture.

Because both engines round-trip the same save schema, **`PlayerDTO` is the
canonical intermediate** — it already exists, so no new normalization layer is
needed. The comparator deserializes both sides into `PlayerDTO` (JS save → serde
JSON tree; Rust `GameState` → `PlayerDTO`) and runs one generic tolerant-diff
walker keyed by a per-field type map. That is far less code than bespoke
`GameState` accessors.

**Where to compare** (the DTO/save layer) also exercises Rust's serialization, so
a mismatch *could* be an encode bug rather than a tick bug. Resolve cheaply with a
**round-trip identity guard** (`decode → encode → decode == identity`): if
round-trip is clean, any post-tick DTO mismatch is a tick bug — keeping DTO-level
ergonomics without attribution confusion.

Tiering: a small curated set (~10–30 saves) in the fast suite; the full 100–1000
set as a separate nightly/manual job.

Both stages live in the `ad-fidelity` crate: the Rust replay/comparison harness,
plus a Node oracle script (alongside the existing `js-harness/`).

## 7. Subtleties & risks

- **Discrete-event amplification** is the main noise mode — *not* FP drift. An
  autobuyer galaxy, a challenge completion, or an auto-infinity firing one tick
  apart (because a value was ~1e-12 off a threshold) diverges the whole state
  vector. Mitigations: short horizons; bisect to the first diverging tick; also
  compare **event counts** over the window (galaxies/infinities/eternities
  gained), which are more robust than raw magnitudes near a threshold.
- **Replicanti RNG** (see §3) is the strongest RNG-driven divergence from
  mid-game on. **Decision: drop the randomness — make JS deterministic to match
  Rust**, rather than synchronizing a shared RNG stream. Two reasons:
  - Rust already grows replicanti by the closed-form expectation
    (`tick_replicanti`, `ad-core/src/replicanti.rs:171`), so there is no Rust RNG
    to seed — a shared-stream scheme would mean *adding* stochastic sampling back
    into Rust.
  - JS's samplers are too complex to mirror in lockstep (five dispatch variants,
    two independent streams `Math.random` + `fastRandom`, all rejection-based),
    so a 1-ULP `log`/`exp` difference can flip an accept/reject branch, consume a
    different number of draws, and desync the stream — turning the rest of the
    run into noise. The LCG source is trivial; the exact sampler port + float
    determinism is the fragile part.

  So, under a test flag, mock the JS samplers to their means
  (`poissonDistribution = mu => mu`, `binomialDistribution = (n, p) => n * p`).
  Replicanti then consumes no RNG in either engine, needs no sequence-syncing,
  still runs JS's real batching/leftover logic minus the dice, and compares with
  log-tolerance — matching Rust by construction. **Scope the mock to the
  samplers; never mock global `Math.random`** — JS draws it for things Rust does
  not model (secret achievement 18 at `intervals.js:66`, the news ticker,
  cosmetics, `Array.randomElement` at `extensions.js:227`, glyph *selection*),
  each of which would desync a shared stream. Glyph *generation* is already
  lockstep via the seeded `xorshift32` (§3); glyph *selection*
  (`Math.floor(Math.random() * choiceCount)`) is a single draw at reality, pinned
  deterministically (force the index, or read it from the save) rather than
  stream-synced.

## 8. Comparison modes (per field / per system)

The fixture schema should make the comparison mode a first-class, per-field (and
per-system) choice:

- `exact` — ints, bools, bitmasks.
- `log-tolerance` — Decimals (log-space relative tolerance).
- `set` — id-arrays (order-insensitive).
- `tolerant-match` — glyph object-arrays (element match + tolerance).
- `expectation` — replicanti and other stochastic systems (JS sampler mocked to
  its mean; see the §7 decision).
- `event-count` — discrete events over the window.
- `ignore` — time/`Date.now` fields, unported systems, UI/options state.

## 9. Speed estimate

- **Rust: negligible.** 1000 saves × ~1111 ticks ≈ 1M ticks; at ~10–100 µs/tick
  that is seconds to a couple of minutes. Never the bottleneck.
- **JS dominates** — and only in oracle generation if the stages are decoupled.
  Boot once, reuse the context; per save ≈ import (~0.1–0.5 s) + up to 1000
  full-engine ticks (~0.1–2 ms each). Ballpark ~0.5–2.5 s/save, embarrassingly
  parallel:

  | Saves | Single-threaded | ~8 cores |
  |-------|-----------------|----------|
  | 100   | ~1–4 min        | tens of seconds |
  | 1000  | ~10–40 min      | ~2–5 min |

  With decoupling, this cost is paid only when regenerating oracles; every
  `cargo test` run replays Rust against cached fixtures and stays in the seconds
  range.

## 10. JS oracle runtime (decided) & open questions

**Decision: run the actual built game in headless Chromium, driven by
Playwright.** Shimming game/UI modules is rejected — the current `vm` +
`shims.js` loader only survives because pre-infinity lets ~90% of systems be
stubbed, and a real mid/late-game save activates them. The two realistic
alternatives were (A) the whole vue-cli bundle in a real browser, or (B) the
core-engine ES modules under jsdom. **B is not viable here without reintroducing
shimming:** the tick loop is welded to the UI — `gameLoop` calls `GameUI.update()`
every tick (`game.js:86, 412, 432, 643`), dispatches `EventHub` tick events
(`:426, :642`), and reads `Tabs` / `ui.$viewModel` (`:631`, `:996`); boot is
`init()` gated by `browserCheck()` (`main.js`; `supportedBrowser`,
`game.js:1052`). Running engine-only in jsdom would require stubbing
`GameUI`/`EventHub`/`Tabs`/`ui` (the rejected approach), and running the *whole*
bundle in jsdom would hit the canvas/WebGL/audio in `ui.js` and the Vue
components that jsdom does not implement. Headless Chromium runs the shipped
bundle with zero API gaps. The oracle step runs occasionally (not in the hot
`cargo test` path; §9), so the process/IPC overhead is acceptable, and it stays
robust as coverage grows into reality/celestials (which add canvas/WebGL).
Playwright over Puppeteer for the trace viewer (debugging the inevitable boot
breakage) and official Python bindings (matching the project's Python tooling);
cross-browser and auto-waiting are not decisive for this Chromium-only,
`evaluate`-driven job — Puppeteer would also suffice.

Determinism controls, injected via `page.addInitScript` before the bundle runs
(the same set any runtime would need — not Chromium-specific):

- **Neutralize the ambient loop** — don't let `GameIntervals` free-run (plain
  `setInterval`, `intervals.js`); drive `gameLoop(diff)` explicitly.
- **Control the clock** — override `Date.now` / `performance.now` with a counter
  advanced by exactly `diff` per tick, so `realDiff`, records, and black-hole
  real time match. Tick granularity is **50 ms** (the game's default
  `updateRate`), exposed as a harness parameter so it can change later.
- **Mock the replicanti samplers to their means** and pin glyph selection (§7).
- **Bypass `browserCheck()`** if headless Chromium's feature/UA set trips it.

Data crosses the boundary only as `GameStorage.export()` strings (§5, §6), so
Playwright's serialization boundary is a non-issue. Verify the per-tick
`GameUI.update()` does not mutate persisted state (it should be a view push);
neutralizing `requestAnimationFrame` both aids determinism and coalesces those
updates for speed.

**Replay parameters (decided).**

- **Autobuyers run during replay.** Without them little happens — autobuyers are
  what trigger the interesting mechanics (galaxies, crunches, eternities), so
  their effects land in the compared economy fields and their settings are part of
  what is tested (§5). (The Automator is not used to drive capture — manual play —
  so its run-state is not a near-term concern.)
- **Tick granularity: 50 ms**, a harness parameter (above).
- **Tolerance: parametrized, determined empirically.** Start simple — a single
  log-space epsilon — but design the comparator so tolerance can be a function of
  horizon (constant or linear in tick count) and, if needed, per field. Keep the
  shape general; fix the constants once we have data.

Open questions:

- The **initial curated save set** — pending the time-based capture
  implementation. One save currently exists in `saves/`.

(The time-based capture rig is designed — see §4, "Capture rig"; event-driven
capture is deferred to phase 2 there.)

### Deferred (low priority)

- **Test that the replicanti sampler was ported faithfully.** The broad suite
  runs in expectation mode (§7) and never exercises the stochastic sampler, so a
  bug in a faithful PTRD/BTRD port would go uncaught. A dedicated seeded-lockstep
  test (shared LCG scoped to the samplers, Rust reproducing JS's draws
  draw-for-draw) would cover it. Only worth building if replicanti *stochastics*
  ever need validating — and note Rust does not currently implement the stochastic
  sampler at all, so this presupposes that work. Low priority.
- **Event-driven capture (phase 2)** — see §4, "Deferred — event-driven capture".
  Includes the `_BEFORE` force-transition mechanism.
- **Glyph-selection pinning inside a replay window** — when a Reality triggers
  mid-replay, both engines must select the same glyph choice. Deferred; rare in a
  short horizon.

## 11. Relation to existing work

- `ad-fidelity` crate + [`README`](../../crates/ad-fidelity/README.md) — the
  current analytical + fixture tests this harness extends.
- [`2026-06-23-fidelity-test-plan.md`](./2026-06-23-fidelity-test-plan.md) —
  scenario-based pre-Infinity/Infinity plan (complementary; scenario coverage vs
  this doc's broad save-replay coverage).
- [`2026-06-23-fidelity-analysis.md`](./2026-06-23-fidelity-analysis.md) — the
  Rust-vs-JS discrepancy analysis.
- [`2026-06-28-save-load-analysis.md`](./2026-06-28-save-load-analysis.md) — the
  save codec + `PlayerDTO` this harness reuses.
- [`../PORTING.md`](../PORTING.md) — the fidelity standard (log-space relative
  tolerance).
