// Eternity Milestone reward text, keyed by the engine's milestone id
// (the original `GameDatabase.eternity.milestones` keys). Display strings live
// frontend-side by design — the engine owns only thresholds + reached state.
// The offline-generation milestones (autoEP / autoEternities / autoInfinities)
// show their base description; their live rate readouts arrive with offline
// progress.
export const ETERNITY_MILESTONE_REWARDS = {
  autobuyerIPMult: "Unlock the Infinity Point multiplier autobuyer",
  keepAutobuyers:
    "You start Eternity with all Normal Challenges complete, all normal autobuyers, and infinity broken",
  autobuyerReplicantiGalaxy: "Unlock the Replicanti Galaxy Autobuyer",
  keepInfinityUpgrades: "You start Eternity with all Infinity Upgrades",
  bigCrunchModes: "Unlock more Big Crunch Autobuyer options",
  autoEP:
    "While offline, gain 25% of your best Eternity Points per minute from previous Eternities",
  autoIC:
    "You complete Infinity Challenges as soon as you unlock them, and keep the Dimensional Sacrifice Autobuyer",
  keepBreakUpgrades: "You start Eternity with all Break Infinity Upgrades",
  autobuyMaxGalaxies: "Unlock the buy max Antimatter Galaxies Autobuyer mode",
  unlockReplicanti: "You start with Replicanti unlocked",
  autobuyerID1: "Unlock the 1st Infinity Dimension Autobuyer",
  autobuyerID2: "Unlock the 2nd Infinity Dimension Autobuyer",
  autobuyerID3: "Unlock the 3rd Infinity Dimension Autobuyer",
  autobuyerID4: "Unlock the 4th Infinity Dimension Autobuyer",
  autobuyerID5: "Unlock the 5th Infinity Dimension Autobuyer",
  autobuyerID6: "Unlock the 6th Infinity Dimension Autobuyer",
  autobuyerID7: "Unlock the 7th Infinity Dimension Autobuyer",
  autobuyerID8: "Unlock the 8th Infinity Dimension Autobuyer",
  autoUnlockID: "You automatically unlock Infinity Dimensions upon reaching them",
  unlockAllND: "Start with all Antimatter Dimensions available for purchase",
  replicantiNoReset:
    "Replicanti Galaxies no longer reset Antimatter, Antimatter Dimensions, Tickspeed, Dimensional Sacrifice, or Dimension Boosts",
  autobuyerReplicantiChance: "Unlock the Replicanti Chance Upgrade Autobuyer",
  autobuyerReplicantiInterval: "Unlock the Replicanti Interval Upgrade Autobuyer",
  autobuyerReplicantiMaxGalaxies:
    "Unlock the Max Replicanti Galaxy Upgrade Autobuyer",
  autobuyerEternity: "Unlock autobuyer for Eternities",
  autoEternities:
    "While offline, gain Eternities at 50% the rate of your fastest Eternity",
  autoInfinities:
    "While offline, gain Infinities equal to 50% your best Infinities/hour this Eternity",
};
