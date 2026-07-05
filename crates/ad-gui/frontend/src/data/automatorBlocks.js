// The block editor's palette + block→text conversion, vendored from
// AutomatorBlocks.vue (`automatorBlocks`) and AutomatorBlockEditor.vue
// (`BlockAutomator.generateText/parseLines`). The engine's blockifier ships
// per-block values; these tables supply the input structure (dropdown
// classes, patterns, targets).

const COMPARISON_OPERATORS = ["<", ">", ">=", "<="];
const COMPARISON_CURRENCIES = [
  "AM", "IP", "EP", "RM", "INFINITIES", "BANKED INFINITIES", "ETERNITIES", "REALITIES",
  "PENDING IP", "PENDING EP", "PENDING TP", "PENDING RM", "PENDING GLYPH LEVEL",
  "DT", "TP", "RG", "REP", "TT", "TOTAL TT", "SPENT TT", "TOTAL COMPLETIONS", "PENDING COMPLETIONS",
  "EC1 COMPLETIONS", "EC2 COMPLETIONS", "EC3 COMPLETIONS", "EC4 COMPLETIONS",
  "EC5 COMPLETIONS", "EC6 COMPLETIONS", "EC7 COMPLETIONS", "EC8 COMPLETIONS",
  "EC9 COMPLETIONS", "EC10 COMPLETIONS", "EC11 COMPLETIONS", "EC12 COMPLETIONS",
];
const RESETS = ["INFINITY", "ETERNITY", "REALITY"];

// `unlock` gates palette entries on snapshot flags: "reality25" = RU25
// bought, "blackHole" = BH1 unlocked, "enslaved" = celestial (never at our
// frontier).
export const AUTOMATOR_BLOCKS = [
  { cmd: "STUDIES RESPEC", alias: "RESPEC TIME STUDIES" },
  {
    cmd: "STUDIES LOAD",
    alias: "LOAD STUDY PRESET",
    allowedPatterns: ["AB"],
    A: ["ID", "NAME"],
    B: ["*"],
    targets: ["singleSelectionInput", "singleTextInput"],
    canWait: true,
  },
  {
    cmd: "STUDIES PURCHASE",
    alias: "PURCHASE STUDIES",
    allowedPatterns: ["A"],
    A: ["*"],
    targets: ["singleTextInput"],
    canWait: true,
  },
  { cmd: "INFINITY", canWait: true },
  { cmd: "ETERNITY", canRespec: true, canWait: true },
  { cmd: "REALITY", canRespec: true, canWait: true, unlock: "reality25" },
  {
    cmd: "UNLOCK",
    allowedPatterns: ["AB", "C"],
    A: ["EC"],
    B: ["*"],
    C: ["DILATION"],
    targets: ["singleSelectionInput", "singleTextInput"],
    canWait: true,
  },
  {
    cmd: "START",
    allowedPatterns: ["AB", "C"],
    A: ["EC"],
    B: ["*"],
    C: ["DILATION"],
    targets: ["singleSelectionInput", "singleTextInput"],
  },
  {
    cmd: "AUTO",
    alias: "CHANGE AUTOBUYER SETTING",
    allowedPatterns: ["AB"],
    A: RESETS,
    B: ["ON", "OFF", "* AUTOBUYER SETTING"],
    targets: ["singleSelectionInput", "singleTextInput"],
  },
  {
    cmd: "BLACK HOLE",
    alias: "TURN BLACK HOLE",
    allowedPatterns: ["A"],
    A: ["ON", "OFF"],
    targets: ["singleSelectionInput"],
    unlock: "blackHole",
  },
  {
    cmd: "STORE GAME TIME",
    alias: "SET GAME TIME STORAGE TO",
    allowedPatterns: ["A"],
    A: ["ON", "OFF", "USE"],
    targets: ["singleSelectionInput"],
    unlock: "enslaved",
  },
  {
    cmd: "NOTIFY",
    alias: "GAME NOTIFICATION:",
    allowedPatterns: ["A"],
    A: ["*"],
    targets: ["singleTextInput"],
  },
  {
    cmd: "COMMENT",
    alias: "NOTE:",
    allowedPatterns: ["A"],
    A: ["*"],
    targets: ["singleTextInput"],
  },
  {
    cmd: "WAIT",
    alias: "PAUSE AUTOMATOR UNTIL",
    allowedPatterns: ["A", "DE", "BCB"],
    A: RESETS,
    B: [...COMPARISON_CURRENCIES, "* SPECIFIED CONSTANT"],
    C: COMPARISON_OPERATORS,
    D: ["BLACK HOLE"],
    E: ["OFF", "BH1", "BH2"],
    targets: ["genericInput1", "compOperator", "genericInput2"],
  },
  {
    cmd: "PAUSE",
    alias: "PAUSE AUTOMATOR FOR",
    allowedPatterns: ["A"],
    A: ["*"],
    targets: ["singleTextInput"],
  },
  {
    cmd: "IF",
    alias: "ENTER BLOCK IF",
    allowedPatterns: ["ABA"],
    A: [...COMPARISON_CURRENCIES, "* SPECIFIED CONSTANT"],
    B: COMPARISON_OPERATORS,
    targets: ["genericInput1", "compOperator", "genericInput2"],
    nested: true,
  },
  {
    cmd: "UNTIL",
    alias: "REPEAT BLOCK UNTIL",
    allowedPatterns: ["A", "BCB"],
    A: RESETS,
    B: [...COMPARISON_CURRENCIES, "* SPECIFIED CONSTANT"],
    C: COMPARISON_OPERATORS,
    targets: ["genericInput1", "compOperator", "genericInput2"],
    nested: true,
  },
  {
    cmd: "WHILE",
    alias: "REPEAT BLOCK WHILE",
    allowedPatterns: ["ABA"],
    A: [...COMPARISON_CURRENCIES, "* SPECIFIED CONSTANT"],
    B: COMPARISON_OPERATORS,
    targets: ["genericInput1", "compOperator", "genericInput2"],
    nested: true,
  },
  { cmd: "BLOB" },
  { cmd: "STOP", alias: "STOP EXECUTION" },
];

