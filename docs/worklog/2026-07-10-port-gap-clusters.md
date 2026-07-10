---
date: 2026-07-10
feature: port-gap-clusters
design_docs:
  - ../design/2026-07-09-port-audit.md
---

# Closing the port-audit gap clusters (1–4)

## Summary
A working session implementing the four remaining game-mechanics clusters from
the 2026-07-09 port audit, one at a time: (1) the Tesseract cluster
(7.3/7.6/4.5), (2) Pelle's `isDisabled` sweep (7.7), (3) per-celestial polish
(7.2/7.4/7.5/7.6), (4) glyph extras (6.2). Every step keeps the fidelity suite
at its 1469/1476 baseline (the 7 residuals are documented precision bounds).

## Cluster 1 — the Tesseract cluster

### 1a. EC completions u8 → u16 + EC1 goal-1000 in Enslaved

`maxCompletions` is dynamic in the original: `Enslaved.isRunning && id === 1 ?
1000 : 5`. Our engine had a constant `EC_MAX_COMPLETIONS: u8 = 5` baked into
the goal formula, the completion banking, the pending-completion scan, and the
EC1 study requirement.

- Widened `GameState.eternity_challenges` to `[u16; 12]` (and the glyph-undo
  snapshot's `ecs`, the automator's `last_ec_completions`, the GUI view field,
  the save decode — which now clamps EC1 at 1000, others at 5).
- Added `ec_max_completions(id)` and threaded it through `ec_goal_at`
  (`goalAtCompletions` clamps at `max − 1`, so EC1's goal keeps scaling to
  1e1800 × (1e200)^999 inside the run), `complete_running_ec`,
  `ec_pending_total_completions`, and EC1's secondary study requirement
  (`20000 + min(completions, isRunning ? 999 : 4) × 20000`).
- Test: cap flips 5 ↔ 1000 with the run flag, goal scaling past 5, requirement
  scaling, banking an 11th completion. Fidelity: 1469/1476 (unchanged).

### 1b. Tesseracts

The ID-purchase-cap currency (`Tesseracts` in `enslaved.js`), previously
state-only. Engine: the hardcoded `BASE_COSTS` table (`10^(1e7·base)` IP,
a *threshold* — buying does not spend IP), `can_buy/buy_tesseract` gated on a
completed Enslaved run, `effective_count` (scaled by the
`tesseractMultFromSingularities` milestone, now implemented:
`1 + log10(singularities)/80`), and `capIncrease` (`250e3 × 2^total`, times
`boundless + 1` — the `boundless` alchemy effect accessor also landed:
`amount/80000`). Wired: the ID purchase cap (`id_purchase_cap` is now
instance-scoped and adds `floor(capIncrease)`), the `darkFromTesseracts`
singularity milestone (`1.1^effectiveCount` into the common dark multiplier),
and IU23's real effect (`floor(0.25 × effectiveCount²)` multiplying IU12's
free Dim Boosts — the two stubs the audit called out). GUI: the vendored
tesseract button on the Infinity Dimensions tab (visible once Enslaved is
completed), `buy_tesseract` command + store action.

Tests: cost table/threshold semantics, cap raise on `id_is_capped`, IU23
scaling. Fidelity: 1469/1476 (unchanged).

### 1c. Real-time storage, amplified Realities, auto-release/auto-store

The remaining Enslaved mechanics:

- **Real-time storage** (`isStoringReal`): a storing tick now mirrors
  `realTimeMechanics` — only Ra memories/momentum + Dark Matter Dimensions run,
  real-time records advance, the interval banks at 70% into `storedReal`
  (cap 8 h + 1 h/Nameless level via Ra's `improvedStoredTime`, self-stopping at
  the cap), autobuyers still tick, everything else freezes. The offline path
  (`autoStoreReal`) banks what fits under the cap in `simulate_offline` and
  simulates the remainder.
- **Amplified Realities** (`boostReality` — a module flag in the original, so
  deliberately `serde(skip)`): `realityBoostRatio`/`canAmplify`, the
  `simulatedRealityCount` machinery with the `partSimulatedReality` fractional
  carry (new save passthrough) and the `multiversal` alchemy effect accessor
  (`32·(amount/25000)²`), threading the multiplier through RM / Realities /
  Perk Points / Relic Shards / the recent-Realities ring, and consuming
  `storedReal` (proportionally under 1 real second). The Achievement-154
  binomial extra stays unmodelled (the engine avoids unseeded randomness).
  IU13's projected-RM requirement now includes the amplification factor.
