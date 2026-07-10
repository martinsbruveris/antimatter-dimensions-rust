# ad-core architecture

`ad-core` is the game engine — the rules. Pure logic, no IO. It owns `GameState`
(all mutable state), the `Action` IR plus the `apply_action` mutation seam that
every action producer (GUI, autobuyers, simulation) routes through, the tick
loop, and the `data` module of static config. It never depends on `ad-sim`.

This is a **living** file map: keep it in sync with the code. Each entry links
the design doc that introduced the system (historical — read it for the *why*,
not the current state).

## Key source files

- `src/state.rs` — `GameState` struct (all mutable game state)
- `src/action.rs` — `Action` IR + `GameState::apply_action`: the single mutation
  seam every action producer (GUI, autobuyers, simulation) routes through
- `src/tick.rs` — Main game loop (`tick()` and `simulate()`)
- `src/dimensions.rs` — Dimension purchasing, production, multipliers
- `src/tickspeed.rs` — Tickspeed upgrades and effects
- `src/galaxy.rs` — Antimatter galaxy purchases; Dimension Boosts
  (`can_dim_boost`, `buy_dim_boost`, and the `autobuyMaxDimboosts` buy-max path
  `max_buy_dim_boosts`)
- `src/sacrifice.rs` — Dimension sacrifice (`sacrifice` mirrors `sacrificeReset`:
  the `chall8TotalSacrifice *= nextBoost` product runs for every sacrifice, not
  only NC8) + the Sacrifice autobuyer gate (`sacrifice_autobuyer_unlocked`)
- `src/crunch.rs` — Big Crunch (Infinity): `can_big_crunch`, `big_crunch`, and the
  shared `big_crunch_reset(forced, entering_challenge)` that both the manual crunch
  and the challenge enter/exit route through. Awards Infinity Points
  (`gained_infinity_points`, pre-break = 1), Infinities (`gained_infinities`), and
  challenge completion only when at the goal; updates the fastest-infinity record; IP
  / infinities / total-time-played persist across the reset. See
  `../../docs/design/2026-07-02-infinity-points-and-records.md`.
- `src/challenges.rs` — Normal Challenges (Feature 2.5): `NormalChallengeState` on
  `GameState` (`current` + `completed` bitmask), unlock/start/exit/complete logic,
  the reward wiring (completing NC1–9 unlocks the AD/Tickspeed autobuyers), the
  "Automatically retry challenges" behavior (`options.retry_challenge`: a crunch
  inside an antimatter challenge re-enters it — `handle_challenge_completion` keeps
  it active and `big_crunch` starts it fresh), the
  per-run accumulator reset (`reset_challenge_stuff`), and the NC-specific helpers
  (`max_dimensions_unlockable`, `max_boosts`, the NC9 same-cost bumps). **All 12
  modifiers are implemented**, each applied inline at its engine site via
  `challenge_running(N)` (so normal play is untouched): NC2/NC3/NC11 tick-state in
  `tick.rs::update_challenges` (`chall2_pow`/`chall3_pow`/`matter`); NC5/NC6/NC9 in
  the cost/tickspeed paths; NC4/NC12 in the dimension buy/production paths; NC7/NC8
  in `infinity_upgrades.rs` (buy-10 / dim-boost power) and `sacrifice.rs`; NC8/NC10
  in `galaxy.rs` and `sacrifice.rs`. See
  `../../docs/design/2026-07-03-normal-challenges.md`.
- `src/records.rs` — `Records`: the modelled slice of `player.records` (total time
  played, this-infinity time/`maxAM`, best-infinity time). Advanced in `tick`; the
  current-infinity records reset on a Big Crunch.
