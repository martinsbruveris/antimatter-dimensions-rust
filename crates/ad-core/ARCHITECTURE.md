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
- `src/galaxy.rs` — Antimatter galaxy purchases
- `src/sacrifice.rs` — Dimension sacrifice
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
  galaxy requirement, reset paths). See
  `../../docs/design/2026-07-03-infinity-upgrades.md`.
- `src/break_infinity_upgrades.rs` — Break Infinity + its 12 upgrades (Feature 2.3).
  `GameState::broke_infinity` (↔ `player.break`) lifts the `1e308` cap and switches
  `gained_infinity_points` to the scaling formula (both in `crunch.rs` / `tick.rs`);
  `break_infinity()` is gated on `break_infinity_unlockable()` (Big Crunch autobuyer
  maxed). This module owns the `BreakInfinityUpgrade` (9 one-time, sharing the save's
  `infinityUpgrades`) + `BreakInfinityRebuyable` (3, in `infinityRebuyables`) types,
  purchase logic, and the effect readers (`break_infinity_upgrade_common_mult`,
  `break_infinity_galaxy_boost`, `break_infinity_autobuyer_speedup`); six effects are
  deferred (neutral). See `../../docs/design/2026-07-03-break-infinity.md`.
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
  their sites (state.rs / replicanti.rs), the reset keeps in eternity.rs.
- `src/time_dimensions.rs` — Time Dimensions (Feature 4.3): 8 EP-bought tiers
  (TD5–8 await Dilation), the threshold/e6000 cost curve, production chain →
  Time Shards → free Tickspeed upgrades (`FreeTickspeed.fromShards` port with
  the 300k softcap + Newton inversion). Made tickspeed a `Decimal`
  (`current_tickspeed_ms`).
- `src/time_studies.rs` — Time Studies (Feature 4.4): Time Theorems (AM/IP/EP
  purchases gated on owning a TD), the 58-study pre-dilation catalogue with the
  original structural rules (Dimension split + TS201, exclusive Pace columns,
  Light/Dark pairs, EC-gated specials), respec. ~40 study effects live at
  their engine sites (each site names its study).
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
  Upgrades (3 rebuyables + 7 one-time) with their effects at the usual sites.
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
  rolls, the early-reality uniformity code, the uncommon guarantee), the
  120-slot inventory + equip/respec, sacrifice (RU19-gated, 5 type boosts),
  and the 20 generated effects' combiners; effects are applied at their
  engine sites (each names its glyph effect).
- `src/perks.rs` — Perks (Feature 6.3): the 35-perk catalogue + connection
  graph, purchase (1 PP, adjacency), on-purchase side effects (START bumps,
  EU1, ACHNR), `starting_ip`/`starting_ep`, and `tick_perk_effects` (EU
  auto-grants, auto TT-gen/TD/Reality-study unlocks). Effects live at their
  sites; the EC-autocomplete and autobuyer-speed perks are deferred.
- `src/reality_upgrades.rs` — Reality Upgrades (Feature 6.4): 5 rebuyable
  Amplifiers (the original's hybrid linear cost scaling) + 20 one-time
  upgrades with `upgReqs` requirement tracking checked at the original's
  events, `applyRUPG10`, RU11/RU14 continuous generation. RU13/RU25
  (autobuyer improvements) deferred.
- `src/black_holes.rs` — Black Holes (Feature 6.5): both holes' state machine
  (BH2's phase advances only while BH1 is active), interval/power/duration
  upgrades, pause + the 5 s unpause power ramp, and the game-speed factor
  consumed by `game_speed_factor` (stacked with the `timespeed` glyph).
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
- `src/achievements.rs` — Normal achievements: `achievement_bits` bitmask helpers
  (`achievement_unlocked`/`unlock_achievement`), the global `achievement_power`
  multiplier, and `starting_antimatter`. Unlocks fire inline from the relevant
  action methods; see `../../docs/design/2026-06-30-achievements.md`.
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
  (Break Infinity) consumes. See `../../docs/design/2026-07-03-autobuyers.md`.
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
