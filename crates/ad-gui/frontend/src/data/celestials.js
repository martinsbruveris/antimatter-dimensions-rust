// Frontend display data for the Celestials tab (Phase 7). Strings live here by
// the project convention: the engine owns state/effects, the frontend owns
// copy. Ids match the engine's save bits / entry ids.

// Teresa threshold unlocks (`secret-formula/celestials/teresa.js`), keyed by id.
export const TERESA_UNLOCK_DESCRIPTIONS = {
  0: "Unlock Teresa's Reality.",
  1: "Unlock passive Eternity Point generation.",
  2: "Unlock Teresa's Perk Point Shop.",
  3: "Unlock Effarig, Celestial of Ancient Relics.",
  4: 'Unlock "Undo" of equipping a Glyph.',
  5: "You start Reality with all Eternity Upgrades unlocked.",
};

// Teresa Perk-Shop rebuyables (`secret-formula/celestials/perk-shop.js`), keyed
// by id. `music`/`fillMusic` (ids 4/5) are unmodelled (music glyphs).
export const PERK_SHOP_DESCRIPTIONS = {
  0: "Increase pre-instability Glyph levels by 5%",
  1: "Double Reality Machine gain",
  2: "Dilation autobuyers buy twice as many Dilation Upgrades at once.",
  3: "Infinity Dimension, Time Dimension, Dilation, and Replicanti autobuyers are 2× faster.",
};

// Teresa's Reality description (`GameDatabase.celestials.descriptions[0]`).
export const TERESA_RUN_DESCRIPTION =
  "Glyph Time Theorem generation is disabled. " +
  "You gain less Infinity Points and Eternity Points (x^0.55).";

// Effarig Relic-Shard unlock descriptions (`secret-formula/celestials/effarig.js`),
// keyed by id. adjuster/glyphFilter/setSaves are glyph-QoL (bit modelled, effect cut).
export const EFFARIG_UNLOCK_DESCRIPTIONS = {
  0: "Adjustable Glyph level factor weights",
  1: "Glyph Filtering",
  2: "Glyph Presets",
  3: "Effarig's Reality",
};

// Effarig stage-reward descriptions (`effarigUnlocks.{infinity,eternity,reality}`),
// keyed by id.
export const EFFARIG_STAGE_DESCRIPTIONS = {
  4: [
    "Replicanti cap is multiplied by a value based on Infinities.",
    "Infinities increase your max Replicanti Galaxies.",
    "Base Infinity Point gain is capped at 1e200 in Effarig's Reality.",
    "Each type of Infinity Point multiplier is capped at 1e50 in Effarig's Reality.",
  ],
  5: [
    "Eternities generate Infinities.",
    "Infinity Points are no longer limited in any way in Effarig's Reality.",
    "You have unlocked The Nameless Ones.",
  ],
  6: [
    "You have unlocked Effarig Glyphs (you may equip at most one and some effects are mutually exclusive).",
  ],
};

export const EFFARIG_STAGE_LABELS = { 4: "Infinity", 5: "Eternity", 6: "Reality" };

// Enslaved unlock descriptions (`ENSLAVED_UNLOCKS`), keyed by id.
export const ENSLAVED_UNLOCK_DESCRIPTIONS = {
  0: "Increase the softcap to Tickspeed upgrades from Time Dimensions by 100,000",
  1: "Unlock The Nameless Ones' Reality (requires a level 5000 Glyph and a 100% rarity Glyph)",
};

// The Nameless Ones' Reality description
// (`GameDatabase.celestials.descriptions[2]`).
export const ENSLAVED_RUN_DESCRIPTION = [
  "Glyph levels are boosted to a minimum of 5000.",
  "Infinity, Time, and 8th Antimatter Dimension purchases are limited to 1 each.",
  "Antimatter Dimension multipliers are always Dilated.",
  "Time Study 192 (uncapped Replicanti) is locked.",
  "The Black Hole is disabled.",
  "Tachyon Particle and Dilated Time production are severely reduced.",
  "Time Theorem generation from Dilation Glyphs is disabled.",
  "Stored game time is discharged at a reduced effectiveness (exponent^0.55).",
];

// Effarig's Reality description (`GameDatabase.celestials.descriptions[1]`).
export const EFFARIG_RUN_DESCRIPTION =
  "All Dimension multipliers, game speed, and tickspeed are severely lowered, like Dilation. " +
  "Infinity Power reduces the production and game speed penalties and Time Shards reduce the " +
  "tickspeed penalty. Glyph levels are temporarily capped, rarity is unaffected.\n" +
  "You will exit Effarig's Reality when you complete a Layer of it for the first time.";

