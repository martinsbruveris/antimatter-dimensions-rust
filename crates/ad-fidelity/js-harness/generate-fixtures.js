// generate-fixtures.js — Runs the actual Antimatter Dimensions
// game code to produce reference values for fidelity tests.
//
// Usage: node generate-fixtures.js
// Output: ../fixtures/pre-infinity.json

const fs = require("fs");
const path = require("path");

// Step 1: Load shims (sets up global scope)
const { player, Decimal } = require("./shims");

// Step 2: Load game source files
const loader = require("./loader");

console.log("Loading game source files...");
loader.loadConstants();
console.log("  ✓ constants.js (DC object)");

loader.loadEffects();
console.log("  ✓ effects.js (Decimal prototype extensions)");

loader.loadMath();
console.log("  ✓ math.js (ExponentialCostScaling)");

// Load dimension infrastructure
loader.loadAndRegister(
  "dimensions/dimension.js",
  "global.DimensionState = DimensionState;"
);
console.log("  ✓ dimension.js (DimensionState base class)");

// The antimatter-dimension.js file needs AntimatterDimension()
// accessor. We'll load it and register.
loader.loadAndRegister(
  "dimensions/antimatter-dimension.js",
  `global.AntimatterDimensionState = AntimatterDimensionState;
global.AntimatterDimensions = AntimatterDimensions;
global.AntimatterDimension = AntimatterDimension;
global.antimatterDimensionCommonMultiplier = antimatterDimensionCommonMultiplier;
global.getDimensionFinalMultiplierUncached = getDimensionFinalMultiplierUncached;
global.applyNDMultipliers = applyNDMultipliers;
`
);
console.log("  ✓ antimatter-dimension.js");

loader.loadAndRegister(
  "dimboost.js",
  "global.DimBoost = DimBoost;\nglobal.DimBoostRequirement = DimBoostRequirement;"
);
console.log("  ✓ dimboost.js (DimBoost)");

loader.loadAndRegister(
  "sacrifice.js",
  "global.Sacrifice = Sacrifice;\nglobal.sacrificeReset = sacrificeReset;"
);
console.log("  ✓ sacrifice.js (Sacrifice)");

loader.loadAndRegister(
  "tickspeed.js",
  `global.effectiveBaseGalaxies = effectiveBaseGalaxies;
global.getTickSpeedMultiplier = getTickSpeedMultiplier;`
);
console.log("  ✓ tickspeed.js");

loader.loadAndRegister(
  "galaxy.js",
  `global.Galaxy = Galaxy;
global.GALAXY_TYPE = GALAXY_TYPE;`
);
console.log("  ✓ galaxy.js");

console.log("\nGenerating fixtures...\n");

// ============================================================
// Fixture generation
// ============================================================

const fixtures = {
  _meta: {
    generated: new Date().toISOString(),
    source: "Antimatter Dimensions JS (pre-infinity, fresh game)",
    description:
      "Reference values computed by running the actual game code " +
      "with all upgrades/effects inactive (pre-infinity state).",
  },
};

// --- Helper to reset player to a fresh state ---
function resetPlayer() {
  player.galaxies = 0;
  player.dimensionBoosts = 0;
  player.totalTickBought = 0;
  player.sacrificed = new Decimal(0);
  player.chall8TotalSacrifice = new Decimal(1);
  player.break = false;
  for (let i = 0; i < 9; i++) {
    player.dimensions.antimatter[i] = {
      bought: 0,
      costBumps: 0,
      amount: new Decimal(0),
    };
  }
}

function toNum(x) {
  if (x instanceof Decimal) return x.toNumber();
  return x;
}

function toStr(x) {
  if (x instanceof Decimal) return x.toString();
  return String(x);
}

// ============================================================
// Section 1: Dimension Costs
// ============================================================
console.log("Section 1: Dimension Costs");
fixtures.dimension_costs = [];

const tiers = [1, 2, 3, 4, 5, 6, 7, 8];
const boughtValues = [0, 1, 9, 10, 11, 19, 20, 30, 50, 80, 100];

