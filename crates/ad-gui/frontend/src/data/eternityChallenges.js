// Eternity Challenge display data (descriptions/rewards live frontend-side;
// the engine owns goals, completions, and run state). From
// secret-formula/challenges/eternity-challenges.js (pre-Pelle text).
export const ETERNITY_CHALLENGES = [
  {
    id: 1,
    description: "Time Dimensions are disabled.",
    reward: "Time Dimension multiplier based on time spent this Eternity",
  },
  {
    id: 2,
    description: "Infinity Dimensions are disabled.",
    reward: "1st Infinity Dimension multiplier based on Infinity Power",
  },
  {
    id: 3,
    description:
      "Antimatter Dimensions 5-8 don't produce anything. Dimensional Sacrifice is disabled.",
    reward: "Increase the multiplier for buying 10 Antimatter Dimensions",
  },
  {
    id: 4,
    description:
      "All Infinity multipliers and generators are disabled. The goal must be reached within a certain number of Infinities or else you will fail the Challenge.",
    reward: "Infinity Dimension multiplier based on unspent IP",
    restriction: (completions) => {
      const limit = Math.max(16 - 4 * completions, 0);
      return limit === 0 ? "without any Infinities" : `in ${limit} Infinities or less`;
    },
  },
  {
    id: 5,
    description:
      "Antimatter Galaxy cost increase scaling starts immediately (normally at 100 Galaxies). Dimension Boost costs scaling is massively increased.",
    reward: "Distant Galaxy cost scaling starts later",
  },
  {
    id: 6,
    description:
      "You cannot gain Antimatter Galaxies normally. The cost of upgrading your max Replicanti Galaxies is massively reduced.",
    reward: "Further reduce Antimatter Dimension cost multiplier growth",
  },
  {
    id: 7,
    description:
      "1st Time Dimensions produce 8th Infinity Dimensions and 1st Infinity Dimensions produce 7th Antimatter Dimensions. Tickspeed also directly applies to Infinity and Time Dimensions.",
    reward: "1st Time Dimension produces 8th Infinity Dimensions",
  },
  {
    id: 8,
    description:
      "You can only upgrade Infinity Dimensions 50 times and Replicanti upgrades 40 times. Infinity Dimension and Replicanti upgrade autobuyers are disabled.",
    reward: "Infinity Power strengthens Replicanti Galaxies",
  },
  {
    id: 9,
    description:
      "You cannot buy Tickspeed upgrades. Infinity Power instead multiplies Time Dimensions with greatly reduced effect.",
    reward: "Infinity Dimension multiplier based on Time Shards",
  },
  {
    id: 10,
    description:
      "Time Dimensions and Infinity Dimensions are disabled. You gain an immense boost from Infinities to Antimatter Dimensions (Infinities^950).",
    reward: "Time Dimension multiplier based on Infinities",
  },
  {
    id: 11,
    description:
      "All Dimension multipliers and powers are disabled except for the multipliers from Infinity Power and Dimension Boosts (to Antimatter Dimensions).",
    reward: "Further reduce Tickspeed cost multiplier growth",
  },
  {
    id: 12,
    description:
      "The game runs ×1000 slower. The goal must be reached within a certain amount of time or you will fail the Challenge.",
    reward: "Infinity Dimension cost multipliers are reduced",
    restriction: (completions) => {
      const limit = Math.max(10 - 2 * completions, 1) / 10;
      return `in ${limit} in-game seconds or less`;
    },
  },
];

// Secondary unlock-requirement resource names (ec-time-studies.js).
export const EC_SECONDARY_RESOURCES = {
  1: "Eternities",
  2: "Tickspeed upgrades from Time Dimensions",
  3: "8th Antimatter Dimensions",
  4: "Infinities",
  5: "Antimatter Galaxies",
  6: "Replicanti Galaxies",
  7: "antimatter",
  8: "Infinity Points",
  9: "Infinity Power",
  10: "Eternity Points",
};
