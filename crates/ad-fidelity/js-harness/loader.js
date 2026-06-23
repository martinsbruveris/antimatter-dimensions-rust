// loader.js — Loads actual game source files from the original
// Antimatter Dimensions repo by stripping ES module syntax and
// evaluating the code.

const fs = require("fs");
const path = require("path");
const vm = require("vm");

const GAME_ROOT = path.resolve(
  __dirname,
  "../../../../antimatter-dimensions/src/core"
);

/**
 * Read a game source file, strip import/export statements, and
 * return the transformed source.
 */
function transformSource(filePath) {
  let source = fs.readFileSync(filePath, "utf-8");

  // Remove import statements (they reference things we've shimmed)
  source = source.replace(
    /^\s*import\s+.*?['"]\s*;?\s*$/gm,
    "// [import removed]"
  );

  // Convert `export class X` → `class X` and register globally
  source = source.replace(
    /^\s*export\s+(class\s+\w+)/gm,
    "$1"
  );

  // Convert `export function X` → `function X`
  source = source.replace(
    /^\s*export\s+(function\s+\w+)/gm,
    "$1"
  );

  // Convert `export const X` → `const X`
  source = source.replace(
    /^\s*export\s+(const\s+\w+)/gm,
    "$1"
  );

  // Convert `export let X` → `let X`
  source = source.replace(
    /^\s*export\s+(let\s+\w+)/gm,
    "$1"
  );

  // Remove bare `export { ... }` and `export *` lines
  source = source.replace(
    /^\s*export\s+\{[^}]*\}\s*;?\s*$/gm,
    "// [export removed]"
  );
  source = source.replace(
    /^\s*export\s+\*.*$/gm,
    "// [export removed]"
  );

  // Remove `export default`
  source = source.replace(
    /^\s*export\s+default\s+/gm,
    ""
  );

  // Replace window.X with global.X (Node has no window)
  source = source.replace(/window\./g, "global.");

  return source;
}

/**
 * Load a game source file into the global scope.
 * Classes and functions become global.
 */
function loadGameFile(relPath) {
  const fullPath = path.join(GAME_ROOT, relPath);
  const source = transformSource(fullPath);
  const script = new vm.Script(source, { filename: relPath });
  script.runInThisContext();
}

/**
 * Load and register the DC (Decimal Constants) object from
 * constants.js.
 */
function loadConstants() {
  const fullPath = path.join(GAME_ROOT, "constants.js");
  let source = transformSource(fullPath);

  // Evaluate and capture DC
  const script = new vm.Script(source + "\nglobal.DC = DC;\n", {
    filename: "constants.js",
  });
  script.runInThisContext();
}

/**
 * Load the ExponentialCostScaling class from math.js.
 * This is a complex file; we only extract the class we need.
 */
function loadMath() {
  const fullPath = path.join(GAME_ROOT, "math.js");
  let source = transformSource(fullPath);

  // The file uses `lngamma` from the import; stub it
  source = "const lngamma = (x) => 0;\n" + source;

  // Register key classes globally
  source += `
global.ExponentialCostScaling = ExponentialCostScaling;
`;

  const script = new vm.Script(source, { filename: "math.js" });
  script.runInThisContext();
}

/**
 * Load Decimal prototype extensions from the Effects system.
 * We skip the Effects object itself (already shimmed) and only
 * load the Decimal.prototype.* methods.
 */
function loadEffects() {
  const fullPath = path.join(
    GAME_ROOT,
    "game-mechanics/effects.js"
  );
  let source = transformSource(fullPath);

  // Remove the Effects object definition — we use the shimmed one
  source = source.replace(
    /^const\s+Effects\s*=\s*\{[\s\S]*?^};/m,
    "// [Effects defined in shims]"
  );

  const script = new vm.Script(source, { filename: "effects.js" });
  script.runInThisContext();
}

/**
 * Load a game source file and return the transformed source
 * (for cases where we need to add registration code).
 */
function loadAndRegister(relPath, registrations) {
  const fullPath = path.join(GAME_ROOT, relPath);
  let source = transformSource(fullPath);
  if (registrations) {
    source += "\n" + registrations;
  }
  const script = new vm.Script(source, { filename: relPath });
  script.runInThisContext();
}

module.exports = {
  transformSource,
  loadGameFile,
  loadConstants,
  loadMath,
  loadEffects,
  loadAndRegister,
  GAME_ROOT,
};
