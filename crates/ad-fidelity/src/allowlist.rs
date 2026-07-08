//! The comparison allowlist over the `player` save tree (design §5).
//!
//! Include-only: a rule here is a field that must match at **full fidelity**.
//! The end goal is byte-parity with the original on every engine-relevant field.
//! The include test is "does it affect the simulation" — crucially, a *time-based*
//! field that **feeds a mechanic** (a game-speed timer, a completion timer gating a
//! reward, a resource banked over time) is engine-relevant and is compared; the
//! harness feeds both engines the same diff, so such fields are reproducible.
//! Only three things stay out of scope: (1) options/UI inputs a tick never mutates,
//! (2) *pure* real-time bookkeeping nothing reads (`realTimePlayed`, `this*.realTime`)
//! and `Date.now`/wall-clock snapshots (`lastUpdate`, `backupTimer`), and (3) values
//! derived from a primary (costs recomputed from purchase counts). Paths are
//! JS/save keys (the comparison runs on the serialized form). A `[]` suffix iterates
//! an array element-wise; [`Compare::IdSet`]/[`Compare::Glyphs`] rules name the
//! container directly.
//!
//! **Listing a field ad-core does not model yet is intentional — it *showcases*
//! the gap.** The write path (`ad-core/src/save/encode.rs`) overlays the modelled
//! fields onto a fresh-start template, so an unmodelled field is emitted as its
//! fresh default. A rule over such a field therefore diverges on a populated save
//! as `Rust = <default>` vs `JS = <real>` — which is the point: the harness
//! exists to surface exactly these "not ported yet" cases, not to hide them. When
//! a divergence appears, read it as either a formula bug (modelled field) or a
//! missing-model gap (fresh-default field); the roundtrip identity guard (design
//! §6) tells the two apart.
//!
//! Consequence: this is a **flat** list — gap rows and modelled rows share one
//! grid, with no `Modelled`/`Gap` distinction. Because a few gaps (`postC4Tier`,
//! the `requirementChecks` run-flags, `records.thisReality.maxAM`) are non-default
//! in essentially every real save, every grid cell currently carries at least one
//! known-gap divergence, so the suite reads all-red until those systems are
//! modelled. That is an accepted trade-off (it maximises gap visibility at the
//! cost of a green regression gate); if the noise ever outweighs the signal,
//! re-introduce a per-rule kind so the pass/fail count tracks only modelled rows.
//!
//! As the port grows into later systems, extend this table (and prefer failing
//! loudly on genuinely new fields once such a check exists — design §5).

use crate::compare::{Compare::*, FieldRule};