for (const tier of tiers) {
  for (const bought of boughtValues) {
    resetPlayer();
    player.dimensions.antimatter[tier - 1] = {
      bought,
      costBumps: 0,
      amount: new Decimal(bought),
    };
    const dim = AntimatterDimension(tier);
    const cost = dim.cost;
    fixtures.dimension_costs.push({
      tier,
      bought,
      cost: toStr(cost),
      log10_cost: cost.log10(),
    });
  }
}
console.log(
  `  ✓ ${fixtures.dimension_costs.length} cost entries`
);

// ============================================================
// Section 2: Buy-10 Multiplier
// ============================================================
console.log("Section 2: Buy-10 Multiplier");
fixtures.buy10_multiplier = [];

// In pre-infinity, buyTenMultiplier = 2
const buyTenMult = AntimatterDimensions.buyTenMultiplier;
fixtures.buy10_multiplier_base = toNum(buyTenMult);

for (const bought of [0, 10, 20, 30, 50, 100]) {
  const groups = Math.floor(bought / 10);
  const mult = Decimal.pow(buyTenMult, groups);
  fixtures.buy10_multiplier.push({
    bought,
    groups,
    multiplier: toStr(mult),
  });
}
console.log(
  `  ✓ ${fixtures.buy10_multiplier.length} buy10 entries`
);

// ============================================================
// Section 3: Dim Boost Multiplier
// ============================================================
console.log("Section 3: Dim Boost Multiplier");
fixtures.dimboost_multiplier = [];

for (const boosts of [0, 1, 2, 4, 5, 8, 10, 15, 20]) {
  for (const tier of [1, 2, 3, 4, 5, 6, 7, 8]) {
    resetPlayer();
    player.dimensionBoosts = boosts;
    const mult = DimBoost.multiplierToNDTier(tier);
    fixtures.dimboost_multiplier.push({
      boosts,
      tier,
      multiplier: toStr(mult),
      log10_multiplier: mult.log10(),
    });
  }
}
console.log(
  `  ✓ ${fixtures.dimboost_multiplier.length} dimboost entries`
);

// ============================================================
// Section 3b: Dim Boost Requirements
// ============================================================
console.log("Section 3b: Dim Boost Requirements");
fixtures.dimboost_requirements = [];

for (const boosts of [0, 1, 2, 3, 4, 5, 6, 10, 15, 20, 50]) {
  resetPlayer();
  player.dimensionBoosts = boosts;
  const req = DimBoost.requirement;
  fixtures.dimboost_requirements.push({
    current_boosts: boosts,
    required_tier: req.tier,
    required_amount: req.amount,
  });
}
console.log(
  `  ✓ ${fixtures.dimboost_requirements.length} requirement entries`
);

// ============================================================
// Section 4: Tickspeed Purchase Multiplier
// ============================================================
console.log("Section 4: Tickspeed");
fixtures.tickspeed_multiplier = [];

for (const galaxies of [0, 1, 2, 3, 4, 5, 10, 15, 20, 30, 50]) {
  resetPlayer();
  player.galaxies = galaxies;
  const mult = getTickSpeedMultiplier();
  fixtures.tickspeed_multiplier.push({
    galaxies,
    multiplier: toStr(mult),
    multiplier_f64: toNum(mult),
  });
}
console.log(
  `  ✓ ${fixtures.tickspeed_multiplier.length} tickspeed entries`
);

// ============================================================
// Section 5: Galaxy Requirements
// ============================================================
console.log("Section 5: Galaxy Requirements");
fixtures.galaxy_requirements = [];

for (const galaxies of [0, 1, 2, 5, 10, 20, 50, 99]) {
  resetPlayer();
  player.galaxies = galaxies;
  const req = Galaxy.requirementAt(galaxies);
  fixtures.galaxy_requirements.push({
    galaxies,
    required_tier: req.tier,
    required_amount: req.amount,
  });
}
console.log(
  `  ✓ ${fixtures.galaxy_requirements.length} galaxy entries`
);

// ============================================================
// Section 6: Sacrifice
// ============================================================
console.log("Section 6: Sacrifice");
fixtures.sacrifice = {};

