// Generates ground-truth format() cases from the real
// @antimatter-dimensions/notations package and emits a Rust integration test.
//
// The original game repo (`antimatter-dimensions`) must be checked out as a sibling
// of this repo's root — i.e. `code/antimatter-dimensions` next to
// `code/antimatter-dimensions-rust` — with its npm deps installed (`npm install`),
// so the notations package and its bundled break_infinity resolve. The path below is
// relative to this script, so it can be run from any cwd:
//   node crates/ad-format/tests/gen_format_cases.mjs > crates/ad-format/tests/format_edge_cases.rs
import { createRequire } from "node:module";
// `../../../../` walks tests/ -> ad-format/ -> crates/ -> repo root -> code/.
const require = createRequire(
  new URL(
    "../../../../antimatter-dimensions/node_modules/@antimatter-dimensions/notations/x.js",
    import.meta.url
  )
);
const N = require("./dist/ad-notations.umd.js");
const Decimal = require("break_infinity.js");

const NOTATIONS = {
  Scientific: new N.ScientificNotation(),
  Engineering: new N.EngineeringNotation(),
  Standard: new N.StandardNotation(),
  Letters: new N.LettersNotation(),
};

// One ground-truth evaluation. opts mirror Rust FormatOptions.
function evalCase(c) {
  const o = {
    places: 2,
    pu: 0,
    pe: 3,
    show: true,
    min: 100000,
    max: 1000000000,
    inf: null,
    ...c.opts,
  };
  N.Settings.exponentCommas = { show: o.show, min: o.min, max: o.max };
  N.Settings.isInfinite = o.inf
    ? (d) => d.gte(new Decimal(o.inf))
    : () => false;
  const out = NOTATIONS[c.notation].format(new Decimal(c.value), o.places, o.pu, o.pe);
  return { ...c, opts: o, expected: out };
}

// ---- Case groups -------------------------------------------------------------
const groups = [];
const G = (name, doc, cases, ignore = null) =>
  groups.push({ name, doc, cases, ignore });

G("infinite_threshold", "inf-threshold boundary, sign, and non-Scientific notations", [
  { notation: "Scientific", value: "1e308", opts: { inf: "1e308" } }, // == is inclusive
  { notation: "Scientific", value: "9.99e307", opts: { inf: "1e308" } }, // just below
  { notation: "Scientific", value: "-1e308", opts: { inf: "1e308" } }, // negative ==
  { notation: "Scientific", value: "1e309", opts: { inf: null } }, // None never Infinite
  { notation: "Standard", value: "1e308", opts: { inf: "1e308" } },
  { notation: "Letters", value: "-1e308", opts: { inf: "1e308" } },
]);

G("region_boundaries", "exponent -300/-301 and 2/3 region splits, very-small sign", [
  { notation: "Scientific", value: "1e-300", opts: { pu: 0 } }, // exp -300 -> under-1000
  { notation: "Scientific", value: "1e-300", opts: { pu: 2 } },
  { notation: "Scientific", value: "1e-301", opts: { pu: 0 } }, // exp -301 -> very-small
  { notation: "Scientific", value: "1e-301", opts: { pu: 2 } },
  { notation: "Scientific", value: "-1e-310", opts: { pu: 0 } }, // very-small negative
  { notation: "Scientific", value: "-1e-310", opts: { pu: 2 } },
  { notation: "Scientific", value: "9.99e2", opts: { pu: 0 } }, // 999 -> under-1000
  { notation: "Scientific", value: "1e3", opts: {} }, // 1000 -> big number
]);

G("small_all_notations", "under-1000 + negatives for Standard and Letters", [
  { notation: "Standard", value: "4.2e1", opts: {} },
  { notation: "Standard", value: "-4.2e1", opts: {} },
  { notation: "Letters", value: "4.2e1", opts: {} },
  { notation: "Letters", value: "-4.2e1", opts: {} },
]);

G("negative_big", "sign path for non-Scientific big numbers", [
  { notation: "Standard", value: "-1e6", opts: {} },
  { notation: "Letters", value: "-1.5e10", opts: {} },
  { notation: "Engineering", value: "-1.23456e100", opts: {} },
]);

G("exponent_display", "show=false, exact min/max boundaries, custom min/max", [
  { notation: "Scientific", value: "1e100000", opts: { show: false } }, // [min,max) -> recursive
  { notation: "Scientific", value: "1e99999", opts: { show: false } }, // < min -> plain
  { notation: "Scientific", value: "1e100000", opts: {} }, // == min -> commas
  { notation: "Scientific", value: "1e99999", opts: {} }, // min-1 -> plain
  { notation: "Scientific", value: "1e1000000000", opts: {} }, // == max -> recursive
  { notation: "Scientific", value: "1e999999999", opts: {} }, // max-1 -> commas
  { notation: "Scientific", value: "1e5000", opts: { min: 10, max: 100000 } }, // custom -> commas
  { notation: "Scientific", value: "1e5", opts: { min: 10, max: 100000 } }, // custom < min -> plain
  { notation: "Scientific", value: "1e100000", opts: { min: 10, max: 100000 } }, // custom == max -> recursive
]);

