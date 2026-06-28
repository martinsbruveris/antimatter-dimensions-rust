//! WebAssembly bindings for the web frontend.
//!
//! Compiled only under the `wasm` feature (via `wasm-pack`), this exposes a
//! single synchronous [`format`] the Vue components call in-process — no IPC.
//! The snapshot ships raw `mantissa × 10^exponent` pairs and JS formats them,
//! per `design-docs/2026-06-25-number-formatting.md` (Option C).

use break_infinity::Decimal;
use wasm_bindgen::prelude::*;

use crate::options::{FormatOptions, Notation};
use crate::router;

/// Map a notation name (case-insensitive, as stored in player options) to its
/// strategy. Unknown names fall back to the `FormatOptions` default.
fn notation_from_str(name: &str) -> Notation {
    match name.to_ascii_lowercase().as_str() {
        "scientific" => Notation::Scientific,
        "engineering" => Notation::Engineering,
        "standard" => Notation::Standard,
        "letters" => Notation::Letters,
        _ => Notation::default(),
    }
}

/// Format a number given as `mantissa × 10^exponent`.
///
/// `mantissa`/`exponent` come straight off a `Decimal` in the snapshot (the
/// exponent is an `i64` widened to `f64`, exact for every in-game magnitude).
/// `notation` is the player's notation name; `places`/`places_under_1000` are the
/// per-call-site digit counts (see [`FormatOptions`]).
#[wasm_bindgen]
pub fn format(
    mantissa: f64,
    exponent: f64,
    notation: &str,
    places: u32,
    places_under_1000: u32,
) -> String {
    let value = Decimal::new(mantissa, exponent as i64);
    let opts = FormatOptions {
        notation: notation_from_str(notation),
        places,
        places_under_1000,
        ..FormatOptions::default()
    };
    router::format(&value, &opts)
}
