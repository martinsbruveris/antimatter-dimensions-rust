// Static display data for the Break Infinity Upgrades (Feature 2.3). Owned state,
// affordability, and cost come from the engine snapshot (`game.break_infinity`);
// only the description text lives here, keyed on the save id (one-time upgrades)
// or index (rebuyables). `deferred: true` marks upgrades whose gameplay effect is
// not wired yet (they can still be bought and persist).

export const BREAK_INFINITY_UPGRADES = [
  {
    id: "totalMult",
    description:
      "Antimatter Dimensions gain a multiplier based on total antimatter produced",
  },
  {
    id: "currentMult",
    description:
      "Antimatter Dimensions gain a multiplier based on current antimatter",
  },
  { id: "postGalaxy", description: "All Galaxies are 50% stronger" },
  {
    id: "infinitiedMult",
    description: "Antimatter Dimensions gain a multiplier based on Infinities",
  },
  {
    id: "achievementMult",
    description:
      "Antimatter Dimensions gain a multiplier based on Achievements completed",
  },
  {
    id: "challengeMult",
    description:
      "Antimatter Dimensions gain a multiplier based on your slowest challenge run",
    deferred: true,
  },
  {
    id: "infinitiedGeneration",
    description: "Passively generate Infinities based on your fastest Infinity",
    deferred: true,
  },
  {
    id: "autobuyMaxDimboosts",
    description: "Unlock the buy-max Dimension Boost Autobuyer mode",
    deferred: true,
  },
  {
    id: "autoBuyerUpgrade",
    description: "Autobuyers unlocked by Normal Challenges work twice as fast",
  },
];

export const BREAK_INFINITY_REBUYABLES = [
  {
    id: 0,
    description: "Reduce the post-infinity Tickspeed cost multiplier scaling",
    deferred: true,
  },
  {
    id: 1,
    description:
      "Reduce the post-infinity Antimatter Dimension cost multiplier scaling",
    deferred: true,
  },
  {
    id: 2,
    description: "Passively generate a percentage of your best IP/min",
    deferred: true,
  },
];
