---
status: Accepted
feature: "7.1-7.4"
---

# Phase 7 (Celestials 1–4): Teresa, Effarig, Enslaved, V

The first four Celestials — the endgame prestige-adjacent encounters unlocked
after the first Reality. Each adds a permanently-persistent state block, a
special "Reality" run under modified rules, and progressive unlocks that feed
back into the earlier layers. This doc covers the port of the four in dependency
order (each unlocks the next) and records the **frontier cuts** — the pieces
that depend on not-yet-built systems (Ra + Alchemy, Lai'tela, Pelle, Imaginary
Upgrades, music glyphs, glyph filter/presets/undo, the hint/quote systems) and
are deferred with a neutral stub.

Original source: `../antimatter-dimensions/src/core/celestials/{teresa,effarig,
enslaved,V}.js` + `src/core/secret-formula/celestials/*`, and the run-modifier
call sites found across `dimensions/`, `tickspeed.js`, `dilation.js`,
`replicanti.js`, `glyphs/glyph-core.js`, `reality.js`, `eternity.js`,
`big-crunch.js`.

## 0. Shared model

All four live under `player.celestials.{teresa,effarig,enslaved,v}` in the
original. In the Rust engine they become a single `CelestialsState` on
`GameState` (`state.rs`), one sub-struct per celestial, serialized under a new
`celestials` field (defaulted, so old saves load). Each celestial is one module
under `crates/ad-core/src/celestials/`.