- **Ra auto-release** (`isAutoReleasing`, persisted): every 5th tick discharges
  1% (`useStoredTime(true)`), keeping 99% banked; the release path also gained
  the full `canRelease` guard set (real-time storing / EC12 / Lai'tela / Doomed
  / auto-inside-run) and the `peakGamespeed` update. Game-time storage now
  applies Ra's `20^level` amplification (previously a dead accessor).
- GUI: EnslavedTab grew the real-time-storage half (store/auto-store buttons,
  efficiency/cap lines) and the auto-release toggle (Ra `autoPulseTime`-gated);
  the Glyphs tab gained the original's `RealityAmplifyButton`. Commands:
  `toggle_store_real_time`, `toggle_auto_store_real`, `toggle_boost_reality`,
  `toggle_auto_release`.

Tests: storing-tick freeze/banking/cap-stop, offline banking remainder,
5-tick auto-release cadence, amplified-Reality reward multiplication +
consumption. Fidelity: 1469/1476 (unchanged). **Cluster 1 complete.**

## Cluster 2 — the Pelle `isDisabled` sweep (7.7)

The audit's biggest single gap: `pelle_is_disabled` existed as a query but only
a handful of sites consulted it. This session ported the full
`disabledMechanicUnlocks` table and every consumer the original has, plus the
Pelle upgrade keep/re-enable gates and the rift-milestone effects that were
documented cuts.

### 2a. The `isDisabled` table + consumer sweep

- `pelle_is_disabled` now covers the full JS key set, including the autobuyer
  keys re-enabled by Pelle Upgrades (`pelle_ad_autobuyer_disabled(tier)` per
  AD-tier via upgrades 0/3, dimboost 1, galaxy 4, tickspeed 5, ID autobuyers 9,
  replicanti upgrades 12, TD autobuyers 18) and `pelle_upgrade_applies(id)`
  (= bought && Doomed, the JS `canBeApplied`).
- ~25 consumer sites gained their doomed branches, each mirrored from the JS
  original: IP/infinities gain (crunch), EP mult/gained eternities (eternity),
  achievements (`achievement_applies` + the 32-id Pelle-disabled list,
  `achievement_power` → 1, starting AM → 100), perks (`perk_applies` + the
  uselessPerks list, starting IP/EP → 0), V unlock effects, Ra unlocks (the
  `disabledByPelle` bitmask), Imaginary Upgrades (`isDisabledInDoomed` set),
  Continuum, Singularity milestones, Alchemy (amounts → 0), Black Holes,
  glyph sacrifice (→ 0) and the doomed equip rules, tachyon gain → ×1,
  replicanti speed (decay rift × special glyph only), the galaxy-strength
  `effects` product, and the doomed galaxy halving in both tickspeed branches.
- Keep-on-reset gates: Armageddon/Eternity/Infinity resets honour
  `keepInfinityUpgrades`/`keepBreakInfinityUpgrades` (6/8),
  `dimBoostResetsNothing`/`galaxyNoResetDimboost` (7/11),
  `replicantiStayUnlocked`/`replicantiGalaxyNoReset` (16/13/22),
  `eternitiesNoReset`/`timeStudiesNoReset` (14/15), `keepEternityUpgrades`
  (17), `keepEternityChallenges` (19), `dilationUpgradesNoReset`/
  `tachyonParticlesNoReset` (20/21), `keepAutobuyers` (2) and
  `keepInfinityChallenges` (10 — ICs also re-unlock from the doomed run's
  peak antimatter, `infinity_challenge_unlocked`).
- EC1 in Enslaved: `ec_max_completions` (1000 vs 5), goal scaling and the u16
  completions threading, with the save clamp per-id.

### 2b. `specialGlyphEffect` (chaos milestone 1)

The five single-glyph Pelle bonuses (`pelle_special_glyph_*`): infinity
(IP+1)^0.2 outside EC>8 → IP gain; time (EP+1)^0.3 → EP gain; replication
10^(53^vacuumFill) → replicanti speed; dilation TG^1.5 (min ×1) → the doomed
DT formula; power ×1.02 → the effective galaxy count in both tickspeed
branches.