// 6a: totalBoost at various sacrificed amounts
fixtures.sacrifice.total_boost = [];
for (const logSacrificed of [0, 1, 10, 20, 50, 100, 200, 1000]) {
  resetPlayer();
  player.dimensionBoosts = 5;
  player.sacrificed = Decimal.pow10(logSacrificed);
  const boost = Sacrifice.totalBoost;
  fixtures.sacrifice.total_boost.push({
    log10_sacrificed: logSacrificed,
    total_boost: toStr(boost),
    log10_total_boost: boost.gt(0) ? boost.log10() : null,
  });
}
console.log(
  `  ✓ ${fixtures.sacrifice.total_boost.length} totalBoost entries`
);

// 6b: nextBoost at various AD1/sacrificed combinations
fixtures.sacrifice.next_boost = [];
const nextBoostCases = [
  { log_ad1: 20, log_sacrificed: 0 },
  { log_ad1: 50, log_sacrificed: 0 },
  { log_ad1: 100, log_sacrificed: 50 },
  { log_ad1: 200, log_sacrificed: 100 },
  { log_ad1: 50, log_sacrificed: 100 },
  { log_ad1: 100, log_sacrificed: 100 },
  { log_ad1: 300, log_sacrificed: 100 },
];
for (const { log_ad1, log_sacrificed } of nextBoostCases) {
  resetPlayer();
  player.dimensionBoosts = 5;
  player.dimensions.antimatter[0] = {
    bought: 0,
    costBumps: 0,
    amount: Decimal.pow10(log_ad1),
  };
  player.sacrificed = Decimal.pow10(log_sacrificed);
  const boost = Sacrifice.nextBoost;
  fixtures.sacrifice.next_boost.push({
    log10_ad1: log_ad1,
    log10_sacrificed: log_sacrificed,
    next_boost: toStr(boost),
  });
}
console.log(
  `  ✓ ${fixtures.sacrifice.next_boost.length} nextBoost entries`
);

// ============================================================
// Section 7: Dimension multiplier (full formula)
// ============================================================
console.log("Section 7: Dimension Multipliers");
fixtures.dimension_multipliers = [];

const multCases = [
  { tier: 1, boosts: 0, bought: 0, galaxies: 0 },
  { tier: 1, boosts: 0, bought: 10, galaxies: 0 },
  { tier: 1, boosts: 4, bought: 20, galaxies: 0 },
  { tier: 1, boosts: 10, bought: 50, galaxies: 0 },
  { tier: 4, boosts: 4, bought: 30, galaxies: 0 },
  { tier: 8, boosts: 10, bought: 0, galaxies: 0 },
  { tier: 8, boosts: 10, bought: 80, galaxies: 5 },
];

for (const c of multCases) {
  resetPlayer();
  player.dimensionBoosts = c.boosts;
  player.galaxies = c.galaxies;
  player.dimensions.antimatter[c.tier - 1] = {
    bought: c.bought,
    costBumps: 0,
    amount: new Decimal(c.bought),
  };
  // For tier 8, set sacrifice_boost
  if (c.tier === 8) {
    player.sacrificed = new Decimal(0);
  }

  // Get the full multiplier via the game's function
  const mult = getDimensionFinalMultiplierUncached(c.tier);
  fixtures.dimension_multipliers.push({
    tier: c.tier,
    boosts: c.boosts,
    bought: c.bought,
    galaxies: c.galaxies,
    multiplier: toStr(mult),
    log10_multiplier: mult.gt(0) ? mult.log10() : null,
  });
}
console.log(
  `  ✓ ${fixtures.dimension_multipliers.length} multiplier entries`
);

// ============================================================
// Write output
// ============================================================
const outPath = path.join(__dirname, "..", "fixtures", "pre-infinity.json");
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, JSON.stringify(fixtures, null, 2) + "\n");
console.log(`\n✅ Fixtures written to ${outPath}`);
console.log(
  `   Total entries: ${
    fixtures.dimension_costs.length +
    fixtures.buy10_multiplier.length +
    fixtures.dimboost_multiplier.length +
    fixtures.dimboost_requirements.length +
    fixtures.tickspeed_multiplier.length +
    fixtures.galaxy_requirements.length +
    fixtures.sacrifice.total_boost.length +
    fixtures.sacrifice.next_boost.length +
    fixtures.dimension_multipliers.length
  }`
);
