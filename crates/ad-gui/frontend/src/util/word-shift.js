// Text-garbling helpers, ported verbatim from the original game's
// `src/core/word-shift.js`. Used to obscure the last achievement row (the Pelle
// achievements, ids 181-188), which the original renders as a shimmering,
// unreadable mystery until the player is Doomed. We have no Pelle yet, so the
// row is permanently obscured — this is a purely cosmetic, frontend-only effect.

// Seeded pseudo-random in [0, 1), deterministic in `x`. Keeping the original's
// exact constants so the shimmer matches the source game.
function predictableRandom(x) {
  let start = Math.pow(x % 97, 4.3) * 232344573;
  const a = 15485863;
  const b = 521791;
  start = (start * a) % b;
  for (let i = 0; i < ((x * x) % 90) + 90; i++) {
    start = (start * a) % b;
  }
  return start / b;
}

// A random glyph from the accented-letter block (charCodes 192-241), matching
// the original's garble alphabet.
function randomSymbol() {
  return String.fromCharCode(Math.floor(Math.random() * 50) + 192);
}

// Replace a `frac` fraction of `str`'s characters with random symbols. The
// chosen indices change every ~0.5 s (keyed off `Date.now()`), producing the
// shimmer. Note the proportion randomized may be slightly under `frac`.
export function randomCrossWords(str, frac = 0.7) {
  if (frac <= 0) return str;
  const x = str.split("");
  for (let i = 0; i < x.length * frac; i++) {
    const randomIndex = Math.floor(
      predictableRandom((Math.floor(Date.now() / 500) % 964372) + 1.618 * i) *
        x.length
    );
    x[randomIndex] = randomSymbol();
  }
  return x.join("");
}