// V main-unlock condition labels (`v.mainUnlock`), in id order.
export const V_MAIN_UNLOCK_LABELS = [
  "Realities",
  "Eternities",
  "Infinities",
  "Dilated Time",
  "Replicanti",
  "Reality Machines",
];

// V-achievement display data (`v.runUnlocks`), keyed by id. `type` drives the
// value formatting: negcount (−value glyphs), int, pow10 (10^value), bh
// (1 / 10^value Black Hole).
export const V_ACHIEVEMENTS = [
  { id: 0, name: "Glyph Knight", type: "negcount" },
  { id: 1, name: "AntiStellar", type: "int" },
  { id: 2, name: "Se7en deadly matters", type: "pow10" },
  { id: 3, name: "Young Boy", type: "pow10" },
  { id: 4, name: "Eternal Sunshine", type: "pow10" },
  { id: 5, name: "Matterception", type: "int" },
  { id: 6, name: "Requiem for a Glyph", type: "negcount" },
  { id: 7, name: "Post-destination", type: "bh" },
  { id: 8, name: "Shutter Glyph", type: "int" },
];

// V ST-gated reward descriptions (`v.unlocks`), keyed by unlock bit id.
export const V_REWARD_DESCRIPTIONS = {
  1: "Spend Perk Points to reduce the goal of all V-Achievement tiers.",
  2: "Antimatter Dimension power based on total Space Theorems.",
  3: "Achievement multiplier reduces Auto-EC completion time.",
  4: "Unlock the ability to Automatically Purge Glyphs on Reality.",
  5: "Achievement multiplier affects Black Hole power.",
  6: "Reduce the Space Theorem cost of Time Studies by 2. Unlock Ra.",
};

// Ra pets (`secret-formula/celestials/ra.js`), keyed by engine index
// (teresa/effarig/enslaved/v). Colours + "based on" tooltips are vendored.
export const RA_PETS = [
  { id: 0, name: "Teresa", color: "#8596ea", chunkGain: "Eternity Points", memoryGain: "current RM" },
  { id: 1, name: "Effarig", color: "#ea8585", chunkGain: "Relic Shards gained", memoryGain: "best Glyph level" },
  { id: 2, name: "Nameless", color: "#f1aa7f", chunkGain: "Time Shards", memoryGain: "total time played" },
  { id: 3, name: "V", color: "#ead584", chunkGain: "Infinity Power", memoryGain: "total Memory levels" },
];

// Ra unlock rewards (`secret-formula/celestials/ra.js`), keyed by unlock id.
export const RA_UNLOCK_DESCRIPTIONS = {
  0: "Tachyon Particles are given immediately when Time Dilation is active.",
  1: "Unlock Charged Infinity Upgrades (one more max every 2 levels).",
  2: "Memory Chunks produce more Memories based on Reality Machines.",
  3: "Unlock Altered Glyphs (new effects based on Glyph Sacrifice).",
  4: "Unlock Effarig's Memories.",
  5: "Purchase caps are raised in Teresa's Perk Point Shop.",
  6: "Gain Tachyon Particles as if reaching √(total antimatter) in Dilation.",
  7: "Get x2 Glyph choices; Relic Shard rarity bonus is always maximal.",
  8: "Unlock Glyph Alchemy (a new Reality tab).",
  9: "Memory Chunks produce more Memories based on highest Glyph level.",
  10: "Glyphs always have 4 effects; Effarig Glyphs up to 7.",
  11: "Unlock Nameless's Memories.",
  12: "Glyph level is increased based on Relic Shards gained.",
  13: "Glyphs always 100% rarity; Glyph Sacrifice raised by Relic Shards.",
  14: "Unlock Black Hole power upgrade autobuyers.",
  15: "Stored game time is amplified; store more real time (Nameless levels).",
  16: "Memory Chunks produce more Memories based on total time played.",
  17: "Black Hole charging uses 99% of game speed; auto-discharge.",
  18: "Unlock V's Memories.",
  19: "Gain more Dilated Time based on peak game speed in each Reality.",
  20: "All basic Glyphs gain the Time-Glyph game-speed effect.",
  21: "Rebuyable Reality upgrades auto-bought; Auto-EC instant.",
  22: "Time Dilation auto-unlocks for free at its Time-Theorem requirement.",
  23: "Memory Chunks produce more Memories based on total Celestial levels.",
  24: "Unlock Hard V-Achievements + a Triad Study every 6 levels.",
  25: "Time Theorems boost all continuous non-dimension production.",
  26: "Achievement multiplier applies to Time Theorem generation.",
  27: "Achievement multiplier is raised ^1.5.",
};

