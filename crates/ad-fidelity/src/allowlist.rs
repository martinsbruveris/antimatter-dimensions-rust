//! The comparison allowlist over the `player` save tree (design §5).
//!
//! Include-only: every rule here is a field we understand and compare; anything
//! not listed (options/UI, unported systems, `Date.now`/real-time and game-time
//! bookkeeping, values derived from a primary) is intentionally not visited.
//! Paths are JS/save keys (the comparison runs on the serialized form). A `[]`
//! suffix iterates an array element-wise; [`Compare::IdSet`]/[`Compare::Glyphs`]
//! rules name the container directly.
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
        R("challenge.infinity.current", Exact),
        R("challenge.infinity.completedBits", Exact),
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
        R("reality.realityMachines", Decimal),
        R("reality.maxRM", Decimal),
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
    ]);

    // --- Black holes (partial) ---
    v.extend([
        R("blackHole[].unlocked", Exact),
        R("blackHole[].active", Exact),
        R("blackHole[].intervalUpgrades", Exact),
        R("blackHole[].powerUpgrades", Exact),
        R("blackHole[].durationUpgrades", Exact),
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
    ]);

    // --- Autobuyers (mutable state; the Automator can change them at runtime) ---
    v.extend([
        R("auto.autobuyersOn", Exact),
        R("auto.antimatterDims.all[].isActive", Exact),
        R("auto.antimatterDims.all[].isBought", Exact),
        R("auto.antimatterDims.all[].mode", Exact),
        R("auto.antimatterDims.all[].interval", Number),
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
    ]);

    v
}
