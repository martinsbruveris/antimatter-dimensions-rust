// shims.js — Minimal global shims for running Antimatter Dimensions
// game code in a pre-infinity context.
//
// In pre-infinity, most upgrades/effects/challenges are inactive.
// These shims make all the globals the game code references resolve
// to their default/inactive values.

const path = require("path");
const Decimal = require(
  path.resolve(__dirname, "../../../../antimatter-dimensions/node_modules/break_infinity.js")
);

// Make Decimal available globally (the game code expects it)
global.Decimal = Decimal;

// --- DC: Decimal Constants ---
// Loaded from the real source via loadGameSource(), but we need a
// few constants available during shim setup.

// --- Inactive effect source ---
// An object that has canBeApplied=false, so applyEffect() is a no-op.
// This is what all upgrades/achievements/etc return in pre-infinity.
function inactiveEffect(defaultVal) {
  return {
    canBeApplied: false,
    isEffectActive: false,
    isEffectConditionSatisfied: false,
    effectValue: defaultVal !== undefined ? defaultVal : 0,
    effectOrDefault(def) { return def; },
    applyEffect(_fn) { /* no-op */ },
    get isUnlocked() { return false; },
    get isBought() { return false; },
    get isCompleted() { return false; },
    get isRunning() { return false; },
    get reward() { return inactiveEffect(); },
    get chargedEffect() { return inactiveEffect(); },
    get config() { return { effect: 0 }; },
    get currentMult() { return new Decimal(1); },
    // Nested effects (e.g. Achievement(141).effects.buyTenMult)
    get effects() {
      return new Proxy({}, {
        get() { return inactiveEffect(); }
      });
    },
    // Decimal-compatible methods used by timesEffectsOf etc.
    toDecimal() { return new Decimal(defaultVal !== undefined ? defaultVal : 1); },
    milestones: new Proxy([], {
      get(_target, prop) {
        if (prop === "length") return 0;
        return inactiveEffect();
      }
    }),
  };
}

// --- Player state (fresh game) ---
const player = {
  galaxies: 0,
  dimensionBoosts: 0,
  totalTickBought: 0,
  sacrificed: new Decimal(0),
  chall2Pow: 1,
  chall8TotalSacrifice: new Decimal(1),
  break: false,
  dilation: {
    active: false,
    totalTachyonGalaxies: 0,
  },
  dimensions: {
    antimatter: Array.from({ length: 9 }, (_, i) => ({
      bought: 0,
      costBumps: 0,
      amount: new Decimal(0),
    })),
  },
  timestudy: { studies: [] },
  records: {
    thisInfinity: {
      maxAM: new Decimal(0),
      time: 0,
      lastBuyTime: 0,
    },
  },
  requirementChecks: {
    infinity: { noSacrifice: true },
    permanent: { singleTickspeed: 0 },
  },
};
global.player = player;

// --- Effects system ---
// In pre-infinity all effect sources are inactive, so Effects
// operations return their identity values.
global.Effects = {
  sum(..._sources) { return 0; },
  product(..._sources) { return 1; },
  max(defaultValue, ..._sources) { return defaultValue; },
  min(defaultValue, ..._sources) { return defaultValue; },
  last(defaultValue, ..._sources) { return defaultValue; },
};

// --- Challenge / upgrade stubs ---
function makeChallenge(isRunning = false) {
  const base = inactiveEffect();
  return Object.create(base, {
    isRunning: { value: isRunning, configurable: true },
    isCompleted: { value: false, configurable: true },
    reward: {
      get() { return inactiveEffect(); },
      configurable: true,
    },
  });
}

global.NormalChallenge = (_n) => makeChallenge(false);
global.InfinityChallenge = (_n) => makeChallenge(false);
global.EternityChallenge = (_n) => makeChallenge(false);

global.Achievement = (_n) => inactiveEffect();
global.TimeStudy = (_n) => inactiveEffect();
global.InfinityUpgrade = new Proxy({}, {
  get(_target, _prop) { return inactiveEffect(); }
});
global.BreakInfinityUpgrade = new Proxy({}, {
  get(_target, _prop) { return inactiveEffect(); }
});
global.Achievements = { power: new Decimal(1) };
global.ShopPurchase = new Proxy({}, {
  get(_target, _prop) { return inactiveEffect(); }
});
global.Currency = {
  antimatter: {
    value: new Decimal(10),
    lt(x) { return this.value.lt(x); },
    subtract(x) { this.value = this.value.sub(x); },
    reset() { this.value = new Decimal(0); },
  },
  infinityPower: { value: new Decimal(0) },
  realityMachines: { value: new Decimal(0) },
};
global.InfinityDimensions = {
  powerConversionRate: 7,
};

