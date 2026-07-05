// Glyph display data, vendored from the original's constants.js
// (GLYPH_SYMBOLS, GlyphRarities, glyphTypes colors) and
// secret-formula/reality/glyph-effects.js (descriptions). Strings and colors
// live frontend-side by design; the engine owns the values.
import { formatDecimal } from "../util/format";

export const GLYPH_TYPES = {
  power: { symbol: "Ω", color: "#22aa48", adjective: "Power" },
  infinity: { symbol: "∞", color: "#b67f33", adjective: "Infinity" },
  replication: { symbol: "Ξ", color: "#03a9f4", adjective: "Replication" },
  time: { symbol: "Δ", color: "#b241e3", adjective: "Time" },
  dilation: { symbol: "Ψ", color: "#64dd17", adjective: "Dilation" },
  companion: { symbol: "♥", color: "#feaec9", adjective: "Companion" },
};

// GlyphRarities (dark-theme colors), descending by minimum strength.
export const GLYPH_RARITIES = [
  { minStrength: 3.5, name: "Celestial", color: "#3d3dec" },
  { minStrength: 3.25, name: "Transcendent", color: "#03ffec" },
  { minStrength: 3, name: "Mythical", color: "#d50000" },
  { minStrength: 2.75, name: "Legendary", color: "#ff9800" },
  { minStrength: 2.5, name: "Epic", color: "#9c27b0" },
  { minStrength: 2, name: "Rare", color: "#5096f3" },
  { minStrength: 1.5, name: "Uncommon", color: "#43a047" },
  { minStrength: 1, name: "Common", color: "#ffffff" },
];

export function rarityOf(strength) {
  return GLYPH_RARITIES.find((r) => strength >= r.minStrength) ??
    GLYPH_RARITIES[GLYPH_RARITIES.length - 1];
}

export function strengthToRarityPercent(strength) {
  return ((strength - 1) * 100) / 2.5;
}

const f = (x, p = 3, p1000 = 3) => formatDecimal(x, p, p1000);
const minusOne = (x) => ({ m: x.m - (x.e === 0 ? 1 : 0), e: x.e });
// Numeric value of a Num for small quantities (safe below ~1e300).
const toNumber = (x) => x.m * Math.pow(10, x.e);

