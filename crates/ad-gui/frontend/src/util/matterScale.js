// Antimatter volume comparisons for the Statistics tab — a port of the
// original `components/tabs/statistics/matter-scale.js`. Display-only, so it
// lives frontend-side; it works in log10 space on the snapshot's raw
// `{ m, e }` numbers because antimatter far exceeds the f64 range.
import { formatDecimal, formatTime } from "./format";
import { numFromLog10, numLog10 as log10 } from "./num";

// The original tables, converted to log10. `2.82e-45` m³ per proton,
// `4.22419e-105` m³ per planck volume.
const PROTON_LOG10 = Math.log10(2.82) - 45;
const PLANCK_LOG10 = Math.log10(4.22419) - 105;

const MICRO_OBJECTS = [
  { log10: -54, name: "attometers cubed" },
  { log10: -63, name: "zeptometers cubed" },
  { log10: -72, name: "yoctometers cubed" },
  { log10: PLANCK_LOG10, name: "planck volumes" },
];

const MACRO_OBJECTS = [
  { log10: PROTON_LOG10, name: "protons", verb: "make" },
  { log10: -42, name: "nuclei", verb: "make" },
  { log10: Math.log10(7.23) - 30, name: "Hydrogen atoms", verb: "make" },
  { log10: Math.log10(5) - 21, name: "viruses", verb: "make" },
  { log10: Math.log10(9) - 17, name: "red blood cells", verb: "make" },
  { log10: Math.log10(6.2) - 11, name: "grains of sand", verb: "make" },
  { log10: Math.log10(5) - 8, name: "grains of rice", verb: "make" },
  { log10: Math.log10(3.555) - 6, name: "teaspoons", verb: "fill" },
  { log10: Math.log10(7.5) - 4, name: "wine bottles", verb: "fill" },
  { log10: 0, name: "fridge-freezers", verb: "fill" },
  { log10: Math.log10(2.5) + 3, name: "Olympic-sized swimming pools", verb: "fill" },
  { log10: Math.log10(2.6006) + 6, name: "Great Pyramids of Giza", verb: "make" },
  { log10: Math.log10(3.3) + 8, name: "Great Walls of China", verb: "make" },
  { log10: Math.log10(5) + 12, name: "large asteroids", verb: "make" },
  { log10: Math.log10(4.5) + 17, name: "dwarf planets", verb: "make" },
  { log10: Math.log10(1.08) + 21, name: "Earths", verb: "make" },
  { log10: Math.log10(1.53) + 24, name: "Jupiters", verb: "make" },
  { log10: Math.log10(1.41) + 27, name: "Suns", verb: "make" },
  { log10: Math.log10(5) + 32, name: "red giants", verb: "make" },
  { log10: Math.log10(8) + 36, name: "hypergiant stars", verb: "make" },
  { log10: Math.log10(1.7) + 45, name: "nebulas", verb: "make" },
  { log10: Math.log10(1.7) + 48, name: "Oort clouds", verb: "make" },
  { log10: Math.log10(3.3) + 55, name: "Local Bubbles", verb: "make" },
  { log10: Math.log10(3.3) + 61, name: "galaxies", verb: "make" },
  { log10: Math.log10(5) + 68, name: "Local Groups", verb: "make" },
  { log10: 73, name: "Sculptor Voids", verb: "make" },
  { log10: Math.log10(3.4) + 80, name: "observable universes", verb: "make" },
  { log10: 113, name: "Dimensions", verb: "make" },
  // DC.C2P1024 = 2^1024 (the Number.MAX_VALUE doubling point).
  { log10: 1024 * Math.log10(2), name: "Infinity Dimensions", verb: "make" },
  { log10: 65000, name: "Time Dimensions", verb: "make" },
];

// The largest macro object not exceeding the plancked amount (the original's
// binary search over `macroObjects`).
function macroScale(planckedLog10) {
  const last = MACRO_OBJECTS[MACRO_OBJECTS.length - 1];
  if (planckedLog10 >= last.log10) return last;
  let low = 0;
  let high = MACRO_OBJECTS.length;
  while (low !== high) {
    const mid = Math.floor((low + high) / 2);
    if (MACRO_OBJECTS[mid].log10 <= planckedLog10) {
      low = mid + 1;
    } else {
      high = mid;
    }
  }
  return MACRO_OBJECTS[high - 1];
}

// The first micro object small enough that `matter × amount < proton`.
function microScale(matterLog10) {
  for (const scale of MICRO_OBJECTS) {
    if (matterLog10 + scale.log10 < PROTON_LOG10) return scale;
  }
  throw new Error("Cannot determine smallest antimatter scale");
}

// The 1–3 line "how much antimatter is that" comparison (the original
// `MatterScale.estimate`). `antimatter` is a snapshot { m, e } number.
export function estimateMatterScale(antimatter) {
  if (!antimatter || antimatter.m === 0) return ["There is no antimatter yet."];
  const matterLog10 = log10(antimatter);
  if (matterLog10 > 100000) {
    return [
      "If you wrote 3 numbers a second, it would take you",
      formatTime((matterLog10 / 3) * 1000),
      "to write down your antimatter amount.",
    ];
  }
  const planckedLog10 = matterLog10 + PLANCK_LOG10;
  if (planckedLog10 > PROTON_LOG10) {
    const scale = macroScale(planckedLog10);
    const amount = formatDecimal(numFromLog10(planckedLog10 - scale.log10), 2, 1);
    return [
      `If every antimatter were a planck volume, you would have enough to ` +
        `${scale.verb} ${amount} ${scale.name}`,
    ];
  }
  const scale = microScale(matterLog10);
  const per = formatDecimal(
    numFromLog10(PROTON_LOG10 - scale.log10 - matterLog10),
    2,
    1,
  );
  return [
    `If every antimatter were ${per} ${scale.name}, ` +
      `you would have enough to make a proton.`,
  ];
}