// Replicanti stubs
global.Replicanti = {
  galaxies: { bought: 0, extra: 0 },
  amount: new Decimal(0),
};
global.ReplicantiUpgrade = {
  galaxies: { value: 0 },
};

global.GalaxyGenerator = { galaxies: 0 };
global.AlchemyResource = new Proxy({}, {
  get() { return inactiveEffect(); }
});
global.GlyphEffect = new Proxy({}, {
  get() { return inactiveEffect(); }
});
global.GlyphAlteration = { isAdded: () => false };
global.GlyphSacrifice = new Proxy({}, {
  get() { return inactiveEffect(); }
});
global.PelleUpgrade = new Proxy({}, {
  get() { return inactiveEffect(); }
});
global.PelleRifts = new Proxy({}, {
  get() { return inactiveEffect(); }
});
global.PelleStrikes = new Proxy({}, {
  get() { return { hasStrike: false }; }
});
global.Pelle = {
  isDoomed: false,
  specialGlyphEffect: { power: 1 },
};
global.DilationUpgrade = new Proxy({}, {
  get() { return inactiveEffect(); }
});
global.ImaginaryUpgrade = (_n) => inactiveEffect();
global.Enslaved = { isRunning: false };
global.Effarig = { isRunning: false };
global.V = { isRunning: false };
global.VUnlocks = new Proxy({}, {
  get() { return inactiveEffect(); }
});
global.Ra = { isRunning: false, momentumValue: 1 };
global.Laitela = { continuumActive: false };
global.Teresa = inactiveEffect();
global.Perk = new Proxy({}, {
  get() { return inactiveEffect(); }
});
global.PlayerProgress = { realityUnlocked: () => false };
global.EternityMilestone = new Proxy({}, {
  get() { return { isReached: false }; }
});
global.RealityUpgrade = (_n) => inactiveEffect();

global.Player = {
  dimensionMultDecrease: 10,
  infinityLimit: new Decimal("1e308"),
  isInAntimatterChallenge: false,
};

// GameCache stub - provides cached values
global.GameCache = {
  antimatterDimensionCommonMultiplier: { value: new Decimal(1) },
  dimensionMultDecrease: { value: 10, invalidate() {} },
};

// Tutorial / EventHub stubs
global.Tutorial = { turnOffEffect() {} };
global.TUTORIAL_STATE = {};
global.EventHub = { dispatch() {} };
global.GAME_EVENT = {};
global.PRESTIGE_EVENT = { DIMENSION_BOOST: 0, ANTIMATTER_GALAXY: 1 };

// Tickspeed stub (will be overwritten by real code)
global.Tickspeed = {
  isAvailableForPurchase: true,
  isAffordable: true,
  cost: new Decimal(1000),
};

// Formatting stubs
global.formatInt = (n) => String(n);
global.formatX = (n) => `×${n}`;
global.formatPow = (n) => `^${n}`;

// Polyfills the game uses
if (!Array.range) {
  Array.range = function(start, count) {
    return Array.from({ length: count }, (_, i) => start + i);
  };
}
if (!Array.prototype.sum) {
  Array.prototype.sum = function() {
    return this.reduce((a, b) => a + b, 0);
  };
}
if (!Number.prototype.clampMin) {
  Number.prototype.clampMin = function(min) {
    return Math.max(this, min);
  };
}
if (!Number.prototype.clampMax) {
  Number.prototype.clampMax = function(max) {
    return Math.min(this, max);
  };
}
if (!Number.prototype.clamp) {
  Number.prototype.clamp = function(min, max) {
    return Math.max(min, Math.min(max, this));
  };
}
Math.clampMin = function(value, min) {
  return Math.max(value, min);
};
Math.clampMax = function(value, max) {
  return Math.min(value, max);
};

if (!Array.prototype.compact) {
  Array.prototype.compact = function() {
    return this.filter(x => x !== null && x !== undefined);
  };
}
if (!Number.prototype.toDecimal) {
  Number.prototype.toDecimal = function() {
    return new Decimal(this.valueOf());
  };
}


// getAdjustedGlyphEffect — returns 1 for multiplication, 0 for
// addition effects
global.getAdjustedGlyphEffect = (_name) => 1;
global.getSecondaryGlyphEffect = (_name) => 1;
global.dilatedValueOf = (v) => v;

// DC constants stub (will be replaced by real loading)
global.DC = {};

module.exports = { player, Decimal, inactiveEffect };
