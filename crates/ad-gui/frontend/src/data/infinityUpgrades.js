// Display data for the Infinity Upgrades grid: the 4×4 layout, descriptions, and
// how each tile renders its effect line. The engine (ad-core) owns owned-state,
// affordability, cost, and the numeric effect *value*; these strings live
// frontend-side by design (same split as data/achievements.js). Verified against
// `secret-formula/infinity/infinity-upgrades.js`.
//
// `effect` describes the tile's effect line:
//   { kind: "mult", places, under } → "×" + formatDecimal(view.effect, places, under)
//   { kind: "text", text }          → a static string (e.g. "×2 ➜ ×2.2")
//   undefined                        → no effect line (the number is in the description)
//
// Columns are top-to-bottom purchase chains, matching the engine's column-major
// `ALL_INFINITY_UPGRADES` order (so column c is snapshot indices c*4 .. c*4+4).

export const INFINITY_UPGRADE_COLUMNS = [
  // Column 0
  [
    {
      id: "timeMult",
      description: "Antimatter Dimensions gain a multiplier based on time played",
      effect: { kind: "mult", places: 2, under: 2 },
    },
    {
      id: "18Mult",
      description: "1st and 8th Antimatter Dimensions gain a multiplier based on Infinities",
      effect: { kind: "mult", places: 1, under: 1 },
    },
    {
      id: "36Mult",
      description: "3rd and 6th Antimatter Dimensions gain a multiplier based on Infinities",
      effect: { kind: "mult", places: 1, under: 1 },
    },
    {
      id: "resetBoost",
      description:
        "Decrease the number of Dimensions needed for Dimension Boosts and Antimatter Galaxies by 9",
    },
  ],
  // Column 1
  [
    {
      id: "dimMult",
      description: "Increase the multiplier for buying 10 Antimatter Dimensions",
      effect: { kind: "text", text: "×2 ➜ ×2.2" },
    },
    {
      id: "27Mult",
      description: "2nd and 7th Antimatter Dimensions gain a multiplier based on Infinities",
      effect: { kind: "mult", places: 1, under: 1 },
    },
    {
      id: "45Mult",
      description: "4th and 5th Antimatter Dimensions gain a multiplier based on Infinities",
      effect: { kind: "mult", places: 1, under: 1 },
    },
    {
      id: "galaxyBoost",
      description: "All Galaxies are twice as strong",
    },
  ],
  // Column 2
  [
    {
      id: "timeMult2",
      description:
        "Antimatter Dimensions gain a multiplier based on time spent in current Infinity",
      effect: { kind: "mult", places: 2, under: 2 },
    },
    {
      id: "unspentBonus",
      description: "Multiplier to 1st Antimatter Dimension based on unspent Infinity Points",
      effect: { kind: "mult", places: 2, under: 2 },
    },
    {
      id: "resetMult",
      description: "Increase Dimension Boost multiplier",
      effect: { kind: "text", text: "×2 ➜ ×2.5" },
    },
    {
      id: "passiveGen",
      description:
        "Passively generate Infinity Points 10 times slower than your fastest Infinity",
    },
  ],
  // Column 3
  [
    {
      id: "skipReset1",
      description:
        "Start every reset with 1 Dimension Boost, automatically unlocking the 5th Antimatter Dimension",
    },
    {
      id: "skipReset2",
      description:
        "Start every reset with 2 Dimension Boosts, automatically unlocking the 6th Antimatter Dimension",
    },
    {
      id: "skipReset3",
      description:
        "Start every reset with 3 Dimension Boosts, automatically unlocking the 7th Antimatter Dimension",
    },
    {
      id: "skipResetGalaxy",
      description:
        "Start every reset with 4 Dimension Boosts, automatically unlocking the 8th Antimatter Dimension; and an Antimatter Galaxy",
    },
  ],
];
