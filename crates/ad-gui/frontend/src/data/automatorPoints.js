// Automator Point source display text, vendored from the original's
// secret-formula (perks.js / reality-upgrades.js `shortDescription` +
// automator.js `otherAutomatorPoints`). The engine ships ids + AP + bought;
// these tables supply the copy for the locked-tab AP page.

export const PERK_AP_DESCRIPTIONS = {
  14: "Start with 10 EP",
  16: "Start with 5e9 EP",
  17: "Start with 10 TP",
  44: "Auto-purchase TT generation",
  45: "Auto-unlock TD 5-8",
  46: "Auto-unlock Reality",
  53: "Unlocking Dilation only requires TT",
  60: "Auto-complete ECs every 60 minutes",
  62: "Auto-complete ECs every 20 minutes",
  72: "Remove EC secondary requirements",
  73: "Bulk EC Completion",
  83: "×3 TP upgrade applies retroactively",
  100: "Dilation Upgrade Autobuyers",
  101: "Faster ID Autobuyers",
  102: "Faster Replicanti Autobuyers",
  103: "Faster Dilation Autobuyers",
  104: "Single TT Autobuyer",
  106: "Max TT Autobuyer",
  107: "Dilation Autobuyer bulk",
  201: "Faster Achievements: every 20 minutes",
  205: "Keep Achievements on Reality",
};

export const UPGRADE_AP_DESCRIPTIONS = {
  10: "Start with 100 Eternities",
  11: "Continuous Infinity generation",
  13: "TD and ×5 EP Autobuyers, improved Eternity Autobuyer",
  14: "Continuous Eternity generation",
  20: "Second Black Hole",
  25: "Reality Autobuyer",
};

// `otherAutomatorPoints`: descriptions + the big background symbols.
export const OTHER_AP_SOURCES = {
  "Reality Count": {
    description: "+2 per Reality, up to 50 Realities",
    symbol: "Ϟ",
    isIcon: false,
  },
  "Black Hole": {
    description: "Unlocking gives 10 AP",
    symbol: "fas fa-circle",
    isIcon: true,
  },
};