// Per-effect display config, keyed by the generated-effect bit. `single` is
// the description on one glyph's tooltip; `total` is the combined-effects
// panel line. Both receive a Num.
export const GLYPH_EFFECTS = {
  0: {
    id: "timepow",
    single: (x) => `Time Dimension power +${f(minusOne(x), 3, 3)}`,
    total: (x) => `Time Dimension multipliers ^${f(x, 3, 3)}`,
  },
  1: {
    id: "timespeed",
    single: (x) => `Multiply game speed by ${f(x, 3, 3)}`,
    total: (x) => `Game runs ×${f(x, 3, 3)} faster`,
  },
  2: {
    id: "timeetermult",
    single: (x) => `Multiply Eternity gain by ${f(x, 2, 2)}`,
    total: (x) => `Eternity gain ×${f(x, 2, 2)}`,
  },
  3: {
    id: "timeEP",
    single: (x) => `Multiply Eternity Point gain by ${f(x, 2, 3)}`,
    total: (x) => `Eternity Point gain ×${f(x, 2, 3)}`,
  },
  4: {
    id: "dilationDT",
    single: (x) => `Multiply Dilated Time gain by ${f(x, 2, 1)}`,
    total: (x) => `Dilated Time gain ×${f(x, 2, 1)}`,
  },
  5: {
    id: "dilationgalaxyThreshold",
    single: (x) => `Tachyon Galaxy threshold multiplier ×${f(x, 3, 3)}`,
    total: (x) => `Tachyon Galaxy threshold multiplier ×${f(x, 3, 3)}`,
  },
  6: {
    id: "dilationTTgen",
    single: (x) =>
      `Generates ${f({ m: x.m * 3.6, e: x.e + 3 }, 2, 2)} Time Theorems per hour`,
    total: (x) =>
      `Generating ${f({ m: x.m * 3.6, e: x.e + 3 }, 2, 2)} Time Theorems per hour`,
  },
  7: {
    id: "dilationpow",
    single: (x) => `Antimatter Dimension power +${f(minusOne(x), 2, 2)} while Dilated`,
    total: (x) => `Antimatter Dimension multipliers ^${f(x, 2, 2)} while Dilated`,
  },
  8: {
    id: "replicationspeed",
    single: (x) => `Multiply Replication speed by ${f(x, 2, 1)}`,
    total: (x) => `Replication speed ×${f(x, 2, 1)}`,
  },
  9: {
    id: "replicationpow",
    single: (x) => `Replicanti multiplier power +${f(minusOne(x), 2, 2)}`,
    total: (x) => `Replicanti multiplier ^${f(x, 2, 2)}`,
  },
  10: {
    id: "replicationdtgain",
    single: (x) =>
      `Multiply Dilated Time gain by +${f({ m: x.m, e: x.e + 4 }, 2, 2)} per 1e10,000 replicanti`,
    total: (x) =>
      `Multiply Dilated Time gain by +${f({ m: x.m, e: x.e + 4 }, 2, 2)} per 1e10,000 replicanti`,
  },
  11: {
    id: "replicationglyphlevel",
    single: (x) => `Replicanti factor for Glyph level: ^0.4 ➜ ^(0.4 + ${f(x, 3, 3)})`,
    total: (x) => `Replicanti factor for Glyph level: ^0.4 ➜ ^(0.4 + ${f(x, 3, 3)})`,
  },
  12: {
    id: "infinitypow",
    single: (x) => `Infinity Dimension power +${f(minusOne(x), 3, 3)}`,
    total: (x) => `Infinity Dimension multipliers ^${f(x, 3, 3)}`,
  },
  13: {
    id: "infinityrate",
    single: (x) => `Infinity Power conversion rate: ^7 ➜ ^(7 + ${f(x, 2, 2)})`,
    total: (x) => `Infinity Power conversion rate: ^7 ➜ ^(7 + ${f(x, 2, 2)})`,
  },
  14: {
    id: "infinityIP",
    single: (x) => `Multiply Infinity Point gain by ${f(x, 2, 3)}`,
    total: (x) => `Infinity Point gain ×${f(x, 2, 3)}`,
  },
  15: {
    id: "infinityinfmult",
    single: (x) => `Multiply Infinity gain by ${f(x, 2, 1)}`,
    total: (x) => `Infinity gain ×${f(x, 2, 1)}`,
  },
  16: {
    id: "powerpow",
    single: (x) => `Antimatter Dimension power +${f(minusOne(x), 3, 3)}`,
    total: (x) => `Antimatter Dimension multipliers ^${f(x, 3, 3)}`,
  },
  17: {
    id: "powermult",
    single: (x) => `Antimatter Dimension multipliers ×${f(x, 2, 0)}`,
    total: (x) => `Antimatter Dimension multipliers ×${f(x, 2, 0)}`,
  },
  18: {
    id: "powerdimboost",
    single: (x) => `Dimension Boost multiplier ×${f(x, 2, 2)}`,
    total: (x) => `Dimension Boost multiplier ×${f(x, 2, 2)}`,
  },
  19: {
    id: "powerbuy10",
    single: (x) => `Increase the bonus from buying 10 Antimatter Dimensions by ${f(x, 2, 2)}`,
    total: (x) => `Multiplier from "Buy 10" ×${f(x, 2, 2)}`,
  },
};

// The companion glyph's fixed flavour text (bits 8/9 in its own space).
export const COMPANION_TEXT = [
  "It does nothing but sit there and cutely smile at you, whisper into your " +
    "dreams politely, and plot the demise of all who stand against you. " +
    "This one-of-a-kind Glyph will never leave you.",
  "Thanks for your dedication for the game! You reached this Eternity Point " +
    "amount on your first Reality.",
];

// The sacrifice-boost descriptions (BASIC_GLYPH_TYPES order matches the
// snapshot's sac_totals / sac_effects arrays).
export const SACRIFICE_DESCRIPTIONS = [
  (v) => `Distant Galaxy scaling starts ${v.toFixed(0)} later`,
  (v) => `×${v.toFixed(2)} bigger multiplier when buying 8th Infinity Dimension`,
  (v) => `Replicanti Galaxy scaling starts ${v.toFixed(0)} later`,
  (v) => `×${v.toFixed(2)} bigger multiplier when buying 8th Time Dimension`,
  (v) => `Multiply Tachyon Particle gain by ×${v.toFixed(2)}`,
];

export const BASIC_TYPE_ORDER = ["power", "infinity", "replication", "time", "dilation"];

export { toNumber };
