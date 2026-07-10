---
date: 2026-07-09
feature: multiple (1.3, 2.2, 2.4, 4.2, 4.4, 5.2, 6.2–6.5, 6.7)
design_docs:
  - ../design/2026-07-09-port-audit.md
---

# Port completion pass — working through the 2026-07-09 audit backlog

## Summary

A multi-feature session driving the deferred items from the 2026-07-09 port
audit to done, one feature at a time. Sections are appended per feature as they
land (each feature is one or more commits).

## 1.3 — Closed-form bulk buyers

**What shipped:** `buy_max_tickspeed` is now a faithful port of the original's
`buyMaxTickSpeed`: outside NC9 it uses the analytic
`CostScale::get_max_bought` (the `ExponentialCostScaling` closed form, charging
only the most expensive purchase), under NC9 it loops purchase-by-purchase so
each buy applies the equal-cost bump and stops at the Big Crunch goal.
`buy_tickspeed` gained the original's full `isAvailableForPurchase` guard
(unlocked + not EC9 + not Continuum + pre-break cost cap).

**Surprises:**
- The AD-side bulk buy (`buy_max_dimension_bulk`) already used the closed form;
  the audit's 1.3 note was stale on that half. Only Tickspeed still looped.
- The closed form is *not* purchase-count-equivalent to a single-buy loop: it
  charges only the top purchase, so with 1e40 AM it buys 38 upgrades where the
  cumulative loop affords 37. That is the original's deliberate simplification,
  and the fidelity suite confirmed it: the change moved the grid from
  **1099/1148 → 1118/1148** (+19 cells).

**Tests:** two new unit tests in `tickspeed.rs` (top-purchase-only charging with
exact counts; the super-exponential branch); the two integration tests that
bought tickspeed from a fresh state now unlock it first (AD2 purchase), since
`buy_tickspeed` carries the original's unlock guard.

## 2.2 — Infinity Upgrades bottom row (`ipMult` + `ipOffline`)

