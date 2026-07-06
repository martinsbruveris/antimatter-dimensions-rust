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
