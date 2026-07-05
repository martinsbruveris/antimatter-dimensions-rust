// Number formatting, backed by the `ad-format` Rust crate compiled to WASM.
//
// The snapshot ships raw numbers ({ m, e } = mantissa × 10^exponent); these
// helpers render them in-process using the player's current notation, so no
// formatting crosses the Tauri IPC boundary. See
// docs/design/2026-06-25-number-formatting.md (Option C).
//
// `init` (in main.js) must finish before any of these run; component renders
// happen after the app mounts, so that ordering holds.
import { format as wasmFormat } from "../wasm/ad_format.js";
import { useGameStore } from "../stores/game";

// Active notation name from player options (falls back before the first snapshot).
function currentNotation() {
  return useGameStore().snapshot?.options?.notation ?? "Standard";
}

// The player's Exponent Notation thresholds (comma / in-notation digit counts).
function exponentDigits() {
  const o = useGameStore().snapshot?.options;
  return [o?.notation_digits_comma ?? 5, o?.notation_digits_notation ?? 9];
}

// Pre-break, values at or above Number.MAX_VALUE render as "Infinite" — the
// original's notation threshold (`player.break` lifts it).
function showInfinite() {
  return !(useGameStore().snapshot?.broke_infinity ?? false);
}

// Format a raw snapshot number ({ m, e }). `places` = mantissa digits for
// numbers ≥ 1000; `placesUnder1000` = digits for smaller numbers. Defaults
// mirror the original game's common `format(value, 2)` call.
export function formatDecimal(num, places = 2, placesUnder1000 = 0) {
  if (!num) return "0";
  const [comma, notation] = exponentDigits();
  return wasmFormat(
    num.m, num.e, currentNotation(), places, placesUnder1000, comma, notation,
    showInfinite(),
  );
}

// A multiplier (`×N`). Two decimal places below 1000, matching the original's
// `formatX(value, 2, 2)` — the form used for the per-dimension multiplier
// (ModernAntimatterDimensionRow.vue) and the Buy-10 / sacrifice multipliers
// (ModernAntimatterDimensionsTab.vue). The caller supplies the leading `×`.
export function formatMultiplier(num) {
  return formatDecimal(num, 2, 2);
}

// Format a duration in milliseconds, mirroring the original `TimeSpan.toString()`:
// at or above 10 s it lists non-zero year/day/hour/minute/second components joined
// by commas and "and" (the original's `toStringNoDecimals`); below that it shows a
// short seconds/ms form. Used by the offline-mode readout and catch-up summary.
export function formatTime(ms) {
  const totalSeconds = ms / 1000;
  if (totalSeconds < 1) return `${Math.round(ms)} ms`;
  if (totalSeconds < 10) return `${totalSeconds.toFixed(3)} seconds`;

  const years = Math.floor(ms / 31536e6);
  const days = Math.floor((ms / 864e5) % 365);
  const hours = Math.floor((ms / 36e5) % 24);
  const minutes = Math.floor((ms / 6e4) % 60);
  const seconds = Math.floor((ms / 1e3) % 60);

  const parts = [];
  const add = (value, name) => {
    if (value !== 0) parts.push(`${value} ${name}${value === 1 ? "" : "s"}`);
  };
  add(years, "year");
  add(days, "day");
  add(hours, "hour");
  add(minutes, "minute");
  add(seconds, "second");

  if (parts.length === 0) return "0 seconds";
  if (parts.length < 2) return parts[0];
  return `${parts.slice(0, -1).join(", ")} and ${parts.slice(-1)[0]}`;
}

// Short duration form, mirroring the original `TimeSpan.toStringShort` (used by
// `timeDisplayShort`, e.g. the bottom-left save timer): sub-second → "N ms",
// under 10 s → 3 decimals, under a minute → 2 decimals, then HH:MM:SS (MM:SS
// under an hour) up to 100 hours, then days/years. Values here stay small (the
// save timer resets each autosave), so plain number formatting matches the
// original's `format(...)` output without the notation machinery.
export function timeDisplayShort(ms) {
  const totalSeconds = ms / 1000;
  if (totalSeconds < 1) return `${Math.round(1000 * totalSeconds)} ms`;
  if (totalSeconds < 10) return `${totalSeconds.toFixed(3)} seconds`;
  if (totalSeconds < 60) return `${totalSeconds.toFixed(2)} seconds`;

  const totalHours = ms / 36e5;
  const pad = (v) => (v < 10 ? `0${v}` : `${v}`);
  if (totalHours < 100) {
    const hours = Math.floor(totalHours);
    const minutes = Math.floor((ms / 6e4) % 60);
    const seconds = Math.floor((ms / 1e3) % 60);
    return hours === 0
      ? `${pad(minutes)}:${pad(seconds)}`
      : `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`;
  }

  const totalDays = ms / 864e5;
  if (totalDays < 500) return `${totalDays.toFixed(2)} days`;
  return `${(ms / 31536e6).toFixed(2)} years`;
}

// Format a sample number for the Exponent Notation modal's live preview, using
// the slider's in-flight `commaDigits`/`notationDigits` (not the stored ones, so
// dragging updates immediately) with the current notation. Mirrors the
// original's `formatPostBreak(num)` — no mantissa places, so a pure power of ten
// reads as e.g. "1e1234".
export function formatExponentSample(num, commaDigits, notationDigits) {
  if (!num) return "0";
  // Never "Infinite": the preview's sample numbers exceed 1e308 by design.
  return wasmFormat(num.m, num.e, currentNotation(), 0, 0, commaDigits, notationDigits, false);
}