- `src/infinity_upgrades.rs` — Infinity Upgrades (Feature 2.2): the `InfinityUpgrade`
  enum + data table (cost, save-id, column prerequisite), purchase logic
  (`buy_infinity_upgrade`, IP-gated bitmask on `GameState::infinity_upgrades`), and
  the effect readers other modules call (`buy_ten_multiplier`, `dim_boost_power`,
  `galaxy_strength_effect`, `reset_boost_reduction`, the AD-multiplier
  contributions, `skip_resets_if_possible`, passive `generate_passive_ip`). Effects
  are *applied* at the original's sites (dimension multiplier, tickspeed, boost/
  galaxy requirement, reset paths). Also the Achievement-41 **bottom row**: the
  `ipMult` rebuyable (`InfinityIPMultUpgrade`'s two-regime cost curve — ×10 steps
  to 1e3M then ×1e10 to the 1e6M cap — with the two-phase geometric-series
  `buy_max_ip_mult` and the Big-Crunch-autobuyer amount bump) and the one-time
  `ipOffline` (its award fires in `offline_currency_gain`, tick.rs; the IP-mult
  autobuyer ticks in autobuyers.rs, gated on the 1-Eternity milestone). See
  `../../docs/design/2026-07-03-infinity-upgrades.md`.
- `src/break_infinity_upgrades.rs` — Break Infinity + its 12 upgrades (Feature 2.3).
  `GameState::broke_infinity` (↔ `player.break`) lifts the `1e308` cap and switches
  `gained_infinity_points` to the scaling formula (both in `crunch.rs` / `tick.rs`);
  `break_infinity()` is gated on `break_infinity_unlockable()` (Big Crunch autobuyer
  maxed). This module owns the `BreakInfinityUpgrade` (9 one-time, sharing the save's
  `infinityUpgrades`) + `BreakInfinityRebuyable` (3, in `infinityRebuyables`) types,
  purchase logic, and the effect readers (`break_infinity_upgrade_common_mult`,
  `break_infinity_galaxy_boost`, `break_infinity_autobuyer_speedup`,
  `is_buy_max_dimboosts_unlocked`); every effect is wired (the cost-scaling
  rebuyables feed `dimension_mult_decrease`/`tickspeed_mult_decrease`, the
  passive IP/Infinity generators tick in `tick.rs`). See
  `../../docs/design/2026-07-03-break-infinity.md`.
- `src/replicanti.rs` — Replicanti (Feature 3.2): `ReplicantiState` on `GameState`,
  unlocked with IP (`unlock_replicanti`), grown each tick (`tick_replicanti`, the
  capped continuous approximation), and spent on Replicanti Galaxies
  (`buy_replicanti_galaxy` → an antimatter-galaxy-like reset). RGs feed the tickspeed
  formula via `effective_galaxies()` (used in `tickspeed.rs`); `replicanti_mult` is
  folded into `id_common_multiplier` (`infinity_dimensions.rs`). Three IP upgrades
  (chance / interval / galaxy cap). Persists across a Big Crunch. See
  `../../docs/design/2026-07-03-replicanti.md`.
- `src/eternity.rs` — Eternity (Feature 4.1): the second prestige.
  `eternity_goal`/`can_eternity` (peak IP this eternity, or the running EC's
  scaled goal), the EP formula (`5^(log10(maxIP + pending crunch IP)/308 − 0.7)
  × totalEPMult`), `eternity()` rewards (EP, eternities, TS191 banked
  infinities, the recent-eternities ring) and the layered reset —
  `eternity_full_reset` (autobuyers/break + respec) over `eternity_reset_core`
  (shared with `startEternityChallenge`), with the Eternity-Milestone keeps.
  Also `update_prestige_rates` (bestIP/EPmin).
- `src/eternity_milestones.rs` — Eternity Milestones (Feature 4.2): the
  27-milestone catalogue (pure derived state, `eternities >= threshold`);
  per-tick autoIC/autoUnlockID hooks; unlockAllND/replicantiNoReset are read at
  their sites (state.rs / replicanti.rs), the reset keeps in eternity.rs. The
  milestone autobuyers live in autobuyers.rs (`MilestoneAutobuyer`: ID 1–8 at
  11–18 Eternities, Replicanti upgrades at 50/60/80, the RG toggle at 3, the
  IP-mult buyer at 1, buy-max Galaxies at 9 via `max_buy_galaxies`); the
  offline generators (`autoEP`/`autoEternities`/`autoInfinities`, via
  `auto_eternities_available`/`auto_infinities_available`) fire from
  `offline_currency_gain` (tick.rs).
