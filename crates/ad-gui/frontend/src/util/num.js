// Shared helpers for the snapshot's raw number representation
// (`Num { m, e }` = mantissa × 10^exponent, mirroring the engine's Decimal).
// Presentation math only — game rules stay in the engine; these exist so
// components can compare/scale snapshot numbers that exceed the f64 range.

// log10 of a Num; -Infinity for missing/zero/non-positive values (so
// comparisons like `numLog10(x) > t` are false for "no value").
export function numLog10(num) {
  if (!num || num.m <= 0) return -Infinity;
  return Math.log10(num.m) + num.e;
}

// Whether a Num is present and positive.
export function gtZero(num) {
  return Boolean(num) && num.m > 0;
}

// Renormalize a raw (mantissa, exponent) pair so the mantissa lands in
// [1, 10); zero/non-finite mantissas collapse to { m: 0, e: 0 }.
export function normalizeNum(m, e) {
  if (m === 0 || !Number.isFinite(m)) return { m: 0, e: 0 };
  const shift = Math.floor(Math.log10(Math.abs(m)));
  return { m: m / Math.pow(10, shift), e: e + shift };
}

// Scale a Num by a plain f64 factor (e.g. per-minute rates), renormalized.
export function scaleNum(num, factor) {
  return normalizeNum(num.m * factor, num.e);
}

// Convert a plain float to a normalized Num. Non-finite values clamp to
// 1e308 (a display ceiling for the rare Infinity readout).
export function floatToNum(f) {
  if (!Number.isFinite(f)) return { m: 1, e: 308 };
  if (f === 0) return { m: 0, e: 0 };
  const e = Math.floor(Math.log10(Math.abs(f)));
  return { m: f / 10 ** e, e };
}

// Rebuild a Num from a log10 value (for quantities computed in log space).
export function numFromLog10(lg) {
  const e = Math.floor(lg);
  return { m: Math.pow(10, lg - e), e };
}

// Arithmetic mean of Nums: sum mantissas relative to the largest exponent
// (terms far below it underflow to 0 — negligible in a mean), renormalized.
export function averageNums(nums) {
  const maxE = Math.max(...nums.map((n) => n.e));
  const sum = nums.reduce((acc, n) => acc + n.m * Math.pow(10, n.e - maxE), 0);
  return normalizeNum(sum / nums.length, maxE);
}
