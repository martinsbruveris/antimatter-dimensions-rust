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