### 2c. Rift-milestone effects + remaining rebuyables

All previously-cut milestone effects, each at its original site:

- **Vacuum** m1: Replicanti uncap without TS192, and the unlock/upgrade costs
  ÷1e130 — divided on *read* while the stored cost steps undivided, as the
  original (`replicanti_chance_cost`/`replicanti_interval_cost`/galaxy cost/
  unlock). m0 already gated glyph equips; it now also drives
  `glyph_active_slot_count` (Doomed: 1 with the milestone, else 0).
- **Decay** m0: the first Pelle rebuyable also multiplies ID1
  (`1e50^(x−9)` — a penalty below 9 purchases, as the original); m1: galaxies
  10% stronger while Replicanti > 1e1300 (tickspeed `effects` product);
  m2: max RG `+ totalMilestones² − 2·totalMilestones`
  (`pelle_total_rift_milestones`).
- **Chaos** m2: +10% of the Eternity EP gain per real second
  (`applyAutoprestige`; the description says 1%, the code says ×0.1 — we
  follow the code).
- **Recursion** m0: Dimboost power `max(100·c²,1) × max(1e4^(c−40),1)` from
  total EC completions (`total_ec_completions`); m1: IDs
  ×`1e1500^(((c−25)/20)^1.7)`.
- **Paradox** m0: TDs 5–8 cost `(cost/1e2250)^0.5` — in the plain-geometric
  and past-1e6000 branches but *not* the threshold walk, as the original
  (`time_dimension_cost_pelle`); the flip rebuilds stored costs via the
  original's `onStateChange` → `updateTimeDimensionCosts` hook
  (`pelle_check_milestone_states` after a successful rift fill, tracked in a
  non-persisted `paradox_m0_last`). m0 also reveals (gates) the Pelle-only
  Dilation upgrades 11–15. m1: the doomed DT formula becomes TP^1.4.
  m2: Infinity-Power conversion ×`min(1.1075^(Σ dilation rebuyables − 60),
  712)`.
- Remaining Pelle rebuyable effects: `glyphLevels` caps the effective glyph
  level while Doomed (`getAdjustedGlyphLevel`), `infConversion` adds
  `(x·3.5)^0.37` to the conversion exponent, `galaxyPower` multiplies the
  galaxy-strength product (`1 + x/50`). Imaginary Upgrade 9 (Cosmic Filament,
  `1 + 0.03·count`) also landed in the ≥3-galaxy tickspeed branch while
  wiring that formula.

Deferred to their feature homes: the `"V"` key (ST-cost study purchases are
not modelled yet — Cluster 3/V), the `"effarig"` key (gates the Effarig
Replicanti cap/bonus-RG effects — Cluster 3/Effarig), and `cursedgalaxies`
in the tickspeed formula (Cluster 4, cursed glyphs).

Tests: rebuyable-effect formulas, milestone counting, doomed glyph slots,
paradox TD cheapening, vacuum cost discounts + uncap, decay max-RG, IC
re-unlock via upgrade 10, the paradox gate on Dilation upgrades 11–15.
Fidelity: 1469/1476 (unchanged). **Cluster 2 complete.**

## Cluster 3 — per-celestial polish (7.2 / 7.4 / 7.5 / 7.6)

The remaining per-celestial gaps, one celestial at a time. Fidelity held at
1469/1476 throughout.

### 3a. Ra — charged Infinity Upgrades, passive generation, uncountability