const R: fn(&'static str, crate::compare::Compare) -> FieldRule = FieldRule::new;

/// The full allowlist, in save-tree reading order.
pub fn allowlist() -> Vec<FieldRule> {
    let mut v = Vec::new();

    // --- Core antimatter economy ---
    v.extend([
        R("antimatter", Decimal),
        R("dimensions.antimatter[].amount", Decimal),
        R("dimensions.antimatter[].bought", Exact),
        R("dimensions.antimatter[].costBumps", Exact),
        R("sacrificed", Decimal),
        R("dimensionBoosts", Exact),
        R("galaxies", Exact),
        R("totalTickBought", Exact),
        R("chall9TickspeedCostBumps", Exact),
        R("chall8TotalSacrifice", Decimal),
        R("chall2Pow", Number),
        R("chall3Pow", Decimal),
        R("matter", Decimal),
        // Gaps (not yet modelled): NC-run and prestige-accumulator state.
        R("ic2Count", Exact), // (gap) IC2 dimension-buy counter
        R("partInfinitied", Number), // (gap) fractional-infinities accumulator
        R("postC4Tier", Exact), // (gap) NC4 reward tier
        R("IPMultPurchases", Exact), // (gap) legacy IP-mult buyer count
    ]);

    // --- Infinity ---
    v.extend([
        R("break", Exact),
        R("infinities", Decimal),
        R("infinityPoints", Decimal),
        R("infinityPower", Decimal),
        R("infinitiesBanked", Decimal),
        R("partInfinityPoint", Number),
        R("dimensions.infinity[].amount", Decimal),
        R("dimensions.infinity[].baseAmount", Number),
        R("dimensions.infinity[].isUnlocked", Exact),
        R("infinityUpgrades", IdSet),
        R("infinityRebuyables", Exact),
        R("infinity.upgradeBits", Exact), // (gap) bitmask twin of infinityUpgrades
        R("challenge.normal.current", Exact), // (gap) NC run state
        R("challenge.normal.completedBits", Exact), // (gap)
        R("challenge.infinity.current", Exact),
        R("challenge.infinity.completedBits", Exact),
        // Best challenge *game*-times (written from `thisInfinity.time`, not the
        // clock): gate the Break-Infinity `worstChallenge` and Eternity-Upgrade-4
        // `infinityChallengeSum` reward multipliers. MAX_VALUE = never completed.
        R("challenge.normal.bestTimes[]", Number), // (gap) NC best-times not modelled
        R("challenge.infinity.bestTimes[]", Number), // modelled (ic_best_times_ms)
    ]);

    // --- Eternity ---
    v.extend([
        R("eternityPoints", Decimal),
        R("eternities", Decimal),
        R("timeShards", Decimal),
        R("totalTickGained", Exact),
        R("timestudy.theorem", Decimal),
        R("timestudy.maxTheorem", Decimal),
        R("timestudy.amBought", Exact),
        R("timestudy.ipBought", Exact),
        R("timestudy.epBought", Exact),
        R("timestudy.studies", IdSet),
        // Automator-consumed study config: `presets` (loaded by `studies load
        // preset`) is modelled; `preferredPaths` (engine-written on split picks,
        // read by `buyStudiesUntil`) is a gap. `shopMinimized` stays skip (UI).
        R("timestudy.presets", Exact),
        R("timestudy.preferredPaths", Exact), // (gap)
        // `player.respec` — clears the study tree on the next Eternity; the
        // Automator sets it (`studies respec`).
        R("respec", Exact),
        R("dimensions.time[].amount", Decimal),
        R("dimensions.time[].bought", Exact),
        R("eternityUpgrades", IdSet),
        R("epmultUpgrades", Exact),
        R("eternityChalls", Exact),
        R("eterc8ids", Exact),
        R("eterc8repl", Exact),
        R("challenge.eternity.current", Exact),
        R("challenge.eternity.unlocked", Exact),
        R("challenge.eternity.requirementBits", Exact),
    ]);

    // --- Replicanti (expectation mode; the JS sampler is mocked to its mean) ---
    v.extend([
        R("replicanti.unl", Exact),
        R("replicanti.amount", Decimal),
        R("replicanti.chance", Number),
        R("replicanti.interval", Number),
        R("replicanti.galaxies", Exact),
        R("replicanti.boughtGalaxyCap", Exact),
    ]);

    // --- Dilation ---
    v.extend([
        R("dilation.studies", IdSet),
        R("dilation.active", Exact),
        R("dilation.tachyonParticles", Decimal),
        R("dilation.dilatedTime", Decimal),
        R("dilation.nextThreshold", Decimal),
        R("dilation.baseTachyonGalaxies", Exact),
        R("dilation.totalTachyonGalaxies", Exact),
        R("dilation.upgrades", IdSet),
        R("dilation.rebuyables", Exact),
        R("dilation.lastEP", Decimal),
    ]);

    // --- Reality (partial) ---
    v.extend([
        R("realities", Exact),
        R("partSimulatedReality", Number), // (gap) fractional-reality accumulator
        R("reality.realityMachines", Decimal),
        R("reality.maxRM", Decimal),
        R("reality.imaginaryMachines", Number), // (gap) iM currency (folded into maxRM today)
        R("reality.iMCap", Number),             // (gap) iM cap
        R("reality.imaginaryUpgReqs", Exact),   // (gap)
        R("reality.glyphs.createdRealityGlyph", Exact), // (gap) one-time gate
        R("reality.unlockedEC", Exact),         // (gap)
        R("reality.partEternitied", Decimal), // (gap) fractional-eternities accumulator
        R("reality.perkPoints", Exact),
        R("reality.perks", IdSet),
        R("reality.rebuyables", Exact),
        R("reality.upgradeBits", Exact),
        R("reality.upgReqs", Exact),
        R("reality.seed", Exact),
        R("reality.initialSeed", Exact),
        R("reality.secondGaussian", Number),
        R("reality.glyphs.sac", Exact),
        R("reality.glyphs.active", Glyphs),
        R("reality.glyphs.inventory", Glyphs),
        // Imaginary Upgrades (Feature 6.4-late / 7.6): the owned-bit set and the
        // rebuyable purchase counts (a "1".."10" id-keyed map). The iM currency
        // itself is re-earned from the cap and rides `maxRM`, so it is not a
        // separate field.
        R("reality.imaginaryUpgradeBits", Exact),
        R("reality.imaginaryRebuyables", Exact),
        // Reality-upgrade req locks + auto-achievement state (modelled).
        R("reality.reqLock.reality", Exact),
        R("reality.respec", Exact), // glyph respec on Reality; Automator-settable
        R("reality.autoAchieve", Exact),
        R("reality.gainedAutoAchievements", Exact),
        // Glyph-automation + EC-automation toggles that change the run (gaps).
        R("reality.autoEC", Exact), // (gap) auto-complete ECs
        R("reality.autoAutoClean", Exact), // (gap) auto-purge glyphs
        R("reality.applyFilterToPurge", Exact), // (gap)
        R("reality.hasCheckedFilter", Exact), // (gap) reality-autobuyer state
        R("reality.autoSort", Exact), // (gap) inventory ordering
        R("reality.autoCollapse", Exact), // (gap) inventory ordering
        R("reality.moveGlyphsOnProtection", Exact), // (gap) inventory ordering
        // Automator run-state (modelled). Scripts/constants (program input) are
        // left out; editor/UI sub-fields stay skip.
        R("reality.automator.forceUnlock", Exact),
        R("reality.automator.state.topLevelScript", Exact),
        R("reality.automator.state.repeat", Exact),
        R("reality.automator.state.forceRestart", Exact),
        R("reality.automator.state.mode", AutomatorMode),
        R("reality.automator.execTimer", Number),
        R("reality.automator.state.stack", AutomatorStack),
    ]);

    // --- Black holes ---
    // The phase/pause fields drive the active state → game-speed multiplier, so
    // they're mechanics, not bookkeeping, despite being real-time-driven.
    v.extend([
        R("blackHole[].unlocked", Exact),
        R("blackHole[].active", Exact),
        R("blackHole[].intervalUpgrades", Exact),
        R("blackHole[].powerUpgrades", Exact),
        R("blackHole[].durationUpgrades", Exact),
        R("blackHole[].phase", Number), // cycle position (drives activation)
        R("blackHole[].activations", Exact), // cycle-crossing count
        R("blackHolePause", Exact),
        R("blackHolePauseTime", Number), // feeds the post-unpause speed ramp
        R("blackHoleAutoPauseMode", Exact), // (gap) auto-pause not modelled
        R("blackHoleNegative", Number),  // (gap) inversion factor
    ]);

    // --- Achievements + requirement checks ---
    v.extend([
        // Rust always writes the full 18 rows; JS grows the array on demand, so
        // its `achievementBits` is a (zero) row shorter until the Pelle row is
        // touched. `PaddedBits` zero-pads the shorter side before comparing.
        R("achievementBits", PaddedBits),
        R("requirementChecks.eternity.noRG", Exact),
        R("requirementChecks.reality.noInfinities", Exact),
        R("requirementChecks.reality.noEternities", Exact),
        R("requirementChecks.reality.maxGlyphs", Exact),
        // Gaps (not yet modelled): the other per-run "avoided X" flags/peaks that
        // gate challenge/achievement/reality rewards.
        R("requirementChecks.infinity.maxAll", Exact),
        R("requirementChecks.infinity.noSacrifice", Exact),
        R("requirementChecks.infinity.noAD8", Exact),
        R("requirementChecks.eternity.onlyAD1", Exact),
        R("requirementChecks.eternity.onlyAD8", Exact),
        R("requirementChecks.eternity.noAD1", Exact),
        R("requirementChecks.reality.noAM", Exact),
        R("requirementChecks.reality.noTriads", Exact),
        R("requirementChecks.reality.noPurchasedTT", Exact),
        R("requirementChecks.reality.noContinuum", Exact),
        R("requirementChecks.reality.maxStudies", Exact),
        R("requirementChecks.reality.maxID1", Decimal),
        R("requirementChecks.reality.slowestBH", Number),
    ]);

    // --- Records: the peaks/rates that gate unlocks and feed formulas ---
    v.extend([
        R("records.totalAntimatter", Decimal),
        R("records.thisInfinity.maxAM", Decimal),
        R("records.thisInfinity.bestIPmin", Decimal),
        R("records.thisInfinity.bestIPminVal", Decimal),
        R("records.thisEternity.maxAM", Decimal),
        R("records.thisEternity.maxIP", Decimal),
        R("records.thisEternity.bestEPmin", Decimal),
        R("records.thisEternity.bestEPminVal", Decimal),
        R("records.thisReality.maxEP", Decimal),
        R("records.thisReality.maxReplicanti", Decimal),
        R("records.thisReality.maxDT", Decimal),
        // Gaps (not yet modelled): peer peaks and the rate records that gate
        // specific rewards / auto-prestige "X-highest" modes.
        R("records.thisReality.maxAM", Decimal),
        R("records.thisReality.maxIP", Decimal),
        R("records.thisEternity.bestIPMsWithoutMaxAll", Decimal),
        R("records.thisEternity.bestInfinitiesPerMs", Decimal),
        R("records.thisReality.bestEternitiesPerMs", Decimal),
        R("records.thisReality.bestRSmin", Number),
        R("records.thisReality.bestRSminVal", Number),
        R("records.bestInfinity.bestIPminEternity", Decimal),
        R("records.bestInfinity.bestIPminReality", Decimal),
        R("records.bestEternity.bestEPminReality", Decimal),
        R("records.fullGameCompletions", Exact),
        // Game-time records: reproducible (harness feeds the same diff) and they
        // exercise the game-speed/time-scaling path (EC12, black holes, dilation).
        // Real-time (`*.realTime`, `realTimePlayed`) and wall-clock snapshots
        // (`gameCreatedTime`) stay skip.
        R("records.totalTimePlayed", Number),
        R("records.timePlayedAtBHUnlock", Number), // (gap) game-time snapshot
        R("records.thisInfinity.time", Number),
        R("records.thisEternity.time", Number),
        R("records.thisReality.time", Number),
        R("records.thisInfinity.lastBuyTime", Number),
        R("records.bestInfinity.time", Number),
        R("records.bestEternity.time", Number),
        R("records.bestReality.time", Number),
        // The recent-prestige rings feed glyph fast-reality, achievements, and the
        // Automator. All-numeric tuples `[gameTime, realTime, gain, …]` → Decimal
        // (handles huge gains + MAX_VALUE placeholders); `[][]` is order-sensitive.
        R("records.recentInfinities[][]", Decimal), // (gap) ring not modelled
        R("records.recentEternities[][]", Decimal),
        R("records.recentRealities[][]", Decimal),
    ]);

    // --- Autobuyers (mutable state; the Automator can change them at runtime) ---
    v.extend([
        R("auto.autobuyersOn", Exact),
        R("auto.antimatterDims.all[].isActive", Exact),
        R("auto.antimatterDims.all[].isBought", Exact),
        R("auto.antimatterDims.all[].mode", Exact),
        R("auto.antimatterDims.all[].interval", Number),
        R("auto.antimatterDims.all[].bulk", Exact),
        R("auto.tickspeed.isActive", Exact),
        R("auto.tickspeed.isBought", Exact),
        R("auto.tickspeed.mode", Exact),
        R("auto.tickspeed.interval", Number),
        R("auto.dimBoost.isActive", Exact),
        R("auto.dimBoost.interval", Number),
        R("auto.galaxy.isActive", Exact),
        R("auto.galaxy.interval", Number),
        R("auto.bigCrunch.isActive", Exact),
        R("auto.bigCrunch.interval", Number),
        R("auto.bigCrunch.mode", Exact),
        R("auto.bigCrunch.amount", Decimal),
        R("auto.bigCrunch.increaseWithMult", Exact),
        R("auto.bigCrunch.time", Number),
        R("auto.bigCrunch.xHighest", Decimal),
        R("auto.eternity.isActive", Exact),
        R("auto.eternity.mode", Exact),
        R("auto.eternity.amount", Decimal),
        R("auto.eternity.increaseWithMult", Exact),
        R("auto.eternity.time", Number),
        R("auto.eternity.xHighest", Decimal),
        R("auto.reality.isActive", Exact),
        R("auto.reality.mode", Exact),
        R("auto.reality.rm", Decimal),
        R("auto.reality.glyph", Exact),
        R("auto.reality.time", Number),
        // `lastTick` = the autobuyer phase, encoded as `realTimePlayed - timer_ms`
        // (reproducible; JS = realTimePlayed at last tick). Modelled for the five
        // core autobuyers. Coarse under relative tolerance at high realTimePlayed.
        R("auto.antimatterDims.all[].lastTick", Number),
        R("auto.tickspeed.lastTick", Number),
        R("auto.dimBoost.lastTick", Number),
        R("auto.galaxy.lastTick", Number),
        R("auto.bigCrunch.lastTick", Number),
        // Galaxy/DimBoost caps shape what the autobuyer buys (like AD mode/bulk).
        R("auto.galaxy.limitGalaxies", Exact), // (gap)
        R("auto.galaxy.maxGalaxies", Exact),   // (gap)
        R("auto.galaxy.buyMax", Exact),        // (gap)
        R("auto.galaxy.buyMaxInterval", Number), // (gap)
        R("auto.dimBoost.limitDimBoosts", Exact), // (gap)
        R("auto.dimBoost.maxDimBoosts", Exact), // (gap)
        R("auto.dimBoost.limitUntilGalaxies", Exact), // (gap)
        R("auto.dimBoost.galaxies", Exact),    // (gap)
        R("auto.dimBoost.buyMaxInterval", Number), // (gap)
        // Gates Lai'tela continuum → continuous dimension buying. Modelled, but
        // encode doesn't write `auto.disableContinuum` back → write-side gap.
        R("auto.disableContinuum", Exact), // (gap)
    ]);

    // --- Autobuyer gaps (subsystems not yet modelled) ---
    // The Automator can toggle these at runtime, so they are mutable state like
    // the modelled autobuyers above; ad-core doesn't drive them yet, so each is a
    // fresh default until then. `all[]` iterates the per-target rows.
    v.extend([
        R("auto.antimatterDims.isActive", Exact), // master toggle
        R("auto.tickspeed.isUnlocked", Exact),
        R("auto.sacrifice.isActive", Exact),
        R("auto.sacrifice.multiplier", Decimal),
        R("auto.reality.shard", Number),
        R("auto.infinityDims.all[].isActive", Exact),
        R("auto.infinityDims.isActive", Exact),
        R("auto.timeDims.all[].isActive", Exact),
        R("auto.timeDims.isActive", Exact),
        R("auto.replicantiGalaxies.isActive", Exact),
        R("auto.replicantiUpgrades.all[].isActive", Exact),
        R("auto.replicantiUpgrades.isActive", Exact),
        R("auto.timeTheorems.isActive", Exact),
        R("auto.dilationUpgrades.all[].isActive", Exact),
        R("auto.dilationUpgrades.isActive", Exact),
        R("auto.blackHolePower.all[].isActive", Exact),
        R("auto.blackHolePower.isActive", Exact),
        R("auto.realityUpgrades.all[].isActive", Exact),
        R("auto.realityUpgrades.isActive", Exact),
        R("auto.imaginaryUpgrades.all[].isActive", Exact),
        R("auto.imaginaryUpgrades.isActive", Exact),
        R("auto.darkMatterDims.isActive", Exact),
        R("auto.ascension.isActive", Exact),
        R("auto.annihilation.isActive", Exact),
        R("auto.annihilation.multiplier", Number),
        R("auto.singularity.isActive", Exact),
        R("auto.ipMultBuyer.isActive", Exact),
        R("auto.epMultBuyer.isActive", Exact),
        // Their timer phases (gaps, alongside the isActive gaps above).
        R("auto.infinityDims.all[].lastTick", Number),
        R("auto.timeDims.all[].lastTick", Number),
        R("auto.replicantiUpgrades.all[].lastTick", Number),
        R("auto.dilationUpgrades.all[].lastTick", Number),
        R("auto.darkMatterDims.lastTick", Number),
        R("auto.ascension.lastTick", Number),
    ]);

    // --- Celestials (Phase 7) ---
    //
    // The include test is "engine-relevant at full fidelity", not "already
    // modelled" (see the module docs): fields ad-core does not model yet are
    // listed too, and diverge as `Rust = fresh-default` vs `JS = real` to
    // showcase the gap. Rows marked "(gap)" below are those not-yet-overlaid.
    //
    // Time-based fields that *feed a mechanic* are included, not skipped: Enslaved
    // `storedReal` (spent to amplify Realities), Ra `momentumTime` (momentum
    // multiplier), Lai'tela `thisCompletion`/`fastestCompletion` (reality-reward
    // multiplier) and the DMD `timeSinceLastUpdate` (production timer). Only pure
    // bookkeeping / UI is skipped: Teresa `timePoured` (transient, not saved), the
    // `quoteBits`, glyph loadout snapshots (`teresa.bestAMSet`, `v.runGlyphs`), the
    // Enslaved hint system, and the Lai'tela/Pelle display toggles.

    // Teresa.
    v.extend([
        R("celestials.teresa.pouredAmount", Number),
        R("celestials.teresa.unlockBits", Exact),
        R("celestials.teresa.run", Exact),
        R("celestials.teresa.bestRunAM", Decimal),
        R("celestials.teresa.perkShop", Exact),
        R("celestials.teresa.lastRepeatedMachines", Decimal), // (gap)
    ]);

    // Effarig. `glyphWeights` drive the auto-glyph-processor's keep/sacrifice and
    // Effarig effect weighting, and gate an Imaginary Upgrade requirement.
    v.extend([
        R("celestials.effarig.relicShards", Number),
        R("celestials.effarig.unlockBits", Exact),
        R("celestials.effarig.run", Exact),
        R("celestials.effarig.glyphWeights", Exact), // (gap) {ep,repl,dt,eternities}
        R("celestials.effarig.autoAdjustGlyphWeights", Exact), // (gap)
    ]);

    // Enslaved (The Nameless Ones). Both `stored` (banked *game* time) and
    // `storedReal` (banked *real* time, spent to amplify Realities) are spendable
    // resources. `autoStoreReal`/`isAutoReleasing` gate the storage/release tick.
    // The hint system stays skip (display-only; `zeroHintTime` uses Date.now).
    v.extend([
        R("celestials.enslaved.isStoring", Exact),
        R("celestials.enslaved.stored", Number),
        R("celestials.enslaved.isStoringReal", Exact),
        R("celestials.enslaved.storedReal", Number),
        R("celestials.enslaved.autoStoreReal", Exact), // (gap)
        R("celestials.enslaved.isAutoReleasing", Exact), // (gap)
        R("celestials.enslaved.run", Exact),
        R("celestials.enslaved.completed", Exact),
        R("celestials.enslaved.tesseracts", Exact),
        R("celestials.enslaved.unlocks", IdSet),
        R("celestials.enslaved.hasSecretStudy", Exact), // (gap)
        R("celestials.enslaved.feltEternity", Exact),   // (gap)
        R("celestials.enslaved.progressBits", Exact),   // (gap)
    ]);

    // V. `runRecords` are the per-condition best values that gate V-achievement
    // completion (default `[-10, 0, …]`, no real-time element); compared in
    // log-space since large conditions store Decimals.
    v.extend([
        R("celestials.v.unlockBits", Exact),
        R("celestials.v.run", Exact),
        R("celestials.v.runUnlocks", IdSet),
        R("celestials.v.goalReductionSteps", Exact),
        R("celestials.v.STSpent", Exact),
        R("celestials.v.runRecords[]", Decimal),
    ]);

    // Ra. Pets are an object keyed by name (`pets.teresa`, …), so each is named
    // explicitly. Memories/Chunks accrue from real time but are the Ra currency.
    v.extend([
        R("celestials.ra.unlockBits", Exact),
        R("celestials.ra.run", Exact),
        R("celestials.ra.charged", IdSet),
        R("celestials.ra.disCharge", Exact),
        R("celestials.ra.peakGamespeed", Number),
        R("celestials.ra.momentumTime", Number), // → momentum memory-gain multiplier
        R("celestials.ra.petWithRemembrance", Exact),
        R("celestials.ra.alchemy[].amount", Number),
        R("celestials.ra.alchemy[].reaction", Exact),
        R("celestials.ra.highestRefinementValue.power", Number),
        R("celestials.ra.highestRefinementValue.infinity", Number),
        R("celestials.ra.highestRefinementValue.time", Number),
        R("celestials.ra.highestRefinementValue.replication", Number),
        R("celestials.ra.highestRefinementValue.dilation", Number),
        R("celestials.ra.highestRefinementValue.effarig", Number),
    ]);
    for pet in ["teresa", "effarig", "enslaved", "v"] {
        v.extend([
            R(leak(format!("celestials.ra.pets.{pet}.level")), Exact),
            R(leak(format!("celestials.ra.pets.{pet}.memories")), Number),
            R(
                leak(format!("celestials.ra.pets.{pet}.memoryChunks")),
                Number,
            ),
            R(
                leak(format!("celestials.ra.pets.{pet}.memoryUpgrades")),
                Exact,
            ),
            R(
                leak(format!("celestials.ra.pets.{pet}.chunkUpgrades")),
                Exact,
            ),
        ]);
    }

    // Lai'tela + the Dark Matter Dimensions.
    v.extend([
        R("celestials.laitela.darkMatter", Decimal),
        R("celestials.laitela.maxDarkMatter", Decimal),
        R("celestials.laitela.darkEnergy", Number),
        R("celestials.laitela.singularities", Number),
        R("celestials.laitela.singularityCapIncreases", Exact),
        R("celestials.laitela.darkMatterMult", Number),
        R("celestials.laitela.run", Exact),
        R("celestials.laitela.entropy", Number),
        R("celestials.laitela.difficultyTier", Exact),
        R("celestials.laitela.upgrades", Exact), // (gap) id-keyed object
        // Completion timers feed the reality-reward multiplier `(360/fastest)^2`
        // and gate the difficulty tier.
        R("celestials.laitela.thisCompletion", Number),
        R("celestials.laitela.fastestCompletion", Number),
        R("celestials.laitela.dimensions[].amount", Decimal),
        R("celestials.laitela.dimensions[].intervalUpgrades", Exact),
        R("celestials.laitela.dimensions[].powerDMUpgrades", Exact),
        R("celestials.laitela.dimensions[].powerDEUpgrades", Exact),
        R("celestials.laitela.dimensions[].ascensionCount", Exact),
        // DMD production timer (accumulates realDiff, produces on interval).
        R(
            "celestials.laitela.dimensions[].timeSinceLastUpdate",
            Number,
        ),
    ]);

    // Pelle (the Doomed reality). Rifts are an object keyed by name; only Decay
    // carries `percentageSpent`.
    v.extend([
        R("celestials.pelle.doomed", Exact),
        R("celestials.pelle.remnants", Number),
        R("celestials.pelle.realityShards", Decimal),
        R("celestials.pelle.records.totalAntimatter", Decimal),
        R("celestials.pelle.records.totalInfinityPoints", Decimal),
        R("celestials.pelle.records.totalEternityPoints", Decimal),
        R("celestials.pelle.upgrades", IdSet),
        R("celestials.pelle.progressBits", Exact),
        R("celestials.pelle.rebuyables", Exact),
        R("celestials.pelle.galaxyGenerator.unlocked", Exact),
        R("celestials.pelle.galaxyGenerator.spentGalaxies", Number),
        R("celestials.pelle.galaxyGenerator.generatedGalaxies", Number),
        R("celestials.pelle.galaxyGenerator.phase", Exact),
        R("celestials.pelle.galaxyGenerator.sacrificeActive", Exact),
        R("celestials.pelle.rifts.decay.percentageSpent", Number),
    ]);
    for rift in ["vacuum", "decay", "chaos", "recursion", "paradox"] {
        v.extend([
            R(leak(format!("celestials.pelle.rifts.{rift}.fill")), Decimal),
            R(leak(format!("celestials.pelle.rifts.{rift}.active")), Exact),
            R(
                leak(format!("celestials.pelle.rifts.{rift}.reducedTo")),
                Number,
            ),
        ]);
    }

    // Pelle's finale flag (top-level `player.isGameEnd`).
    v.push(R("isGameEnd", Exact));

    v
}

/// Leak an owned path into a `&'static str`. The allowlist is built once per
/// process, so the handful of programmatically-composed celestial paths (per Ra
/// pet, per Pelle rift) are leaked rather than threading lifetimes through
/// [`FieldRule`]. Bounded and one-shot — not called in a loop over time.
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
