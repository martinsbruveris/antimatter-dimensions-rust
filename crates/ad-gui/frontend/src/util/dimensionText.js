export const DIM_NAMES = ["1st", "2nd", "3rd", "4th", "5th", "6th", "7th", "8th"];

// Mirror of the JS DimBoost.unlockedByBoost for a fresh pre-infinity
// run (no achievements/milestones that alter the text).
export function dimBoostText(boosts, power) {
  const maxUnlockable = 8;
  let newUnlock = "";
  if (boosts < maxUnlockable - 4) {
    newUnlock = `unlock the ${boosts + 5}th Dimension`;
  } else if (boosts === 4) {
    newUnlock = "unlock Sacrifice";
  }
  const multText = `give a ×${power.toFixed(1)} multiplier `;
  let range = "to the 1st Dimension";
  if (boosts > 0) range = `to Dimensions 1-${Math.min(boosts + 1, 8)}`;
  if (boosts >= maxUnlockable - 1) range = "to all Dimensions";
  const effects = newUnlock === ""
    ? `${multText} ${range}`
    : `${newUnlock} and ${multText} ${range}`;
  return `Reset your Dimensions to ${effects}`;
}

export const GALAXY_BUTTON_TEXT =
  "Reset your Dimensions and Dimension Boosts to increase the power of Tickspeed upgrades";