- **Charged Infinity Upgrades** (the audit's "charged-IU effect variants"):
  all 11 charged formulas, each replacing its suppressed normal effect
  (`isEffectActive = isBought && !isCharged`): the four `dimInfinityMult`
  pairs / `totalTimeMult` / `thisInfinityTimeMult` become *powers* on the AD
  multiplier (`charged_iu_ad_power` in `applyNDPowers`), `unspentIPMult`
  swaps to `(IP/2)^(1.5·√level)+1`, `buy10Mult`/`dimboostMult` gain
  `1 + level/200` powers, `galaxyBoost` becomes `2 + √level/100`,
  `resetBoost` swaps its −9 for a `1/(1 + √level/10)` requirement multiplier
  (rounded), and `ipGen` generates RM per real second
  (`gainedRM × level² × autoPrestige boost`) while its IP stream keeps
  running (the original reads `effectValue` directly). Charging actions
  (`charge_infinity_upgrade` / `discharge_infinity_upgrade` /
  `can_charge_infinity_upgrade` with the Doomed gate) + the armed
  `disCharge`-on-Reality flag were already consumed on reset. The save's
  `charged` Set holds *string* save-ids — the DTO/encode round-trip was
  numeric and is now string-mapped.
- **`passivePrestigeGen` restructure**: the old `generate_passive_infinities`
  (BreakInfinity term only) and `tick_reality_upgrade_generation` (RU11/RU14,
  wrongly Ra-boosted and un-floored) merged into the JS-shaped
  `passive_prestige_gen`: the RU14 eternitied block (`Ach113 × RU3 ×
  realities·RaBoost × timeetermult`, raised to the Alchemy `eternity` power,
  floored through the new persisted `partEternitied` carry) and the infGen
  block (skipped in EC4; BreakInfinity term × RU5 × RU7 × Ra ×
  `infinityinfmult`; RU11 at 10%/s *without* the Ra term; Effarig's
  Eternity-unlock term `gainedInfinities × (eternities − ⌊gain/2⌋) × dt`;
  shared `partInfinitied` carry). Doomed skips the whole thing.
- **Alchemy**: `uncountability` now generates Realities + Perk Points per
  real second (integral Realities via a `realities_frac` carry that
  round-trips into the save's fractional `player.realities`); `eternity`
  (passive-gen power) and `cardinality` (over-cap Replicanti scale factor)
  gained readers.
- `ipGen`'s IP stream also picked up the missing Teresa/V/Doomed zero-gate.

### 3b. Effarig — the persistent Infinity/Eternity rewards

- `replicanti_cap()`: `max(1, infinitiesTotal^(TS31 ? 120 : 30)) × 1.8e308`
  with the Infinity-stage unlock, dead while Doomed
  (`Pelle.isDisabled("effarig")` — the audit's deferred `"effarig"` key).
  `effarig_bonus_rg` feeds the extra-RG sum. The whole replicanti over-cap
  path became cap-aware and picked up three sibling gaps: the
  `ReplicantiGrowth.scaleFactor` variants (Alchemy `cardinality`, ×2 while
  Doomed, ×10 past 1e2000 with the Eternity strike — including the
  back-out term in the interval), V's doubled `postScale`, and the Doomed
  e308-per-loop growth clamp.
- The Eternity-stage unlock's passive-Infinity term landed in
  `passive_prestige_gen` (above).

### 3c. V — goal reduction + autoAutoClean

- **Perk-Point goal reduction** (`VUnlocks.shardReduction`): per-achievement
  `shardReduction`/`maxShardReduction` curves, step sizes (100 for
  Matterception, 2 for Post-destination), `reductionCost` (1000 PP/step;
  hard achievements ×1.15 per step with the bulk factor),
  `v_reduce_goal` spending `reality.perk_points`, and `conditionValue`
  threading into `tryComplete` + the status view. The doomed/unlock gates
  mirror `isReduced`.
- **autoAutoClean**: `glyph_auto_clean(threshold)` — the faithful
  `Glyphs.autoClean` (top-down, delete-as-you-go) on top of the new
  `isObjectivelyUseless` comparison (`biggerIsBetter` derived per effect as
  `effect(100,2) > effect(1,1.01)`, superset-of-effects + level-or-strength
  filter, Effarig/Reality compare-threshold 1) and the
  `applyFilterToPurge` escape hatch. V's 16-ST unlock + the
  `player.reality.autoAutoClean` flag auto-purge after each Reality
  (`finish_process_reality`). New persisted flags: `autoAutoClean`,
  `applyFilterToPurge`.

### 3d. Lai'tela — exact Continuum + the four autobuyers

- **Continuum**: `CostScale::get_continuum_value` (the original's
  `getContinuumValue`, including the quadratic branch past 1.8e308) replaces
  the linear approximation; AD values now use the tier's real cost scale +
  `currencyAmount` (NC6) + the `isAvailableForPurchase` zero-gate.
  `ad_total_amount` (`max(amount, ⌊10·continuumValue⌋)`) feeds production,
  Dimboost/Galaxy requirement checks, and Sacrifice's AD8 gate.
- **Autobuyers** (`autobuyers.rs`): DMD (interval `1000 ×
  darkAutobuyerSpeed`, bulk via the faithful `maxAllDMDimensions` — 2%-of-DM
  bulk pass + cheapest-first greedy against the snapshot balance, replacing
  the old approximation), Ascension (same interval), Annihilation
  (threshold-triggered, with the persisted `multiplier` input), and
  Singularity condense (`DE ≥ cap × autoCondense`). Save round-trip for all
  four (`player.auto.darkMatterDims/ascension/annihilation/singularity`).

### GUI

Infinity-Upgrades tab: charge/discharge on click + charged/chargeable tile
styles; Ra tab: discharge-on-Reality toggle; V tab: per-achievement
goal-reduction buttons (PP cost); Glyphs tab: purge / harsh purge /
sacrifice-all buttons + filter-protects-purge and auto-purge-on-Reality
toggles; Lai'tela tab: the four autobuyer toggles + annihilation threshold
input. 13 new commands.

Tests: charged-IU swaps (galaxy/dimboost/resetBoost/tier-pair/ipGen-RM),
RU11/14 carries, Effarig cap/bonusRG/eternity-infinities, V reduction
(cost scaling + completion at the reduced goal), auto-clean purge semantics,
Continuum quadratic branch + effective amounts, maxAll bulk-buy,
annihilation/condense autobuyers. ad-core suite: 604. Fidelity: 1469/1476
(unchanged). **Cluster 3 complete.**

## Cluster 4 — glyph extras (6.2)

### Cursed glyphs

- `GlyphType::Cursed` (save id `"cursed"`, non-generated effect bits 0–3, no
  sacrifice value): previously *skipped on save load*, now round-tripped.
- `give_cursed_glyph` (`Glyphs.giveCursedGlyph`): a level-6666, 100%-rarity
  glyph with all four effects; needs free inventory space and at most 5
  cursed glyphs in existence; surfaced on the V tab behind Ra's Hard-V flip.
- The four effects at their sites: `cursedgalaxies` (`level^-0.03`) into the
  ≥3-galaxy tickspeed branch, `curseddimensions` (`level^-0.035`) as a power
  on the AD/ID/TD multipliers, `cursedtickspeed` (`max(log10(level), 1)`,
  additive) widening the free-tickspeed threshold
  (`1 + (base − 1) × effect`), and `cursedEP` (`10^(-level/10)`) leading
  `totalEPMult`. The original's conflicting-effect combination is ported:
  when `cursedgalaxies`/`realitygalaxies` (or `curseddimensions`/
  `effarigdimensions`) are both present and their product is under 1, the
  cursed side absorbs the product and the conflicting effect goes neutral
  (`timeEP`'s pair stays out of frontier with Glyph Alteration).
- Cursed glyphs count −4 in the `maxGlyphs` requirement check (each equipped
  one nets −3), which makes Imaginary Upgrade 22's `< −10` gate and V's hard
  "Requiem for a Glyph" achievement reachable. While Doomed they are
  forbidden from equipping (with Effarig/Reality); purges skip them except
  "sacrifice all".

### Glyph presets (Effarig's `setSaves` unlock)

`player.reality.glyphs.sets` (7 presets of `{name, glyphs}`) now
round-trips — the data previously vanished through the Rust engine.
`save_glyph_set` (empty slot + something equipped), `delete_glyph_set`, and
`load_glyph_set`, which equips exactly-matching inventory glyphs (same type/
level/strength/effects) into free slots — a documented simplification of the
original's fuzzy greedy/lenient matching options. GUI: a presets panel on
the Glyphs tab, save/load/delete per slot.

### Cosmetics decision

Glyph cosmetics (custom colors/symbols, the cosmetic sets from the shop) are
**cut**: they are pure presentation with zero engine effect, and the save
fields pass through untouched. The only mechanical touchpoint — customized
glyphs being protected from non-harsh purges — is dropped along with them
(no glyph can be customized in our port).

Tests: creation + 5-cap, the four effect formulas, the conflict combination,
preset save/load/delete. ad-core suite: 608. Fidelity: 1469/1476 (unchanged).
**Cluster 4 complete — all four port-gap clusters done.** The audit doc's
status rows (4.5, 6.2, 6.7 note, 7.2–7.7) were brought current.