- `src/time_dimensions.rs` — Time Dimensions (Feature 4.3): 8 EP-bought tiers
  (TD5–8 await Dilation), the threshold/e6000 cost curve, production chain →
  Time Shards → free Tickspeed upgrades (`FreeTickspeed.fromShards` port with
  the 300k softcap + Newton inversion). Made tickspeed a `Decimal`
  (`current_tickspeed_ms`).
- `src/time_studies.rs` — Time Studies (Feature 4.4): Time Theorems (AM/IP/EP
  purchases gated on owning a TD), the 58-study pre-dilation catalogue with the
  original structural rules (Dimension split + TS201, exclusive Pace columns,
  Light/Dark pairs, EC-gated specials), respec, and the preset slots + study
  import strings. All 58 study effects live at their engine sites (each site
  names its study); Triad studies (Ra) are out of frontier.
- `src/eternity_challenges.rs` — Eternity Challenges (Feature 4.5): EC study
  slots (TT cost + secondary requirements + requirementBits waivers),
  start/exit/complete flow through the Eternity reset, scaled goals (×5
  completions), EC4/EC12 restriction failures, the EC12 game-speed factor,
  and the restriction/reward effect readers consumed across the engine.
- `src/dilation.rs` — Time Dilation (Features 5.1 + 5.2): dilation studies
  (the real unlock gate + TD5–8), the dilated-run flow through the Eternity
  reset (`dilatedValueOf` compression applied to the final AD/ID/TD
  multipliers and the tickspeed interval), Tachyon Particles / Dilated Time /
  Tachyon Galaxies (threshold crossings, free galaxies), and the Dilation
  Upgrades — 3 rebuyables + 7 one-time plus the Pelle-only 11–15
  (`pelle_rebuyables`; the Doomed branch of `dilation_gain_per_second`, the
  TG multiplier in `update_tachyon_galaxies`, the tickspeed power in
  tickspeed.rs, the threshold cube root) — with effects at the usual sites.
- `src/eternity_upgrades.rs` — Eternity Upgrades (Feature 4.6): the 6 one-time
  EP upgrades (ID mults from EP/eternities/IC record times — with per-IC
  best-time records written on completion in crunch.rs — TD mults from
  achievements/TT/days played) and the rebuyable ×5 `epMult` feeding
  `totalEPMult`.
- `src/reality.rs` — Reality (Feature 6.1): `RealityState` (RM, realities,
  Perk Points, the glyph RNG seeds, Reality-Upgrade bits, auto-achievement
  machinery) + `RequirementChecks`, the RM formula (`uncapped_rm` with the
  pre-first-reality softcap), `gained_glyph_level` (EP/replicanti/DT records →
  instability softcaps), `reality()`/`reset_reality()` and the full
  `finishProcessReality` port, achievement locking + `tick_auto_achievements`.
- `src/glyphs.rs` — Glyphs (Feature 6.2): the JS-faithful seeded `GlyphRng`
  (xorshift32 + Marsaglia spare, ToInt32 seed semantics; outputs verified
  bit-for-bit against the original's algorithm), generation (strength/effect
  rolls with the Effarig rarity boosts, the early-reality uniformity code, the
  uncommon guarantee, the Effarig type's 7-effect pool with the rm/glyph
  exclusion), the 120-slot inventory + equip/respec (one Effarig glyph max),
  sacrifice (RU19-gated, 7 type boosts incl. effarig rarity + reality memory
  chunks), the 27 generated + 4 Reality-glyph effects' combiners (applied at
  their engine sites), Reality-glyph creation from the reality Alchemy
  resource, the auto-glyph filter (`AutoGlyphProcessor`: 7 score modes, 3
  rejection modes, per-type configs — consumed by `auto_reality`), and
  Teresa's Glyph undo (equip snapshots + the rewinding reset).