export const AUTOMATOR_BLOCKS_MAP = Object.fromEntries(
  AUTOMATOR_BLOCKS.map((b) => [b.cmd, b]),
);

// Palette entries offered for dragging (BLOB is blacklisted, as in the
// original).
export const PALETTE_BLOCKS = AUTOMATOR_BLOCKS.filter((b) => b.cmd !== "BLOB");

let nextBlockId = 1;
export function newBlockId() {
  return nextBlockId++;
}

// Merge an engine blockify value-block with its palette config + a UI id.
export function hydrateBlock(raw) {
  const config = AUTOMATOR_BLOCKS_MAP[raw.cmd] ?? { cmd: raw.cmd };
  const block = { ...config, ...raw, id: newBlockId() };
  if (block.nested) {
    block.nest = (raw.nest ?? []).map(hydrateBlock);
  }
  return block;
}

// `BlockAutomator.generateText`: one block → one script line.
export function generateText(block, indentation = 0) {
  let parsed = `${"\t".repeat(indentation)}${block.cmd} `;
  parsed = parsed.replace("COMMENT", "//").replace("BLOB", "blob  ");
  if (block.canWait && block.nowait) {
    parsed = parsed.replace(/(\S+)/u, "$1 NOWAIT");
  }
  if (block.respec) parsed += ` RESPEC`;

  const props = ["genericInput1", "compOperator", "genericInput2", "singleSelectionInput", "singleTextInput"];
  for (const prop of props) {
    if (block[prop]) parsed += ` ${block[prop]}`;
  }
  if (block.cmd === "IF" || block.cmd === "WHILE" || block.cmd === "UNTIL") {
    parsed += " {";
  }
  return parsed.replace("  ", " ");
}

// `BlockAutomator.parseLines`: blocks → script lines (recursing into nests).
export function parseLines(blocks, indentation = 0) {
  const lines = [];
  for (const block of blocks) {
    // A mid-drag template placeholder has no cmd (it gets unpacked into real
    // blocks by the drag's end handler).
    if (!block.cmd) continue;
    lines.push(generateText(block, indentation));
    if (block.cmd === "IF" || block.cmd === "WHILE" || block.cmd === "UNTIL") {
      lines.push(...parseLines(block.nest ?? [], indentation + 1));
      lines.push(`${"\t".repeat(indentation)}}`);
    }
  }
  return lines;
}

// Text line count of a block (nested headers span their nest + the `}`).
export function numberOfLinesInBlock(block) {
  return block.nested
    ? Math.max((block.nest ?? []).reduce((v, b) => v + numberOfLinesInBlock(b), 1), 2)
    : 1;
}