// Glyph Alchemy resources (`secret-formula/celestials/alchemy.js`), keyed by id.
export const ALCHEMY_RESOURCES = [
  { id: 0, name: "Power", symbol: "Ω", effect: "Antimatter Dimension multipliers (power)" },
  { id: 1, name: "Infinity", symbol: "∞", effect: "Infinity Dimension multipliers (power)" },
  { id: 2, name: "Time", symbol: "Δ", effect: "Time Dimension multipliers (power)" },
  { id: 3, name: "Replication", symbol: "Ξ", effect: "Replication speed" },
  { id: 4, name: "Dilation", symbol: "Ψ", effect: "Dilated Time production" },
  { id: 5, name: "Cardinality", symbol: "α", effect: "Reduces Replicanti slowdown above cap" },
  { id: 6, name: "Eternity", symbol: "τ", effect: "Eternity generation (power)" },
  { id: 7, name: "Dimensionality", symbol: "ρ", effect: "Large multiplier to all Dimensions" },
  { id: 8, name: "Inflation", symbol: "λ", effect: "Extra power for very large AD multipliers" },
  { id: 9, name: "Alternation", symbol: "ω", effect: "Tachyon Galaxy strength from Replicanti" },
  { id: 10, name: "Effarig", symbol: "Ϙ", effect: "Relic Shard gain" },
  { id: 11, name: "Synergism", symbol: "π", effect: "Alchemy Reaction efficiency" },
  { id: 12, name: "Momentum", symbol: "μ", effect: "All-Dimension power that grows over time" },
  { id: 13, name: "Decoherence", symbol: "ξ", effect: "Refining spills to all base resources" },
  { id: 14, name: "Exponential", symbol: "Γ", effect: "IP multiplied by Replicanti" },
  { id: 15, name: "Force", symbol: "Φ", effect: "AD multiplied by Reality Machines" },
  { id: 16, name: "Uncountability", symbol: "Θ", effect: "Passive Realities & Perk Points" },
  { id: 17, name: "Boundless", symbol: "Π", effect: "Stronger Tesseracts" },
  { id: 18, name: "Multiversal", symbol: "Σ", effect: "Each Reality simulates more Realities" },
  { id: 19, name: "Unpredictability", symbol: "Λ", effect: "Reactions may trigger twice" },
  { id: 20, name: "Reality", symbol: "Ϟ", effect: "Consumed to create Reality Glyphs" },
];

// Singularity Milestone descriptions (`secret-formula/celestials/
// singularity-milestones.js`), keyed by engine milestone id.
export const MILESTONE_DESCRIPTIONS = [
  "Continuum percentage multiplier",
  "Dark Matter production multiplier",
  "Dark Energy production multiplier",
  "Dark Matter Dimension upgrades are cheaper",
  "Singularity gain multiplier",
  "Dark Matter Dimension interval decrease",
  "Ascension affects Dark Matter production more",
  "Ascension increases the interval less",
  "Automatically condense Singularities above the cap",
  "Dark Matter Dimension Autobuyers",
  "DMD Ascension Autobuyers",
  "Autobuyer speed for all DMD Autobuyers",
  "Dark Energy mult. based on disabled Dimensions",
  "Increased Singularity gain per cap increase",
  "DMD Interval cost scaling is better",
  "Tesseracts boost Dark Matter and Dark Energy",
  "Infinities boost Dark Matter and Dark Energy",
  "Singularities improve the repeatable DT upgrade",
  "Boost Dark Matter/Energy based on Glyph level",
  "Singularities boost game speed",
  "Time Theorems boost Dark Matter and Dark Energy",
  "Annihilation mult. generates the 4th DMD",
  "4th DMD amount boosts Dark Matter and Dark Energy",
  "Unlock an Autobuyer for Annihilation",
  "Singularities give a power effect to Time Theorems",
  "Game speed boosts Dark Matter and Dark Energy",
  "Singularities boost pre-instability Glyph level",
  "Dilated Time boosts Dark Matter production",
  "Infinities gain a power effect from Singularities",
  "Singularities increase effective Tesseract count",
];

