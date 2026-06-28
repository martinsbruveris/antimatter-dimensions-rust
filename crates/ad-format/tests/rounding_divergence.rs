//! Pinned, intentional divergence from the JS reference for exact half-way mantissas.
//!
//! Rust's `{:.*}` formatter rounds half-to-even; the JS
//! `@antimatter-dimensions/notations` reference uses `toFixed`, which rounds
//! half-away-from-zero. This is an accepted presentation-layer difference: it only
//! shows up when the mantissa is an exact binary half at the rounded digit (e.g.
//! `2.5`, `1.25`), a corner that is essentially never hit in-game.
//!
//! These assertions lock the current Rust behaviour as a regression guard; the
//! trailing comment on each line records what JS would print instead. (Unlike
//! `format_edge_cases.rs`, these are NOT checked against JS ground truth.) See the
//! `TODO(fidelity)` in `mantissa.rs` and `design-docs/2026-06-28-ad-format-test-plan.md`.

use ad_format::{format, FormatOptions, Notation};
use break_infinity::Decimal;

fn sci(value: &str, places: u32) -> String {
    let opts = FormatOptions {
        places,
        ..FormatOptions::new(Notation::Scientific)
    };
    format(&value.parse::<Decimal>().unwrap(), &opts)
}

#[test]
fn halfway_mantissa_rounds_half_to_even() {
    assert_eq!(sci("2.5e3", 0), "2e3"); // JS toFixed: "3e3"
    assert_eq!(sci("1.25e3", 1), "1.2e3"); // JS toFixed: "1.3e3"
    assert_eq!(sci("1.5e3", 0), "2e3"); // agrees with JS: "2e3"
}