**What shipped:** the Achievement-41 bottom row is now fully modelled. Engine:
the `ipMult` rebuyable's two-regime cost curve (×10 steps to 1e3M, ×1e10 steps
to the 1e6M cap), single purchase + the original's two-phase geometric-series
`buyMax` (via new `Decimal::afford/sum_geometric_series` helpers ported into
`break_infinity`), the Big-Crunch-autobuyer dynamic-amount ×2 bump per purchase
(TS181 suppresses it), the `ipOffline` one-time purchase, and the offline
catch-up award (`offline_currency_gain`, called engine-side by
`simulate_offline` and once by the GUI's chunked replay). The IP-mult autobuyer
(1-Eternity milestone) ticks `buy_max_ip_mult` every update. GUI: the bottom
row on the Infinity Upgrades tab (multiplier tile + spoon buttons + ipOffline
tile, vendored classes), new Tauri commands, and the cost-cap footer.

**Fidelity fixes found along the way:**
- `playerInfinityUpgradesOnReset` is now a shared faithful port: it honours
  Reality Upgrade 10 (previously the Reality reset always cleared the upgrade
  bitmasks and `apply_rupg10` never restored them) and grants/clears
  `ipOffline` with the milestone keeps (the original's keep *sets* include it).
- The `ipMult` effect in `total_ip_mult` now carries Effarig's Eternity-stage
  cap (the E1E6 default cap is above the natural e993k max, so only Effarig's
  can bind).

**Deviations:** the original gates the `ipOffline` award on
`player.options.offlineProgress`; that toggle is an 8.8 gap and offline
progress is always on here, so the award is ungated (noted in the code).

**Tests:** eight new unit tests (gating, cost curve, geometric-series buyMax,
threshold crossing, cap, autobuyer bump, ipOffline award, milestone keeps).
Fidelity grid unchanged (1118/1148).

## 2.4 — The achievements tail (everything except 22 News)

**What shipped:** every remaining achievement condition and effect, including
row 18 (Pelle). Conditions: 35 (6-hour offline, fired from the offline
catch-up's replayed interval), 61 (all AD autobuyer bulks maxed — this also
added the missing `upgradeBulk` purchase path, engine + GUI button), 62
(`bestRunIPPM` ≥ 1e8), 65/74 (NC best-time sums), 111 (the recent-infinities
geometric ring), 117 (750-boost bulk purchase), 156 (`noPurchasedTT`), 165
(level-5000 glyph), 172 (RM cap + no charged IUs/equipped glyphs/triads),
181–188 (doom / Pelle upgrades / IC5-while-doomed / strikes / TS181 / game
end). Effects: 126 (RG divides Replicanti by 1.8e308 instead of reset), 133
(ICs stay unlocked + start Eternities completed), 138 (removes TS133's ×10
downside), 156 (×2.5 generated TT — 137's ×2-while-dilated TT half was also
missing and landed with it), 168 (×1.1 Ra memories), 171 (×2 glyph
sacrifice), 175 (synergism uncapped + momentum ×10), 183 (AD `^1.0812…`),
187 (dtGain base ×1.35). New `RequirementChecks` flags `noPurchasedTT` /
`noTriads` with save round-trip.

**Documented approximations:** 35 keys off the replayed away interval rather
than wall-clock `lastUpdate`; 165's per-factor glyph weights always sit at the
equal defaults (weights unmodelled); 171 still requires only the 5 basic
sacrifice types until 6.2 adds Effarig/Reality glyphs; 172's `noTriads` can
never be cleared (Triad Studies unmodelled).

**Tests:** nine new unit tests across achievements/autobuyers seams. Fidelity:
1118 → **1121/1148** (+3).

## 4.2 — Eternity Milestones: the autobuyer + offline-generator tail

**What shipped:** every remaining milestone effect. A new `MilestoneAutobuyer`
type (active flag + `lastTick` timer, fixed 1 s interval) powers the 8
Infinity Dimension autobuyers (milestones 11–18), the 3 Replicanti-upgrade
autobuyers (50/60/80, skipped in EC8, while-loop bulk equivalent to the
original's closed forms), and — intervalless — the Replicanti Galaxy
autobuyer (3, honouring TS131's disable unless Achievement 138). Milestone 9
(`autobuyMaxGalaxies`) switches the Galaxy autobuyer to the `buyMaxInterval`
cadence and a new `max_buy_galaxies` bulk purchase (binary search over the
non-cumulative `Galaxy.requirementAt`, refactored out of
`galaxy_requirement`). The manual RG purchase now buys the full
`Replicanti.galaxies.gain` (bulk with Achievement 126). The offline
catch-up awards the milestone generators in the original's priority order:
`autoEternities` (200) → `autoInfinities` (1000) → `autoEP` (6), plus the
existing `ipOffline` term. Save round-trip for `auto.infinityDims`,
`auto.replicantiUpgrades`, `auto.replicantiGalaxies` (group flags, per-entry
active flags, `lastTick` phases). GUI: grouped autobuyer boxes on the
Autobuyers tab (group toggle + per-entry toggles, vendored row classes).

**Decisions:** the milestone autobuyers tick in the Rust engine's established
order (IDs after the AD autobuyers; RG/Replicanti-upgrades after the prestige
group) rather than reproducing the original's full `Autobuyers.all` order —
the fidelity suite stayed at 1121/1148, so no observable divergence. The ID
autobuyer interval ignores the `autobuyerFasterID` perk for now (task 6.3).

**Tests:** five new unit tests (ID autobuyer fire + milestone gate, RG
autobuyer, Replicanti-upgrade bulk, offline generator priority, galaxy
buy-max with and without a limit).

## 4.4 — Time Studies: audit verification (no code gap found)

**What happened:** the audit's 4.4 note ("some effects await Break-Infinity
cost knobs") turned out to be stale. All 58 pre-dilation study effects are
consulted at engine sites (verified by sweeping `time_study_bought` call
sites and spot-checking formulas — TS111's 285 divisor, TS227/228 sacrifice
terms, TS231 boost power — against the original), the cost-scale knobs
(`dimensionMultDecrease`/`tickSpeedMultDecrease` with the Break-Infinity
rebuyables and EC6/EC11 rewards) landed during the fidelity push, and the
preset slots + import strings are modelled. This session only corrected the
stale docs: the "deferred/neutral" comments in `break_infinity_upgrades.rs`,
the ad-core ARCHITECTURE entries, and the audit rows 2.3 / 4.4 / 4.5 (whose
EC6/EC11/EC8 items are likewise long since wired). Triad studies stay out of
frontier (Ra content).

## 5.2 — Pelle-only Dilation Upgrades 11–15

**What shipped:** the five Doomed-only Dilation Upgrades. State:
`pelle_rebuyables: [u32; 3]` (11 `dtGainPelle` ×5/purchase, 12
`galaxyMultiplier`, 13 `tickspeedPower`) plus one-time bits 14
(`galaxyThresholdPelle`, cube-rooting the TG threshold mult) and 15
(`flatDilationMult`, `1e9^min(((log10 EP − 1500)/2500)^1.2, 1)`), all costed
per the original (1e14/1e15/1e16 rebuyable bases ×100/×1000/×1e4; 1e45/1e55
one-time). `dilation_gain_per_second` now has the Doomed branch
(`TP × dtGain × dtGainPelle × flatDilationMult / 1e5`) — the Paradox-rift TP
power and the special Pelle glyph terms are existing 7.7 cuts (×1). The
tickspeed power lands in `current_tickspeed_ms` before the Effarig override,
matching `Tickspeed.current`. Save round-trip via the existing dilation
rebuyables id-map (now 1–3 + 11–13) and the upgrades id list (now 4–10 +
14–15). GUI: the Pelle rows appear on the Dilation tab once Doomed with the
Paradox rift's first milestone, mirroring the original's
allRebuyables/allSingleUpgrades layout.

**Tests:** five new unit tests (doom gating + costs, the Doomed DT formula,
TG multiplier, threshold cube root, tickspeed power). Fidelity steady at
1121/1148.

## 6.2 — Effarig + Reality glyph types, the glyph filter, undo

**What shipped:** the two remaining glyph types and both deferred QoL
systems.

*Effarig glyphs:* `GlyphType::Effarig` with the 7 effarig effects (bits
20–26) — RM multiplier, instability delay, game-speed power, achievement
power, buy-10 power, all-Dimension power, AM-exponent bend — each applied at
its original site (reality.rs / eternity_challenges.rs / achievements.rs /
infinity_upgrades.rs / dimensions.rs / infinity_dimensions.rs /
time_dimensions.rs). Generation is RNG-stream-faithful: the type joins the
random pool once Effarig's Reality is completed, `randomStrength` now carries
the full `increasedRarity` term (relic-shard `maxRarityBoost`, Achievement
146, the effarig sacrifice boost), the effect roll caps at 7 with Ra's
`glyphEffectCount`, and `effarigrm`/`effarigglyph` are mutually exclusive at
the roll. One Effarig glyph may be equipped at a time. Sacrifice widened to 7
types (`sac: [f64; 7]`), which also makes Achievement 171 faithful; effarig
glyphs refine into the effarig Alchemy resource.

*Reality glyphs:* constructed (never rolled) at 100% rarity from the reality
Alchemy resource (`create_reality_glyph`, consuming the whole resource, with
the 0/9k/15k/25k effect thresholds). Their 4 effects live in the
non-generated bit space: basic-glyph level boost (into
`adjusted_glyph_level`), galaxy strength (tickspeed ≥3 branch), Amplifier
power (`reality_rebuyable_effect`, which also gained the previously-unwired
Imaginary Intensifier bases), and the DT-exponent bump in
`getGlyphLevelInputs`. Reality sacrifice multiplies Ra Memory Chunk gain.

*Filter:* the full `AutoGlyphProcessor` port — 7 score modes (lowest
sacrifice / effect count / rarity / specified effect / effect score / lowest
alchemy / refinement value), 3 rejection modes (sacrifice / refine /
refine-to-cap), per-type configs, `pick`/`wouldKeep` wired into
`auto_reality` behind Effarig's glyph-filter unlock. Save round-trip for
`filter` settings. Compact filter panel in the Glyphs tab.

*Undo:* Teresa's unlock — every equip snapshots the run
(`Glyphs.saveUndo`: currencies, TT, EC completions, run clocks, stored time,
dilation state), `undo_glyph` unequips, performs the reward-free Reality
reset preserving the celestial run flags, and restores the snapshot. The
undo stack round-trips (`toBitmask` numbers for dilation studies/upgrades).

**Also fixed en route:** the instability thresholds now honour Imaginary
Upgrade 7 (`+200`/purchase) alongside `effarigglyph` (both were hardcoded
1000/4000).

**Tests:** six new unit tests (pool gating, rm/glyph exclusion, creation +
thresholds, basic-level boost, filter keep/reject/pick, undo restore).
Fidelity steady at 1121/1148.

## 6.3 — The deferred perks: EC auto-completion + autobuyer speed

**What shipped:** the PEC chain (perks 60/61/62 — 60/40/20-minute EC
auto-completion) as a faithful `EternityChallenges.autoComplete.tick` port:
`reality.autoEC` + `lastAutoEC` state (save round-trip; reset on Reality),
real-time accrual gated on PEC1, sequential completion of the next
not-fully-completed EC, the strict `remaining − interval > 0` loop, V's
`fastAutoEC` reward (achievement power divides the interval — a 7.4 deferred
effect) and Ra's `instantEC` unlock (immediate full completion — a 7.5
deferred bit). The autobuyer-speed perks 101/102 (×3 faster ID / Replicanti
autobuyers) now scale the milestone-autobuyer intervals, together with
Teresa's `autoSpeed` Perk-Shop effect (×2 per purchase). Perk 103 (dilation
autobuyers) stays inert until those autobuyers exist (a Ra QoL cut). The
RU12/IU15 requirement-lock guards on autocompletion are out of frontier
(armed req-locks unmodelled).

**Tests:** two new unit tests (sequential completion incl. the toggle-off
clamp and interval changes; the interval factor). Fidelity steady at
1121/1148.

## 6.4 — RU13's autobuyer half (TD + EP-mult autobuyers)

**What shipped:** the last missing Reality-Upgrade effects. RU13 now unlocks
the 8 Time Dimension autobuyers (the `MilestoneAutobuyer` machinery from 4.2:
1 s interval ÷ Teresa's `autoSpeed`, buy-max above 1e10 EP / singles below,
timers reset on Reality) and the EP-multiplier autobuyer (intervalless:
`applyEU2` then `buy_max_ep_mult`, not while Doomed — the original invokes it
from the TD tick to order it before dimension purchases; our fixed tick order
achieves the same). RU25's Reality autobuyer and RU13's Eternity-autobuyer
modes were already wired. Save round-trip for `auto.timeDims` and
`auto.epMultBuyer`; grouped Autobuyers-tab boxes for both.

**Tests:** one new unit test (RU13 gates both autobuyers).

## 6.5 — Black Holes: inversion + auto-pause

**What shipped:** the two deferred Black-Hole mechanics. *Inversion:*
`blackHoleNegative` (set via the 10^−x slider, unlocked by Ra's hard-V flip
with both holes permanent) makes the game run *slower* than 1 while paused
(`areNegative`, disabled in Enslaved/Lai'tela runs), and the
`requirementChecks.reality.slowestBH` tracker now exists for Imaginary
Upgrade 24's gate — following the original's write points (reality reset
seeds it from the current inversion; unpausing, starting EC12, and an
Enslaved discharge reset it to 1; weakening the slider raises it). V's
`achievementBH` reward (another 7.4 deferred effect) multiplies each active
hole's power. *Auto-pause:* `blackHoleAutoPauseMode` (never / before BH1 /
before BH2) with the faithful `timeToNextPause` — analytic for BH1, the
100-step transition scan for BH2 — wired into `tick_black_holes` via
`autoPauseData` (the tick advances exactly to the pause point 5 s before the
activation, then pauses). Save round-trip for all three fields plus
`slowestBH`; the Black Hole tab gains the auto-pause selector and the
inversion slider.

**Tests:** two new unit tests (inversion + tracker resets; the auto-pause
stopping 5 s before the first activation).