// Imaginary Upgrade descriptions (`secret-formula/reality/imaginary-upgrades.js`).
export const IMAGINARY_UPGRADE_DESCRIPTIONS = {
  11: "Time Dimension power based on total antimatter",
  12: "Time Study power based on total time played",
  13: "Multiply the Imaginary Machine cap",
  14: "Antimatter Dimension multiplier based on Dark Matter",
  15: "Convert Antimatter Dimensions to Continuum and unlock Lai'tela",
  16: "Unlock the 2nd Dark Matter Dimension",
  17: "Unlock the 3rd Dark Matter Dimension",
  18: "Unlock the 4th Dark Matter Dimension",
  19: "Unlock Dark Matter Annihilation",
  20: "Autobuyers for repeatable Imaginary Upgrades; ×10 iM speed",
  21: "Annihilation multiplier improved by Imaginary Machines",
  22: "All Glyph Sacrifice totals raised to 1e100",
  23: "Free Dimboost count based on Tesseract count",
  24: "Free Dimboost strength based on Singularity count",
  25: "Unlock Pelle, Celestial of Antimatter",
};

// Imaginary Rebuyable descriptions, keyed by id 1-10.
export const IMAGINARY_REBUYABLE_DESCRIPTIONS = {
  1: "Increase Temporal Amplifier multiplier",
  2: "Increase Replicative Amplifier multiplier",
  3: "Increase Eternal Amplifier multiplier",
  4: "Increase Superluminal Amplifier multiplier",
  5: "Increase Boundless Amplifier multiplier",
  6: "Increase the Reality Machine cap",
  7: "Delay Glyph Instability starting level",
  8: "Multiply Infinity Dimensions",
  9: "Increase Galaxy strength",
  10: "Increase Singularity gain",
};

// Pelle rift names + primary effect labels (`secret-formula/celestials/
// rifts.js`), keyed by engine rift id.
export const PELLE_RIFTS = [
  { id: 0, name: "Vacuum", drain: "IP", effect: "IP gain" },
  { id: 1, name: "Decay", drain: "Replicanti", effect: "Replicanti speed" },
  { id: 2, name: "Chaos", drain: "Decay", effect: "Time Dimensions" },
  { id: 3, name: "Recursion", drain: "EP", effect: "EP formula divisor" },
  { id: 4, name: "Paradox", drain: "Dilated Time", effect: "All Dimensions (power)" },
];

// Pelle rebuyable descriptions, keyed by id 0-4.
export const PELLE_REBUYABLE_DESCRIPTIONS = {
  0: "Multiplier to Antimatter Dimensions",
  1: "Multiplier to game speed",
  2: "Increase the Glyph level allowed in Pelle",
  3: "Increase Infinity Power conversion rate",
  4: "Multiply Galaxy power",
};

// Pelle one-time upgrade descriptions (`secret-formula/celestials/
// pelle-upgrades.js`), keyed by id 0-22.
export const PELLE_UPGRADE_DESCRIPTIONS = {
  0: "Permanent Autobuyers for Antimatter Dimensions 1-4",
  1: "Permanent Autobuyer for Dimension Boosts",
  2: "Autobuyer upgrades no longer reset on Armageddon",
  3: "Permanent Autobuyers for Antimatter Dimensions 5-8",
  4: "Permanent Autobuyer for Antimatter Galaxies",
  5: "Permanent Autobuyer for Tickspeed upgrades",
  6: "Infinity Upgrades no longer reset on Armageddon",
  7: "Dimension Boosts no longer reset anything",
  8: "Break Infinity Upgrades no longer reset on Armageddon",
  9: "Permanent Autobuyers for Infinity Dimensions",
  10: "Infinity Challenges no longer reset on Armageddon",
  11: "Galaxies no longer reset Dimension Boosts",
  12: "Permanent Autobuyers for Replicanti Upgrades",
  13: "Replicanti Galaxies no longer reset on Infinity",
  14: "Eternities no longer reset on Armageddon",
  15: "Time Studies and Theorems no longer reset on Armageddon",
  16: "Replicanti is permanently unlocked",
  17: "Eternity Upgrades no longer reset on Armageddon",
  18: "Permanent Autobuyers for Time Dimensions",
  19: "Eternity Challenges no longer reset on Armageddon",
  20: "Dilation Upgrades no longer reset on Armageddon",
  21: "Tachyon Particles no longer reset on Armageddon",
  22: "Replicanti Galaxies no longer reset anything",
};
