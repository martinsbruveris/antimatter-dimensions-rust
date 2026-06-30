//! WebAssembly bindings for the web frontend.
//!
//! Compiled only under the `wasm` feature (via `wasm-pack`), this exposes a
//! single synchronous [`format`] the Vue components call in-process — no IPC.
//! The snapshot ships raw `mantissa × 10^exponent` pairs and JS formats them,
//! per `design-docs/2026-06-25-number-formatting.md` (Option C).

use break_infinity::Decimal;
use wasm_bindgen::prelude::*;

use crate::options::{ExponentDisplay, FormatOptions, Notation};
use crate::router;

/// Map a notation name (case-insensitive, as stored in player options) to its
/// strategy. Unknown names fall back to the `FormatOptions` default.
fn notation_from_str(name: &str) -> Notation {
    match name.to_ascii_lowercase().as_str() {
        "scientific" => Notation::Scientific,
        "engineering" => Notation::Engineering,
        "standard" => Notation::Standard,
        "letters" => Notation::Letters,
        "mixed scientific" => Notation::MixedScientific,
        "mixed engineering" => Notation::MixedEngineering,
        "logarithm" => Notation::Logarithm,
        "infinity" => Notation::Infinity,
        _ => Notation::default(),
    }
}

/// 10^`digits`, the threshold an exponent must reach for the next display tier.
/// `digits` is a slider value in [3, 15], so the result never overflows `i64`.
fn threshold(digits: u32) -> i64 {
    10i64.saturating_pow(digits)
}

/// Format a number given as `mantissa × 10^exponent`.
///
/// `mantissa`/`exponent` come straight off a `Decimal` in the snapshot (the
/// exponent is an `i64` widened to `f64`, exact for every in-game magnitude).
/// `notation` is the player's notation name; `places`/`places_under_1000` are the
/// per-call-site digit counts. `comma_digits`/`notation_digits` are the player's
/// Exponent Notation thresholds: the exponent gets commas at 10^`comma_digits`
/// and switches to in-notation at 10^`notation_digits` (see [`FormatOptions`]).
#[wasm_bindgen]
pub fn format(
    mantissa: f64,
    exponent: f64,
    notation: &str,
    places: u32,
    places_under_1000: u32,
    comma_digits: u32,
    notation_digits: u32,
) -> String {
    let value = Decimal::new(mantissa, exponent as i64);
    let opts = FormatOptions {
        notation: notation_from_str(notation),
        places,
        places_under_1000,
        exponent_display: ExponentDisplay {
            show: true,
            min: threshold(comma_digits),
            max: threshold(notation_digits),
        },
        ..FormatOptions::default()
    };
    router::format(&value, &opts)
}
