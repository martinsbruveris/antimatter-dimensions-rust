// Script-template UI metadata, vendored from the original
// ../antimatter-dimensions/src/core/secret-formula/script-templates.js.
// Script generation and the game-state warnings live in the engine
// (automator/templates.rs); this file only drives the input prompts.

// Input-type behaviors (`paramTypes`). `boolDisplay` marks a two-state
// button input ([true-text, false-text]); `isValidString` validates text
// inputs client-side; `map` converts the input to the engine's param value.
// The "tree" type is validated in the modal instead: a preset reference is
// checked against the snapshot presets and everything else goes through the
// engine's import-string validator (an async command).
export const TEMPLATE_PARAM_TYPES = [
  { name: "tree" },
  {
    name: "integer",
    // `AutobuyerInputFunctions.int.tryParse`: explicit format check so junk
    // like "361ebqv3" doesn't parse as 361.
    isValidString: (str) => /^\d+$/u.test(str.replaceAll(",", "")),
  },
  {
    name: "decimal",
    // `AutobuyerInputFunctions.decimal.tryParse` accepts logarithm ("e30"),
    // scientific ("2.3e41"), and mixed-scientific ("2.33e41.2") notation.
    isValidString: (str) => {
      if (!str) return false;
      const s = str.replaceAll(",", "");
      return (
        /^e\d*[.]?\d+$/u.test(s) ||
        /^\d*[.]?\d+(e\d+)?$/u.test(s) ||
        /^\d*[.]?\d+(e\d*[.]?\d+)?$/u.test(s)
      );
    },
  },
  { name: "boolean", boolDisplay: [true, false] },
  { name: "nowait", boolDisplay: ["Continue onward", "Keep buying Studies"] },
  {
    name: "mode",
    boolDisplay: ["X times highest", "Seconds since last"],
    map: (x) => (x ? "mult" : "time"),
  },
];

export const AUTOMATOR_TEMPLATES = [
  {
    name: "Climb EP",
    description: `This script performs repeated Eternities, attempting to re-purchase a Time Study Tree every
      Eternity. Autobuyer settings must be supplied for the Infinity and Eternity Autobuyers. The script will
      repeat until a final Eternity Point value is reached.`,
    inputs: [
      { name: "treeStudies", type: "tree", prompt: "Or directly enter your time studies" },
      { name: "treeNowait", type: "nowait", prompt: "Missing Study behavior" },
      { name: "finalEP", type: "decimal", prompt: "Target EP" },
      { name: "autoInfMode", type: "mode", prompt: "Infinity Autobuyer Mode" },
      { name: "autoInfValue", type: "decimal", prompt: "Infinity Autobuyer Threshold" },
      { name: "autoEterMode", type: "mode", prompt: "Eternity Autobuyer Mode" },
      { name: "autoEterValue", type: "decimal", prompt: "Eternity Autobuyer Threshold" },
    ],
  },
  {
    name: "Grind Eternities",
    description: `This script performs repeated fast Eternities after buying a specified Time Study Tree.
      Auto-Infinity will be set to "Times Highest" with a specified number of crunches and Auto-Eternity will
      trigger as soon as possible. The script will repeat until a final Eternity count is reached.`,
    inputs: [
      { name: "treeStudies", type: "tree", prompt: "Or directly enter your time studies" },
      { name: "treeNowait", type: "nowait", prompt: "Missing Study behavior" },
      { name: "crunchesPerEternity", type: "integer", prompt: "Crunches per Eternity" },
      { name: "eternities", type: "decimal", prompt: "Target Eternity Count" },
    ],
  },
  {
    name: "Grind Infinities",
    description: `This script buys a specified Time Study Tree and then configures your Autobuyers for gaining
      Infinities. It will repeat until a final Infinity count is reached; the count can be for Banked Infinities,
      in which case it will get all Infinities before performing a single Eternity.`,
    inputs: [
      { name: "treeStudies", type: "tree", prompt: "Or directly enter your time studies" },
      { name: "treeNowait", type: "nowait", prompt: "Missing Study behavior" },
      { name: "infinities", type: "decimal", prompt: "Target Infinity Count" },
      { name: "isBanked", type: "boolean", prompt: "Use Banked for Target?" },
    ],
  },
  {
    name: "Complete Eternity Challenge",
    description: `This script buys a specified Time Study Tree and then unlocks a specified Eternity Challenge.
      Then it will set your Infinity Autobuyer to your specified settings and enter the Eternity Challenge.
      Finally, it will wait until at least the desired number of completions before triggering an Eternity to
      complete the Challenge.`,
    inputs: [
      { name: "treeStudies", type: "tree", prompt: "Or directly enter your time studies" },
      { name: "treeNowait", type: "nowait", prompt: "Missing Study behavior" },
      { name: "ec", type: "integer", prompt: "Eternity Challenge ID" },
      { name: "completions", type: "integer", prompt: "Target Completion Count" },
      { name: "autoInfMode", type: "mode", prompt: "Infinity Autobuyer Mode" },
      { name: "autoInfValue", type: "decimal", prompt: "Infinity Autobuyer Threshold" },
    ],
  },
  {
    name: "Unlock Dilation",
    description: `This script performs repeated Eternities, attempting to re-purchase a Time Study Tree every
      Eternity. Settings must be supplied for the Eternity Autobuyer; your Infinity Autobuyer will be
      turned off. The script loops until you have the total Time Theorem requirement to unlock Dilation, and then
      it will unlock Dilation once it does.`,
    inputs: [
      { name: "treeStudies", type: "tree", prompt: "Or directly enter your time studies" },
      { name: "treeNowait", type: "nowait", prompt: "Missing Study behavior" },
      { name: "finalEP", type: "decimal", prompt: "Target EP" },
      { name: "autoEterMode", type: "mode", prompt: "Eternity Autobuyer Mode" },
      { name: "autoEterValue", type: "decimal", prompt: "Eternity Autobuyer Threshold" },
    ],
  },
];

export function templateParamType(name) {
  return TEMPLATE_PARAM_TYPES.find((p) => p.name === name);
}