**Run flag & "in a celestial reality".** Each celestial has a `run: bool`.
`clearCelestialRuns()` clears them all (called at the start of every Reality
reset and on entering any celestial run — a celestial reality is mutually
exclusive with the others). `isInCelestialReality()` = any run flag set. Entering
a celestial run is a *manual Reality* that first clears the other runs, sets its
own flag, and runs the normal Reality reset — i.e. it routes through
`finish_process_reality`. Exiting is just the next Reality (manual or via
completing the run's objective) which clears the flag in the reset.

**Run modifiers.** The four runs transform the *final* Antimatter Dimension
multiplier (and a few other final values) at the same seam the existing Time
Dilation transform uses (`dimensions.rs::dimension_final_multiplier`, after
`applyNDPowers`). The original order (`getDimensionFinalMultiplierUncached`) is:

1. dilation compression — when `dilation.active` **or `Enslaved.isRunning`**;
2. `ndMultDT` dilation upgrade;
3. `Effarig.multiplier(x)` **or** (else if) `V` → `x^0.5`.

So Enslaved makes AD mults "always dilated", and Effarig/V replace the dilation
transform's *final* stage. These are added inline behind `self.<cel>.is_running()`
guards so normal play is untouched.

**Entry gating.** Celestial *reality* entry requires the relevant unlock bit and
`realityUnlocked`. The whole Celestials tab is gated on `Teresa.isUnlocked`
(Achievement 147 in the original — but that achievement is not in our wired set;
see §5). We gate on `reality_unlocked()` (realities ≥ 1), matching "unlocked
immediately after first Reality".

## 1. Teresa (7.1)

State (`player.celestials.teresa`): `poured_amount: f64` (RM poured, cap 1e24),
`time_poured: f64`, `unlock_bits: u32`, `run: bool`, `best_run_am: Decimal`,
`perk_shop: [u32; 5]`.

- **Pour RM** (`pourRM(diff)`): while `poured_amount < 1e24`, accumulate
  `time_poured += diff`, pour `min((poured+1e6)·0.01·time_poured², RM)` RM into
  the pool (subtract from RM). Drives `rmMultiplier = max(250·(poured/1e24)^0.1,
  1)` — a global **RM-gain** multiplier — and the unlock bits.
- **Unlocks** (bit upgrades, price in poured RM): `startEU` 1e6 (start Reality
  with all 6 Eternity Upgrades), `undo` 1e10 (glyph-undo QoL — **cut**, bit
  modelled), `run` 1e14 (unlock Teresa's Reality), `epGen` 1e18 (passive EP
  generation), `shop` 1e21 (unlock Perk Shop), `effarig` 1e24 (unlock Effarig).
- **Teresa's Reality** run modifier: base IP and EP gain → `x^0.55`; Glyph
  Time-Theorem generation disabled. Completing it (a Reality while running) sets
  `best_run_am = max(best_run_am, antimatter)`.
- **Reward** `runRewardMultiplier = max((log10(best_run_am+1)/1.5e8)^12, 1)` — a
  **glyph-sacrifice** multiplier (wired into `glyphs.rs` sacrifice).
- **Perk Shop** (Perk-Point rebuyables, `cost = initial·2^bought`, capped):
  `glyphLevel` (+5%/buy pre-instability glyph level, cap 1.05^11), `rmMult` (×2
  RM/buy, cap 2048), `bulkDilation` (dilation-autobuyer bulk — **inert**, no such
  autobuyer knob yet), `autoSpeed` (ID/TD/dilation/replicanti autobuyer ×2 speed
  — **inert** pending those autobuyers), plus `musicGlyph`/`fillMusicGlyph`
  (**cut**, music glyphs unmodelled). Modelled bits: `glyphLevel` + `rmMult` are
  live; the other three are stored and display but neutral (documented).
- **epGen**: passive EP `= 0.01 · best EP/min · dt` (mirrors the original's
  `player.reality.epmultUpgrades`-independent Teresa gen — actually
  `Teresa.rmMultiplier`? No: the original generates EP at `bestEPmin/…`). We port
  the original `generateEPForTeresa`: EP += (peak EP/min) × dt/60000 × 0.01.

## 2. Effarig (7.2)

State (`player.celestials.effarig`): `relic_shards: f64`, `unlock_bits: u32`,
`run: bool`. (`glyphWeights`/`autoAdjustGlyphWeights` are glyph-adjuster QoL —
**cut**.)

- **Relic Shards**: gained on every Reality once `TeresaUnlocks.effarig` is
  unlocked: `shardsGained = floor((log10(EP)/7500)^glyphEffectAmount) ·
  effarigAlchemy`. `glyphEffectAmount` = count of distinct effects across
  equipped generated + non-generated glyphs. Alchemy factor = 1 (Ra, cut).
- **Unlocks** (bought with relic shards): `adjuster` 1e7, `glyphFilter` 2e8,
  `setSaves` 3e9 (all three glyph-QoL — bits modelled, effects **cut**), `run`
  5e11 (Effarig's Reality). Plus three **stage** unlocks earned by *completing*
  stages of the run: `infinity` (id 4), `eternity` (id 5), `reality` (id 6).
- **Effarig's Reality** (`EFFARIG_STAGES` INFINITY→ETERNITY→REALITY→COMPLETED,
  by which stage-unlock bits are set): the run applies dilation-like nerfs —
  `multiplier(x)` compresses AD mults and `tickspeed` compresses tickspeed, both
  via `nerfFactor(power)` where `power` is Infinity Power (mults) / Time Shards
  (tick). Glyph level is **capped** (100 / 1500 / 2000 by stage). Reaching
  Infinity in-run unlocks the infinity stage (big-crunch hook); Eternity unlocks
  eternity stage; Reality completes it (reality stage). `eternityCap = 1e50` EP
  during the eternity stage. On completing a *new* stage the run exits (per the
  description "exit when you complete a Layer for the first time").
- **infinity-stage effects**: Replicanti cap ×(infinities-based), max RG +bonus,
  IP capped 1e200 + each IP mult capped 1e50 (Effarig infinity nerf, applied in
  `infinity_upgrades`/crunch). `bonusRG = floor(log10(replicantiCap)/
  LOG10_MAX_VALUE − 1)`. eternity-stage: eternities generate infinities; IP
  uncapped. reality-stage: unlocks the **Effarig glyph type**.
- **maxRarityBoost** `= 5·log10(log10(relicShards+10))` — a glyph-rarity boost
  (wired into glyph generation).
- **Effarig glyph type**: after the reality stage, glyph generation may roll an
  `effarig` glyph (one equip-limit, mutually-exclusive effects). Adding a new
  glyph type touches `glyphs.rs` generation + effects. **Scope:** we add the
  `effarig` type to the roll table gated on the reality-stage bit and its
  effects at their sites; the full "at most one equipped / mutually exclusive"
  UI nicety is minimal.

## 3. Enslaved — The Nameless Ones (7.3)

State (`player.celestials.enslaved`): `is_storing`, `stored: f64` (game time,
ms), `is_storing_real`, `stored_real: f64` (real time, ms), `auto_store_real`,
`is_auto_releasing`, `unlocks: u8` (bitset of ids 0/1), `run`, `completed`,
`tesseracts: u32`. (`hasSecretStudy`, `feltEternity`, hint/progress bits, quote
bits — flavor/QoL, **cut** except a stored `feltEternity`/`completed` for
round-trip.)

- **Time storage.** Two modes, mutually exclusive: *store game time* (the black
  hole's game-speed boost is absorbed into `stored` instead of applied) and
  *store real time* (real wall-time accumulates into `stored_real` at 70%
  efficiency, cap 8 h). Only one active at a time; both gated on unlock + not in
  a celestial run / EC12 / paused BH. **Store game time** integrates in the tick
  loop: when `is_storing_game_time`, the game-speed factor above 1 is redirected
  into `stored` rather than speeding the game.
- **Release** (`useStoredTime`): dumps `stored` as a single burst — the next
  tick's `diff` becomes `min(stored, timeCap=1e300)` ms (a massive speed spike),
  then `stored = 0`. Inside Enslaved's own Reality the release is nerfed via
  `storedTimeInsideEnslaved(x) = 1e3·10^(log10(x/1e3)^0.55)` for `x>1e3`.
- **Unlocks** (bought with `stored`, price in ms): `FREE_TICKSPEED_SOFTCAP`
  (1e35 yr → +1e5 to the Time-Dimension free-tickspeed softcap), `RUN` (1e40 yr,
  needs a level-5000 + rarity-100 glyph in `bestReality` records → Enslaved's
  Reality).
- **Enslaved's Reality** run modifiers: glyph level **min** 5000; 8th AD /
  Infinity / Time purchases limited to 1 each; AD mults **always dilated**; TS192
  (uncapped replicanti) locked; Black Hole disabled; TP & DT production ×nerf
  (`tachyonNerf = 0.3` exponent, DT `10^(log10(dt+1)^0.85−1)`); TT-from-Dilation-
  glyphs disabled; certain EC goals raised (EC1 needs 1000 completions);
  discharge nerfed. Completing a Reality in-run sets `completed = true`.
- **Tesseracts** (post-completion, bought with IP; hardcoded base-cost array,
  `cost = 10^(1e7·base)`): raise the Infinity-Dimension purchase cap. The first
  costs `10^2e7` IP — effectively unreachable in-frontier but the mechanic +
  cap-increase are modelled.
- **Cuts:** the whole hints/progress/quotes system, `feelEternity`, secret study,
  auto-release + Ra `improvedStoredTime`, reality amplification (`boostReality`,
  `canAmplify`) — flavor/Ra-gated.

## 4. V (7.4)

State (`player.celestials.v`): `unlock_bits: u32`, `run`, `run_unlocks: [u32;9]`
(tier completions per V-achievement), `goal_reduction_steps: [u32;9]`,
`st_spent: u32`, `run_records: [Decimal;9]` (best value reached per achievement).
(`runGlyphs`, `wantsFlipped`, quote bits — records/flavor, **cut**.)

- **Main unlock**: six simultaneous conditions (`mainUnlock`): 10000 realities;
  1e70 eternities; 1e160 total infinities; 1e320 DT (this-reality peak); 1e320000
  replicanti (peak); 1e60 RM. When all six `progress()≥1`, the player may
  `unlockCelestial` (sets bit 0). Progress feeds the navigation UI.
- **V's Reality** run modifier: AD mults, EP gain, IP gain, DT/s all → `x^0.5`;
  Replicanti interval **squared**.
- **V-achievements** (`runUnlocks`, 9 of them, 6 tiers each — ids 6/7/8 are
  "hard", 5 tiers, only completable when `isFlipped` = Ra-gated, so hard tiers
  never complete in-frontier). Each has a `currentValue()`, a per-tier `values[]`
  goal, and a `condition()` (usually "V.isRunning + some run context", e.g.
  EC7/EC12/EC5-dilated). `tryComplete` records the best value reached and bumps
  completions while `best ≥ goal`. Space Theorems `= Σ completions` (ids ≥6 count
  ×2). We port all 9 conditions/values; the ones needing unbuilt context (Ra,
  Imaginary) simply never fire.
- **VUnlocks** (Space-Theorem-gated rewards): `shardReduction` (2 ST → spend PP to
  reduce goals), `adPow` (5 ST → AD **power** `1+√ST/100`), `fastAutoEC` (10 ST →
  auto-EC speed, **inert** pending auto-EC), `autoAutoClean` (16 ST → glyph purge,
  **cut**), `achievementBH` (30 ST → BH power ×achievement mult), `raUnlock` (36 ST
  → −2 ST cost of studies + unlock Ra, Ra **cut** so just the ST discount).
- **Goal reduction**: spend Perk Points (via `shardReduction`) to lower each
  achievement's goal by `shardReduction(tiers)`; `nextNormalReductionCost = 1000`,
  hard = `1000·1.15^steps`.

## 5. Availability / achievement gating

The original gates Teresa on Achievement 147 (unlocked on first Reality) and
several run conditions on achievements/records we do track (glyph level/rarity in
`records.best_reality`). Since our achievement grid is display-only past row 3,
we gate the Celestials tab and Teresa on `reality_unlocked()` and derive the rest
from tracked records. This is the one deliberate divergence from the original's
exact unlock condition and is noted here + in the worklog.

## 6. Save / load

`CelestialsState` is `#[serde(default)]` on `GameState`, and the DTO layer
(`save/dto.rs`) maps it to/from `player.celestials.{teresa,effarig,enslaved,v}`
so real saves round-trip (unmodelled sub-fields — Ra/Laitela/Pelle, quote/hint
bits, glyph weights — are preserved verbatim through a passthrough or default).
The Ra/Laitela/Pelle sub-objects are kept as opaque JSON on the DTO so a real
save's celestial block survives a load→save round-trip.

## 7. UI

A new top-level **Celestials** tab (original tab id `celestials`), gated on
`reality_unlocked()`, with per-celestial subtabs (Teresa / Effarig / Enslaved /
V). Each subtab is a fresh Vue 3 rebuild of the original
`components/tabs/celestial-*` markup, vendoring its CSS, reading the snapshot and
dispatching commands. The original's animated SVG *celestial-navigation* hub is
**cut** in favour of plain subtabs (the navigation graph is ~66 KB of bespoke
SVG animation with no gameplay effect). Per-celestial UI scope:

- Teresa: pour-RM bar + pour button, the unlock list, run button, perk shop grid.
- Effarig: relic-shard header, unlock list, run button + stage indicator.
- Enslaved: time-storage panel (store/release toggles + readouts), unlock list,
  run button, tesseract button.
- V: the six main-unlock progress bars, run button, the 9 V-achievement tiles
  with tier progress, the ST/unlock reward list, goal-reduction buttons.

## 8. Deferred (frontier cuts) — summary

Ra + Alchemy, Lai'tela, Pelle (celestials 5–7); Imaginary Upgrades; music
glyphs, glyph filter/presets/undo, glyph weight adjuster (glyph QoL); the
hints/progress/quotes/celestial-quote systems; celestial-navigation SVG hub;
`feelEternity`, secret study, reality amplification, auto-release (Enslaved
flavor/Ra); auto-EC speed + auto-glyph-purge (V rewards pending their systems);
V hard-achievement completion (Ra-gated flip). Each is stored (for round-trip)
but neutral, and re-checked when its owning system lands.