- `src/perks.rs` — Perks (Feature 6.3): the 35-perk catalogue + connection
  graph, purchase (1 PP, adjacency), on-purchase side effects (START bumps,
  EU1, ACHNR), `starting_ip`/`starting_ep`, and `tick_perk_effects` (EU
  auto-grants, auto TT-gen/TD/Reality-study unlocks). Effects live at their
  sites; the PEC perks drive `ec_auto_complete_tick`
  (eternity_challenges.rs, with V's `fastAutoEC` and Ra's `instantEC`), and
  `perk_autobuyer_faster` shrinks the ID/Replicanti autobuyer intervals
  (with Teresa's `autoSpeed` Perk-Shop effect).
- `src/reality_upgrades.rs` — Reality Upgrades (Feature 6.4): 5 rebuyable
  Amplifiers (the original's hybrid linear cost scaling, with the Imaginary
  Intensifier base adds and the `realityrow1pow` Reality-glyph power) + 20
  one-time upgrades with `upgReqs` requirement tracking checked at the
  original's events, `applyRUPG10`, RU11/RU14 continuous generation. RU13
  unlocks the TD + EP-mult autobuyers and the Eternity autobuyer modes;
  RU25 the Reality autobuyer (all in autobuyers.rs).
- `src/black_holes.rs` — Black Holes (Feature 6.5): both holes' state machine
  (BH2's phase advances only while BH1 is active), interval/power/duration
  upgrades, pause + the 5 s unpause power ramp, the inversion
  (`blackHoleNegative` slows the game while paused; `slowestBH` tracked for
  Imaginary Upgrade 24), the auto-pause modes (`timeToNextPause`: analytic
  for BH1, the bounded transition scan for BH2), and the game-speed factor
  consumed by `game_speed_factor` (stacked with the `timespeed` glyph and
  V's `achievementBH` reward).
- `src/automator/` — The Automator (Feature 6.6, all five stages): `mod.rs`
  (script/constant storage + limits, AP unlock at 100), `lexer.rs` +
  `parser.rs` + `compile.rs` (hand-written line-oriented scanner,
  recursive-descent parser with per-line error recovery, game-state-aware
  validation with the original's error text), `program.rs` + `exec.rs` (the
  instruction set and the stack machine: interval
  `max(0.994^realities × 500, 1)` ms, ≤100 commands/update, save-resume by
  line re-matching), `blocks.rs` (text → block-editor structures),
  `templates.rs` (the five script generators + warnings), `transfer.rs`
  (serde-gated import/export text codec). See
  `../../docs/design/2026-07-05-automator.md`.
- `src/celestials/` — Celestials (Phase 7). `mod.rs` owns `CelestialsState`
  (`player.celestials`) + the shared run machinery: the mutually-exclusive
  per-celestial `run` flags, `is_in_celestial_reality`, `clear_celestial_runs`
  (called from `reality_reset_internal`), `start_celestial_reality` (a reward-
  free Reality that sets one run flag), and `celestial_reality_completion_hooks`
  (run from `finish_process_reality`). `teresa.rs` (Feature 7.1): pour-RM pool +
  `rmMultiplier` (into `reality_machine_multiplier` → `uncapped_rm`), the 6
  threshold unlocks (`startEU` grants the 6 EUs on reset), Teresa's Reality
  (IP/EP `^0.55` in `crunch.rs`/`eternity.rs`, glyph-TT-gen off in `dilation.rs`),
  the `runRewardMultiplier` glyph-sacrifice bonus (`glyphs.rs`), passive `epGen`
  (`tick.rs`), and the 4-entry Perk Shop. `effarig.rs` (Feature 7.2): Relic
  Shards + the 4 shard unlocks, the 3-stage run (Infinity/Eternity/Reality) with
  its prestige-hook unlocks + reward-free exits (`crunch.rs`/`eternity.rs`), the
  dilation-like nerfs — `effarig_multiplier` (AD final mult, `dimensions.rs`),
  `effarig_tickspeed` (`tickspeed.rs`), the glyph-level cap via
  `adjusted_glyph_level` (`glyphs.rs`) — and the Infinity-stage IP handling
  (`total_ip_mult` → 1 + base cap 1e200, `crunch.rs`); the persistent glyph/
  replicanti rewards (Effarig glyph type, `maxRarityBoost`, cap mult/`bonusRG`)
  are deferred. `enslaved.rs` (Feature 7.3): game-time storage (bank the
  Black-Hole boost via `enslaved_apply_time_flow` at the top of `tick`) + release
  burst, the 2 stored-time unlocks (softcap → `free_tickspeed_softcap`; run,
  glyph-gated), and the run restrictions — glyph-level min (`adjusted_glyph_level`),
  always-dilated AD (`dimensions.rs`), 8th-AD/ID/TD purchase caps
  (`state.rs`/`infinity_dimensions.rs`/`time_dimensions.rs`), TS192 lock
  (`replicanti.rs`), disabled Black Hole + Effarig game-speed nerf
  (`game_speed_factor`), TP/DT nerfs (`dilation.rs`), the discharge nerf; EC1
  goal-1000 (needs a u16 completion widening), real-time storage/amplification,
  and Tesseracts' effect are deferred. `v.rs` (Feature 7.4): the six main-unlock
  conditions + `v_unlock_celestial`, V's Reality run modifiers (AD/EP/IP/DT `^0.5`
  across `dimensions.rs`/`crunch.rs`/`eternity.rs`/`dilation.rs`, squared
  Replicanti interval in `replicanti.rs`), the 9 V-achievements
  (`v_check_for_unlocks` in `tick.rs` runs `tryComplete`; hard ids 6–8 need Ra's
  flip so never complete), Space Theorems, and the `adPow` AD power
  (`dimensions.rs`). The Perk-Point goal reduction and the fastAutoEC/
  autoAutoClean/achievementBH/raUnlock reward effects are deferred. See
  `../../docs/design/2026-07-06-celestials.md`. `ra.rs` + `alchemy.rs`
  (Feature 7.5): Ra's four Celestial-Memory pets (memories/chunks/levels/
  upgrades, `ra_memory_tick` from real time in `tick.rs`), the 28 unlocks +
  effect readers, Remembrance, the charged-Infinity-Upgrade count gate +
  discharge (state only; the charged effect *variants* deferred), momentum +
  peak-game-speed tracking, and Ra's Reality. Effects wired at their sites:
  `continuousTTBoost` (replicanti/dilated-time/TT-gen/infinities/eternities),
  `achievementPower` (`^1.5` in `achievements.rs`), `achievementTTMult`,
  `unlockHardV` (→ `v_is_flipped`), `relicShardGlyphLevelBoost` (glyph level in
  `reality.rs`). `alchemy.rs`: the 21 resources, caps, the reaction engine
  (`apply_alchemy_reactions` per rewarded Reality in `reality.rs`), Glyph
  *refinement* wired into `glyphs.rs::sacrifice_glyph`, and the effect readers
  (power/infinity/time → AD/ID/TD power; replication; dilation; dimensionality;
  effarig; momentum; force; exponential IP; inflation; synergism/
  unpredictability/decoherence internal to reactions). Deferred: the `reality`
  resource's Reality Glyph, `uncountability` passive generation (u32 realities),
  `boundless`/`multiversal` (inert targets), and the QoL/automation unlocks. See
  `../../docs/design/2026-07-07-ra.md`. `laitela.rs` + `singularity.rs` +
  `imaginary_upgrades.rs` (Feature 7.6): Lai'tela's `LaitelaState` — the 4 Dark
  Matter Dimensions (`dmd_tick` real-time DM/DE production in `tick.rs`,
  interval/powerDM/powerDE upgrades, ascension), annihilation, Dark Energy →
  Singularities (`singularity.rs`, the 30-milestone catalogue + `completions`/
  effect readers), Continuum (`ad_continuum_value`/`tickspeed_continuum_value`
  into the buy-10 seams — linear-branch approximation), and the entropy
  destabilization run (`laitela_reality_tick`, `maxAllowedDimension` disabling
  top AD/ID/TD tiers). Milestone effects wired: DMD-internal (dark mult/interval/
  cost/ascension), plus `gamespeedFromSingularities` (`game_speed_factor`),
  `glyphLevelFromSingularities` (`reality.rs`). `imaginary_upgrades.rs`: Imaginary
  Machines (the ratcheted `iMCap` + balance, both saved; approach the cap ×
  iU13's multiplier), the 10 rebuyables + 15 one-time upgrades — all
  requirements wired (the deep 11–14/22–24 latch into `imaginaryUpgReqs` at
  their tick/Reality events; 22's cursed-glyph gate stays unreachable) and
  effects (iU8 ID mult, iU10 singularity gain, iU11 TD pow, iU12+iU23 free
  Dimboosts via `total_dim_boosts`, iU13 cap mult, iU14 per-purchase `^1.5`,
  iU15/19/21 Lai'tela wiring, iU22 sacrifice fill). Effarig's glyph-weight
  adjuster (`glyph_weights` in `getGlyphLevelInputs`) landed with iU12's
  requirement. Deferred: the Continuum super-exponential branch, the
  DMD/annihilation/condense autobuyers, and the tesseract-linked effects.
  iU25 (Pelle unlock) lands with 7.7. See `../../docs/design/2026-07-07-laitela.md`. `pelle.rs` (Feature 7.7):
  Pelle's `PelleState` — dooming (`doom_reality`, gated on Imaginary Upgrade 25) +
  Armageddon, Remnants (`remnants_gain` from the doomed records) → Reality Shards
  (`pelle_tick` in `tick.rs`), the 5 Rifts (fill/percentage/effect/milestones,
  drained from IP/Replicanti/EP/DT), Strikes (`pelle_trigger_strike` from crunch/
  eternity/galaxy/dilate + the 115-TT check), Pelle Upgrades (5 rebuyable + 23
  one-time), the Galaxy Generator (`galaxy_generator_loop`, phases + sacrifice),
  and the antimatter game-end (`game_end_state` → `is_game_end`). Effects wired:
  the doomed `antimatterDimensionMult` + `timeSpeedMult` rebuyables, the Infinity-
  Strike AD `^0.5` penalty, and the Paradox all-dim power. **The full `isDisabled`
  disable-everything sweep is a documented cut** (`pelle_is_disabled` is a query;
  only a subset of sites consult it); the credits/song/`zalgo` finale is cut. See
  `../../docs/design/2026-07-07-pelle.md`.
- `src/achievements.rs` — Normal achievements: `achievement_bits` bitmask helpers
  (`achievement_unlocked`/`unlock_achievement`), the global `achievement_power`
  multiplier, `starting_antimatter`, and the `achievement_ad_common_mult` term.
  Rows 1–18 are wired: unlock conditions live in per-event `check_*_achievements`
  dispatchers (`check_tick_achievements`, `check_crunch_before/after`,
  `check_galaxy_before/after`, `check_sacrifice_before/after`,
  `check_eternity_before/after`, `check_reality_before/after`,
  `check_infinity_challenge_completed`, `check_perk_bought`,
  `check_reality_upgrade_bought`, `check_black_hole_unlocked/upgrade`,
  `check_singularity_before/after`, `check_annihilation`,
  `check_challenge_failed`) called at the matching action seam; effects live at
  their consumption sites across the engine. `IMPLEMENTED_ACHIEVEMENTS` lists the
  naturally-awardable ids — every row (1–18, including Pelle) except 22 (News,
  needs Feature 8.1); its doc comment records the documented approximations
  (35's offline seam, 165's always-equal weights, 171's basic-only sacrifice
  types, 172's unclearable `noTriads`). See
  `../../docs/design/2026-06-30-achievements.md` and the
  `2026-07-09-normal-achievements-wiring` worklog.
- `src/tab_notifications.rs` — Tab notification badges: the pulsing yellow `!`
  on tab/subtab buttons pointing at newly relevant content. `tab_notifications`
  (the badged `tabKey + subtabKey` strings, ↔ `player.tabNotifications`) +
  `triggered_tab_notification_bits` on `GameState`; the modelled
  `TabNotificationId`s (firstInfinity / breakInfinity / ICUnlock / replicanti /
  newAutobuyer) fire inline from `big_crunch_reset`, `break_infinity`,
  `upgrade_autobuyer_interval`, and the per-tick IC-unlock/affordable-autobuyer
  checks; the frontend acknowledges a viewed tab via `tab_notification_seen`.
  See `../../docs/design/2026-07-04-tab-notifications.md`.
- `src/tutorial.rs` — Tutorial-highlight state machine (`tutorial_state` /
  `tutorial_active`): the gold glow + `!` that points a new player at the next
  action. Advances passively in `tick()` and on the boost/galaxy/tickspeed
  actions; the frontend renders the highlight. See
  `../../docs/design/2026-06-30-ui-reveal-and-tutorial.md`.
- `src/autobuyers.rs` — Automation system (Feature 2.6). The 8 AD + Tickspeed
  autobuyers (antimatter-unlocked "slow versions") plus the Dim Boost / Galaxy /
  Big Crunch autobuyers (challenge-only, from NC10/11/12). The IP-cost
  **interval-upgrade** machinery (`cost ×2`, `interval ×0.6`, 100 ms floor) is
  addressed via the `AutobuyerTarget` handle (`autobuyer_can_be_upgraded`,
  `upgrade_autobuyer_interval`, `has_maxed_interval`). `break_infinity_unlockable()`
  exposes the NC12-completed + maxed-Big-Crunch-interval gate that Feature 2.3
  (Break Infinity) consumes. Each interval autobuyer's `timer_ms` is the
  elapsed-time form of the original's `timeSinceLastTick` (`realTimePlayed −
  lastTick`); `Autobuyer::advance` mirrors `IntervaledAutobuyerState` — it tests
  the phase held over from prior ticks *before* adding the current `dt` (the
  original compares against the pre-advance `realTimePlayed`) and resets to 0 on
  a fire (dropping overshoot, like `lastTick = realTimePlayed`). A fire is gated
  on the autobuyer's `ready` flag — its `canTick` minus the interval test, which
  `tick_autobuyers` builds per autobuyer (active + unlocked + the action-specific
  condition: AD/Tickspeed `isAvailableForPurchase && isAffordable`, Dim Boost /
  Galaxy `canBeBought && requirement`, Big Crunch `Player.canCrunch`) — so the
  phase keeps accruing while an autobuyer waits to afford its purchase instead of
  restarting each interval. The AD autobuyers also carry a group toggle
  (`ad_group_active` ↔ `auto.antimatterDims.isActive`): once every tier is
  maxed/unlocked with unlimited bulk the UI collapses them into one control
  (`ad_autobuyer_collapse_display`) and that group flag gates all tiers. Order
  matters where autobuyers share state: the Galaxy autobuyer runs **before** the
  Dim Boost one (the original's `singleComplex` order), so after the AD autobuyers
  grow the 8th dimension a galaxy pre-empts a boost at the same threshold; the
  buy-max Dim Boost autobuyer's `resetTickOn` is `INFINITY` (not `ANTIMATTER_GALAXY`),
  so a galaxy doesn't reset its phase (`reset_autobuyer_ticks`). The save codec
  converts `lastTick ↔ timer_ms` on load/store (see `save/dto.rs` / `save/encode.rs`);
  discarding it desynchronises every autobuyer's firing phase on replay. See
  `../../docs/design/2026-07-03-autobuyers.md`.
- `src/options.rs` — `Options` struct: player UI/UX preferences (mirrors JS
  `player.options`), held in `GameState`, preserved across a Big Crunch.
  Includes the per-action `Confirmations` toggles (boost/galaxy/sacrifice/crunch),
  the `Animations`/`ShowHintText`/`AwayProgress` toggle groups, header-gain
  coloring, the sidebar resource id, and the hidden-tab bitmasks
  (`hidden_tab_bits`/`hidden_subtab_bits`, keyed by the original tab ids)
- `src/observed.rs` — `ObservedState`: read-only snapshot of `GameState` plus
  computed fields (costs, affordability, `next_sacrifice_boost`). The decision
  input for `ad-sim` controllers and the trace/GUI view.
- `src/data/` — Static game configuration (constants, costs, dimension configs)
