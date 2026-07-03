// Display data for the 12 Normal Challenges: id, name (shown by the header's
// challenge display as "<name> Challenge"), description, reward, and the
// Infinities needed to unlock (lockedAt). Strings live frontend-side (same split
// as data/achievements.js and data/infinityUpgrades.js); the engine owns the
// run/complete/unlock state and the rule modifiers. Text is taken verbatim from
// `secret-formula/challenges/normal-challenges.js` (formatInt(...) inlined).

export const NORMAL_CHALLENGES = [
  {
    id: 1,
    name: "1st Antimatter Dimension Autobuyer",
    description: "reach Infinity for the first time.",
    reward: "Upgradeable 1st Antimatter Dimension Autobuyer",
    lockedAt: 0,
  },
  {
    id: 2,
    name: "2nd Antimatter Dimension Autobuyer",
    description:
      "buying Antimatter Dimensions or Tickspeed upgrades halts production of all " +
      "Antimatter Dimensions. Production gradually returns to normal over 3 minutes.",
    reward: "Upgradeable 2nd Antimatter Dimension Autobuyer",
    lockedAt: 0,
  },
  {
    id: 3,
    name: "3rd Antimatter Dimension",
    description:
      "the 1st Antimatter Dimension is heavily weakened, but gets an uncapped " +
      "exponentially increasing multiplier. This multiplier resets after Dimension " +
      "Boosts and Antimatter Galaxies.",
    reward: "Upgradeable 3rd Antimatter Dimension Autobuyer",
    lockedAt: 0,
  },
  {
    id: 4,
    name: "4th Antimatter Dimension Autobuyer",
    description:
      "buying an Antimatter Dimension automatically erases all lower tier Antimatter " +
      "Dimensions, like a sacrifice without the boost.",
    reward: "Upgradeable 4th Antimatter Dimension Autobuyer",
    lockedAt: 0,
  },
  {
    id: 5,
    name: "5th Antimatter Dimension Autobuyer",
    description: "the Tickspeed purchase multiplier starts at ×1.080 instead of ×1.1245.",
    reward: "Upgradeable 5th Antimatter Dimension Autobuyer",
    lockedAt: 0,
  },
  {
    id: 6,
    name: "6th Antimatter Dimension Autobuyer",
    description:
      "upgrading each Antimatter Dimension costs the Antimatter Dimension 2 tiers " +
      "below it instead of antimatter. Antimatter Dimension prices are modified.",
    reward: "Upgradeable 6th Antimatter Dimension Autobuyer",
    lockedAt: 0,
  },
  {
    id: 7,
    name: "7th Antimatter Dimension Autobuyer",
    description:
      "the multiplier from buying 10 Antimatter Dimensions is reduced to ×1. This " +
      "increases by ×0.2 per Dimension Boost, to a maximum of ×2, and is unaffected " +
      "by any upgrades.",
    reward: "Upgradeable 7th Antimatter Dimension Autobuyer",
    lockedAt: 0,
  },
  {
    id: 8,
    name: "8th Antimatter Dimension Autobuyer",
    description:
      "Dimension Boosts provide no multiplier and Antimatter Galaxies cannot be " +
      "bought. Dimensional Sacrifice resets antimatter and all Antimatter Dimensions, " +
      "but also gives a significantly stronger multiplier.",
    reward: "Upgradeable 8th Antimatter Dimension Autobuyer",
    lockedAt: 0,
  },
  {
    id: 9,
    name: "Tickspeed Autobuyer",
    description:
      "whenever you buy Tickspeed upgrades or 10 of an Antimatter Dimension, " +
      "everything else of equal cost will increase to its next cost step.",
    reward: "Upgradeable Tickspeed Autobuyer",
    lockedAt: 0,
  },
  {
    id: 10,
    name: "Automated Dimension Boosts",
    description:
      "there are only 6 Antimatter Dimensions. Dimension Boost and Antimatter Galaxy " +
      "costs are modified.",
    reward: "Dimension Boosts Autobuyer",
    lockedAt: 16,
  },
  {
    id: 11,
    name: "Automated Antimatter Galaxies",
    description:
      "there is normal matter which rises once you have at least 1 2nd Antimatter " +
      "Dimension. If it exceeds your antimatter, it will Dimension Boost without " +
      "giving the bonus.",
    reward: "Antimatter Galaxies Autobuyer",
    lockedAt: 16,
  },
  {
    id: 12,
    name: "Automated Big Crunches",
    description:
      "each Antimatter Dimension produces the Dimension 2 tiers below it instead of " +
      "1. Both 1st and 2nd Dimensions produce antimatter. The 2nd, 4th, and 6th " +
      "Dimensions are made stronger to compensate.",
    reward: "Big Crunches Autobuyer",
    lockedAt: 16,
  },
];