G("places_exponent", "nested mantissa places in the recursive branch (max(2))", [
  { notation: "Scientific", value: "1e1000000000", opts: { pe: 0 } }, // -> 2 places
  { notation: "Scientific", value: "1e1000000000", opts: { pe: 4 } },
  { notation: "Scientific", value: "1e1000000000000000", opts: {} }, // recursion depth 1 (1e15)
]);

G("standard_extra", "roll-over into a new letter, places vs places_under_1000, regex paths", [
  { notation: "Standard", value: "9.99999e5", opts: {} }, // mantissa rounds 1000 -> roll to M
  { notation: "Standard", value: "1.5e6", opts: { places: 3 } },
  { notation: "Standard", value: "1.5e2", opts: { places: 3, pu: 1 } }, // sub-1000 uses pu
  { notation: "Standard", value: "1e303", opts: {} },
  { notation: "Standard", value: "1e6003", opts: {} },
  { notation: "Standard", value: "1e100002", opts: {} },
]);

G("letters_extra", "3-letter transcription, residues, exponent-display ignored", [
  { notation: "Letters", value: "1e2109", opts: {} }, // aaa
  { notation: "Letters", value: "1e2106", opts: {} }, // zz (remainder==0 carry)
  { notation: "Letters", value: "1e9", opts: {} }, // residue 0
  { notation: "Letters", value: "1e10", opts: {} }, // residue 1
  { notation: "Letters", value: "1e11", opts: {} }, // residue 2
  { notation: "Letters", value: "1e1000002", opts: {} }, // huge exponent, no commas/recursion
  { notation: "Letters", value: "1e2109", opts: { pe: 0, show: false } }, // exponent-display ignored
]);

G("places_sweep", "mantissa places 0/1/2/5", [
  { notation: "Scientific", value: "1.23456e100", opts: { places: 0 } },
  { notation: "Scientific", value: "1.23456e100", opts: { places: 1 } },
  { notation: "Scientific", value: "1.23456e100", opts: { places: 2 } },
  { notation: "Scientific", value: "1.23456e100", opts: { places: 5 } },
]);

// NOTE: exact half-way mantissas (2.5, 1.25, …) are deliberately NOT generated
// here — Rust rounds half-to-even while JS `toFixed` rounds half-away, an accepted
// presentation-layer divergence. That behaviour is pinned directly in
// `tests/rounding_divergence.rs`, which asserts the Rust values (not JS ground
// truth), so this generated file stays 100% in agreement with the JS reference.

// ---- Emit --------------------------------------------------------------------
const results = groups.map((g) => ({ ...g, cases: g.cases.map(evalCase) }));

// Dump JSON to stderr for inspection; Rust to stdout.
console.error(JSON.stringify(results, null, 1));

const esc = (s) => s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
const optExpr = (o) => {
  const ed =
    o.show === true && o.min === 100000 && o.max === 1000000000
      ? "DEF_ED"
      : `ExponentDisplay { show: ${o.show}, min: ${o.min}, max: ${o.max} }`;
  const inf = o.inf ? `Some("${o.inf}")` : "None";
  return `o(Notation::${"$N"}, ${o.places}, ${o.pu}, ${o.pe}, ${ed}, ${inf})`;
};

let rs = `//! Black-box fidelity tests: \`format(value, opts)\` -> string, checked against
//! ground truth from the real \`@antimatter-dimensions/notations\` v3.1.0 package.
//!
//! Generated by \`gen_format_cases.mjs\` (alongside this file); see the design doc
//! \`design-docs/2026-06-28-ad-format-test-plan.md\`. Regenerate with:
//!     node crates/ad-format/tests/gen_format_cases.mjs > crates/ad-format/tests/format_edge_cases.rs
//!     cargo fmt -p ad-format
//!
//! Values are parsed identically on both sides (both are break_infinity ports), so
//! the only variable under test is the formatting logic. Do not hand-edit expected
//! strings; regenerate instead.

use ad_format::{format, ExponentDisplay, FormatOptions, Notation};
use break_infinity::Decimal;

fn d(s: &str) -> Decimal {
    s.parse().expect("valid Decimal literal")
}

const DEF_ED: ExponentDisplay = ExponentDisplay {
    show: true,
    min: 100_000,
    max: 1_000_000_000,
};

/// Compact FormatOptions builder mirroring the JS call \`format(v, places, pu, pe)\`
/// plus the global \`Settings\` (exponent display, infinite threshold).
fn o(
    notation: Notation,
    places: u32,
    pu: u32,
    pe: u32,
    exponent_display: ExponentDisplay,
    inf: Option<&str>,
) -> FormatOptions {
    FormatOptions {
        notation,
        places,
        places_under_1000: pu,
        places_exponent: pe,
        exponent_display,
        inf_threshold: inf.map(d),
    }
}
`;

for (const g of results) {
  rs += `\n#[test]\n`;
  if (g.ignore) rs += `#[ignore = "${esc(g.ignore)}"]\n`;
  rs += `fn ${g.name}() {\n    // ${g.doc}\n`;
  for (const c of g.cases) {
    const opt = optExpr(c.opts).replace("$N", c.notation);
    rs += `    assert_eq!(format(&d("${esc(c.value)}"), &${opt}), "${esc(c.expected)}");\n`;
  }
  rs += `}\n`;
}

process.stdout.write(rs);
