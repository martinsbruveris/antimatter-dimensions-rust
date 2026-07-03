// Static display data for the 8 Infinity Challenges (Feature 2.7). Live
// unlocked/running/completed state comes from the engine snapshot
// (`game.infinity_challenges`); the restriction/reward/goal text lives here,
// keyed by id.

export const INFINITY_CHALLENGES = [
  {
    id: 1,
    goal: "1e650",
    unlockAM: "1e2000",
    description:
      "all Normal Challenge restrictions run at once, except the Tickspeed and Big Crunch challenges.",
    reward: "×1.3 to all Infinity Dimensions per Infinity Challenge completed.",
  },
  {
    id: 2,
    goal: "1e10500",
    unlockAM: "1e11000",
    description:
      "Dimensional Sacrifice happens automatically every 400 ms once you have an 8th Antimatter Dimension.",
    reward: "Dimensional Sacrifice autobuyer, and a much stronger Sacrifice.",
  },
  {
    id: 3,
    goal: "1e5000",
    unlockAM: "1e12000",
    description:
      "Tickspeed upgrades are always ×1; instead each purchase grants a static Antimatter Dimension multiplier that grows with Antimatter Galaxies.",
    reward: "that Antimatter Dimension multiplier, kept permanently.",
  },
  {
    id: 4,
    goal: "1e13000",
    unlockAM: "1e14000",
    description:
      "only the most recently bought Antimatter Dimension produces normally; every other dimension is raised to the 0.25th power.",
    reward: "all Antimatter Dimension multipliers are raised to the 1.05th power.",
  },
  {
    id: 5,
    goal: "1e16500",
    unlockAM: "1e18000",
    description:
      "buying Antimatter Dimensions 1–4 raises the cost of all cheaper dimensions; buying 5–8 raises all pricier ones.",
    reward:
      "all Galaxies are 10% stronger, and Galaxy and Dimension Boost costs drop by 1.",
  },
  {
    id: 6,
    goal: "2e22222",
    unlockAM: "1e22500",
    description:
      "exponentially rising matter divides all Antimatter Dimension multipliers once you have a 2nd Antimatter Dimension.",
    reward: "Infinity Dimension multiplier based on Tickspeed.",
  },
  {
    id: 7,
    goal: "1e10000",
    unlockAM: "1e23000",
    description:
      "you cannot buy Antimatter Galaxies, but the base Dimension Boost multiplier rises to a maximum of ×10.",
    reward: "the base Dimension Boost multiplier is at least ×4.",
  },
  {
    id: 8,
    goal: "1e27000",
    unlockAM: "1e28000",
    description:
      "Antimatter Dimension production continually drops over time; buying an Antimatter Dimension or Tickspeed upgrade resets it to 100%.",
    reward:
      "a multiplier to Antimatter Dimensions 2–7 based on your 1st and 8th dimension multipliers.",
  },
];
