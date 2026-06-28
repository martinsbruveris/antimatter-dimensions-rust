// Number formatting, backed by the `ad-format` Rust crate compiled to WASM.
//
// The snapshot ships raw numbers ({ m, e } = mantissa × 10^exponent); these
// helpers render them in-process using the player's current notation, so no
// formatting crosses the Tauri IPC boundary. See
// design-docs/2026-06-25-number-formatting.md (Option C).
//
// `init` (in main.js) must finish before any of these run; component renders
// happen after the app mounts, so that ordering holds.
import { format as wasmFormat } from "../wasm/ad_format.js";
import { useGameStore } from "../stores/game";

// Active notation name from player options (falls back before the first snapshot).
function currentNotation() {
  return useGameStore().snapshot?.options?.notation ?? "Standard";
}

// Format a raw snapshot number ({ m, e }). `places` = mantissa digits for
// numbers ≥ 1000; `placesUnder1000` = digits for smaller numbers. Defaults
// mirror the original game's common `format(value, 2)` call.
export function formatDecimal(num, places = 2, placesUnder1000 = 0) {
  if (!num) return "0";
  return wasmFormat(num.m, num.e, currentNotation(), places, placesUnder1000);
}

// A multiplier (`×N`); keeps one decimal under 1000 like the original `formatX`.
export function formatMultiplier(num) {
  return formatDecimal(num, 2, 1);
}
