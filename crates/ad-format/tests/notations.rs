//! Fidelity tests for the notation strategies.
//!
//! Expected strings are generated from the real `@antimatter-dimensions/notations`
//! package (mirroring the game call `format(value, places, 0, 3)`); see the crate's
//! design doc. Regenerate with a node script requiring the package in
//! `../antimatter-dimensions`.

use ad_format::{format, FormatOptions, Notation};
use break_infinity::Decimal;

/// `FormatOptions` for `notation` with the given mantissa `places` (defaults
/// otherwise: `places_under_1000 = 0`, `places_exponent = 3`).
fn opts(notation: Notation, places: u32) -> FormatOptions {
    FormatOptions {
        places,
        ..FormatOptions::new(notation)
    }
}

fn sci(value: Decimal, places: u32) -> String {
    format(&value, &opts(Notation::Scientific, places))
}

fn eng(value: Decimal, places: u32) -> String {
    format(&value, &opts(Notation::Engineering, places))
}

fn std_(value: Decimal, places: u32) -> String {
    format(&value, &opts(Notation::Standard, places))
}

fn letters(value: Decimal, places: u32) -> String {
    format(&value, &opts(Notation::Letters, places))
}

fn mixed_sci(value: Decimal, places: u32) -> String {
    format(&value, &opts(Notation::MixedScientific, places))
}

fn mixed_eng(value: Decimal, places: u32) -> String {
    format(&value, &opts(Notation::MixedEngineering, places))
}

fn logarithm(value: Decimal, places: u32) -> String {
    format(&value, &opts(Notation::Logarithm, places))
}

fn infinity(value: Decimal, places: u32) -> String {
    format(&value, &opts(Notation::Infinity, places))
}

#[test]
fn scientific_places_2() {
    assert_eq!(sci(Decimal::new(1.5, 10), 2), "1.50e10");
    assert_eq!(sci(Decimal::new(1.23456, 100), 2), "1.23e100");
    assert_eq!(sci(Decimal::new(9.999, 9), 2), "1.00e10"); // mantissa roll-over
    assert_eq!(sci(Decimal::new(9.9999, 3), 2), "1.00e4"); // roll-over at low exp
    assert_eq!(sci(Decimal::new(1.0, 3), 2), "1.00e3");
    assert_eq!(sci(Decimal::from_float(1234.0), 2), "1.23e3");
    assert_eq!(sci(Decimal::new(1.0, 5), 2), "1.00e5");
    assert_eq!(sci(Decimal::new(6.7, 21), 2), "6.70e21");
    assert_eq!(sci(Decimal::new(1.0, 1000), 2), "1.00e1000");
}

#[test]
fn scientific_places_0() {
    assert_eq!(sci(Decimal::new(1.5, 10), 0), "2e10");
    assert_eq!(sci(Decimal::new(9.999, 9), 0), "1e10");
    assert_eq!(sci(Decimal::from_float(1234.0), 0), "1e3");
}

#[test]
fn engineering_places_2() {
    assert_eq!(eng(Decimal::new(1.5, 10), 2), "15.00e9");
    assert_eq!(eng(Decimal::new(1.23456, 100), 2), "12.35e99");
    assert_eq!(eng(Decimal::new(9.999, 9), 2), "10.00e9");
    assert_eq!(eng(Decimal::new(9.9999, 3), 2), "10.00e3");
    assert_eq!(eng(Decimal::new(1.0, 3), 2), "1.00e3");
    assert_eq!(eng(Decimal::new(1.0, 5), 2), "100.00e3");
    assert_eq!(eng(Decimal::new(1.5, 5), 2), "150.00e3");
    assert_eq!(eng(Decimal::new(1.0, 100), 2), "10.00e99");
    assert_eq!(eng(Decimal::new(6.7, 21), 2), "6.70e21");
}

#[test]
fn engineering_places_0() {
    assert_eq!(eng(Decimal::new(1.5, 10), 0), "15e9");
    assert_eq!(eng(Decimal::new(9.999, 9), 0), "10e9");
    assert_eq!(eng(Decimal::new(1.0, 5), 0), "100e3");
}

#[test]
fn standard_places_2() {
    assert_eq!(std_(Decimal::new(1.0, 3), 2), "1.00 K");
    assert_eq!(std_(Decimal::from_float(1234.0), 2), "1.23 K");
    assert_eq!(std_(Decimal::new(1.0, 6), 2), "1.00 M");
    assert_eq!(std_(Decimal::new(1.5, 6), 2), "1.50 M");
    assert_eq!(std_(Decimal::new(1.0, 9), 2), "1.00 B");
    assert_eq!(std_(Decimal::new(1.0, 30), 2), "1.00 No");
    assert_eq!(std_(Decimal::new(1.0, 33), 2), "1.00 Dc");
    assert_eq!(std_(Decimal::new(1.0, 36), 2), "1.00 UDc"); // multi-letter prefix
    assert_eq!(std_(Decimal::new(1.0, 63), 2), "1.00 Vg");
    assert_eq!(std_(Decimal::new(9.999, 8), 2), "999.90 M"); // no roll-over
    assert_eq!(std_(Decimal::new(1.0, 303), 2), "1.00 Ce");
    assert_eq!(std_(Decimal::new(1.0, 3003), 2), "1.00 MI"); // regex cleanup path
}

#[test]
fn standard_places_0() {
    assert_eq!(std_(Decimal::new(1.0, 3), 0), "1 K");
    assert_eq!(std_(Decimal::new(1.5, 6), 0), "2 M");
    assert_eq!(std_(Decimal::new(1.0, 9), 0), "1 B");
}

#[test]
fn letters_places_2() {
    assert_eq!(letters(Decimal::new(1.0, 3), 2), "1.00a");
    assert_eq!(letters(Decimal::from_float(1234.0), 2), "1.23a");
    assert_eq!(letters(Decimal::new(1.5, 10), 2), "15.00c");
    assert_eq!(letters(Decimal::new(1.0, 6), 2), "1.00b");
    assert_eq!(letters(Decimal::new(1.0, 78), 2), "1.00z");
    assert_eq!(letters(Decimal::new(1.0, 81), 2), "1.00aa"); // base-26 carry
    assert_eq!(letters(Decimal::new(1.0, 84), 2), "1.00ab");
    assert_eq!(letters(Decimal::new(1.0, 2028), 2), "1.00yz"); // remainder == 0 case
    assert_eq!(letters(Decimal::new(1.0, 2031), 2), "1.00za");
    assert_eq!(letters(Decimal::new(9.999, 8), 2), "999.90b");
}

#[test]
fn letters_places_0() {
    assert_eq!(letters(Decimal::new(1.5, 10), 0), "15c");
    assert_eq!(letters(Decimal::new(1.0, 78), 0), "1z");
}

#[test]
fn comma_grouped_exponents() {
    // `min` (1e5) <= exponent < `max` (1e9): the exponent itself is rendered in
    // full with thousands separators rather than recursively in notation.
    assert_eq!(sci(Decimal::new(1.0, 100_000), 2), "1.00e100,000");
    assert_eq!(sci(Decimal::new(1.5, 100_000), 2), "1.50e100,000");
    assert_eq!(sci(Decimal::new(1.0, 1_234_567), 2), "1.00e1,234,567");
    assert_eq!(sci(Decimal::new(1.0, 100_000), 0), "1e100,000");
    // Engineering rebases to a multiple of 3: 100001 -> 99999 (mantissa 100),
    // which falls below `min` and so prints plain (no commas).
    assert_eq!(eng(Decimal::new(1.0, 100_001), 2), "100.00e99999");
    assert_eq!(eng(Decimal::new(1.0, 100_002), 2), "1.00e100,002");
    assert_eq!(std_(Decimal::new(1.0, 100_002), 2), "1.00 TTgMI-TTgTc");
}

#[test]
fn recursive_notation_exponents() {
    // exponent >= `max` (1e9): the exponent is itself formatted in this notation
    // (with `places_exponent` = 3), nested after the mantissa. The mantissa keeps
    // its `places` (these notations do not drop it when the exponent is formatted).
    assert_eq!(sci(Decimal::new(1.0, 1_000_000_000), 2), "1.00e1.000e9");
    assert_eq!(sci(Decimal::new(1.23, 1_000_000_000), 2), "1.23e1.000e9");
    assert_eq!(
        sci(Decimal::new(1.0, 1_230_000_000_000_000), 2),
        "1.00e1.230e15"
    );
    assert_eq!(sci(Decimal::new(1.0, 1_000_000_000), 0), "1e1.000e9");
    // Mantissa roll-over pushes the exponent across the recursion boundary.
    assert_eq!(sci(Decimal::new(9.999, 999_999_999), 2), "1.00e1.000e9");
    // Engineering rebases 1e9 down to 999,999,999 (mantissa 10), which is < `max`
    // and so comma-grouped rather than recursive — a boundary the step-3 split moves.
    assert_eq!(
        eng(Decimal::new(1.0, 1_000_000_000), 2),
        "10.00e999,999,999"
    );
    assert_eq!(
        eng(Decimal::new(1.0, 1_230_000_000_000_000), 2),
        "1.00e1.230e15"
    );
}

#[test]
fn mixed_scientific_places_2() {
    // Below 1e33: Standard letter abbreviations.
    assert_eq!(mixed_sci(Decimal::new(1.0, 3), 2), "1.00 K");
    assert_eq!(mixed_sci(Decimal::new(1.234, 3), 2), "1.23 K");
    assert_eq!(mixed_sci(Decimal::new(1.5, 6), 2), "1.50 M");
    assert_eq!(mixed_sci(Decimal::new(1.5, 10), 2), "15.00 B");
    assert_eq!(mixed_sci(Decimal::new(6.7, 21), 2), "6.70 Sx");
    assert_eq!(mixed_sci(Decimal::new(1.0, 30), 2), "1.00 No");
    assert_eq!(mixed_sci(Decimal::new(1.0, 32), 2), "100.00 No");
    assert_eq!(mixed_sci(Decimal::new(9.999, 8), 2), "999.90 M");
    // At/above 1e33: scientific.
    assert_eq!(mixed_sci(Decimal::new(1.0, 33), 2), "1.00e33");
    assert_eq!(mixed_sci(Decimal::new(1.0, 36), 2), "1.00e36");
    assert_eq!(mixed_sci(Decimal::new(1.0, 100), 2), "1.00e100");
    assert_eq!(mixed_sci(Decimal::new(1.0, 100_000), 2), "1.00e100,000");
    // Recursive exponent renders Standard-style (Mixed recurses into itself).
    assert_eq!(
        mixed_sci(Decimal::new(1.0, 1_000_000_000), 2),
        "1.00e1.000 B"
    );
    assert_eq!(
        mixed_sci(Decimal::new(1.23, 1_000_000_000), 2),
        "1.23e1.000 B"
    );
    assert_eq!(
        mixed_sci(Decimal::new(1.0, 1_230_000_000_000_000), 2),
        "1.00e1.230 Qa"
    );
}

#[test]
fn mixed_scientific_places_0() {
    assert_eq!(mixed_sci(Decimal::new(1.5, 6), 0), "2 M");
    assert_eq!(mixed_sci(Decimal::new(6.7, 21), 0), "7 Sx");
    assert_eq!(mixed_sci(Decimal::new(1.0, 33), 0), "1e33");
    assert_eq!(mixed_sci(Decimal::new(1.0, 1_000_000_000), 0), "1e1.000 B");
}

#[test]
fn mixed_engineering_places_2() {
    // Below 1e33: identical to Mixed scientific (Standard).
    assert_eq!(mixed_eng(Decimal::new(1.5, 6), 2), "1.50 M");
    assert_eq!(mixed_eng(Decimal::new(1.0, 32), 2), "100.00 No");
    assert_eq!(mixed_eng(Decimal::new(9.999, 8), 2), "999.90 M");
    // At/above 1e33: engineering (exponent forced to multiples of 3).
    assert_eq!(mixed_eng(Decimal::new(1.0, 33), 2), "1.00e33");
    assert_eq!(mixed_eng(Decimal::new(1.0, 100), 2), "10.00e99");
    assert_eq!(mixed_eng(Decimal::new(1.0, 100_000), 2), "10.00e99999");
    assert_eq!(
        mixed_eng(Decimal::new(1.0, 1_000_000_000), 2),
        "10.00e999,999,999"
    );
    assert_eq!(
        mixed_eng(Decimal::new(1.0, 1_230_000_000_000_000), 2),
        "1.00e1.230 Qa"
    );
}

#[test]
fn mixed_engineering_places_0() {
    assert_eq!(mixed_eng(Decimal::new(1.0, 100), 0), "10e99");
    assert_eq!(
        mixed_eng(Decimal::new(1.23, 1_000_000_000), 0),
        "12e999,999,999"
    );
}

#[test]
fn logarithm_places_2() {
    assert_eq!(logarithm(Decimal::new(1.0, 3), 2), "e3.00");
    assert_eq!(logarithm(Decimal::new(1.234, 3), 2), "e3.09");
    assert_eq!(logarithm(Decimal::new(1.5, 6), 2), "e6.18");
    assert_eq!(logarithm(Decimal::new(6.7, 21), 2), "e21.83");
    assert_eq!(logarithm(Decimal::new(1.0, 100), 2), "e100.00");
    // Comma-grouped once log10 reaches `min` (1e5).
    assert_eq!(logarithm(Decimal::new(1.0, 100_000), 2), "e100,000");
    assert_eq!(logarithm(Decimal::new(1.0, 1_234_567), 2), "e1,234,567");
    // Recursive once log10 reaches `max` (1e9): the log is itself logarithm-formatted.
    assert_eq!(logarithm(Decimal::new(1.0, 1_000_000_000), 2), "ee9.000");
    assert_eq!(
        logarithm(Decimal::new(1.0, 1_230_000_000_000_000), 2),
        "ee15.090"
    );
}

#[test]
fn logarithm_places_0() {
    // Below `min` always keeps at least one decimal (`max(places, 1)`).
    assert_eq!(logarithm(Decimal::new(1.0, 3), 0), "e3.0");
    assert_eq!(logarithm(Decimal::new(1.234, 3), 0), "e3.1");
    assert_eq!(logarithm(Decimal::new(1.5, 6), 0), "e6.2");
    assert_eq!(logarithm(Decimal::new(1.0, 100_000), 0), "e100,000");
    assert_eq!(logarithm(Decimal::new(1.0, 1_000_000_000), 0), "ee9.000");
}

#[test]
fn infinity_notation_places_2() {
    assert_eq!(infinity(Decimal::new(1.0, 3), 2), "0.0097\u{221e}");
    assert_eq!(infinity(Decimal::new(1.234, 3), 2), "0.0100\u{221e}");
    assert_eq!(infinity(Decimal::new(1.0, 9), 2), "0.0292\u{221e}");
    assert_eq!(infinity(Decimal::new(1.0, 100), 2), "0.3244\u{221e}");
    // Three decimals (and commas) once the count passes 1000.
    assert_eq!(infinity(Decimal::new(1.0, 100_000), 2), "324.4070\u{221e}");
    assert_eq!(
        infinity(Decimal::new(1.0, 1_234_567), 2),
        "4,005.022\u{221e}"
    );
    assert_eq!(
        infinity(Decimal::new(1.0, 1_000_000_000), 2),
        "3,244,070.405\u{221e}"
    );
    assert_eq!(
        infinity(Decimal::new(1.0, 1_230_000_000_000_000), 2),
        "3,990,206,598,351.031\u{221e}"
    );
}

#[test]
fn infinity_notation_places_0() {
    // `places` only matters once it exceeds the built-in `infPlaces` (4 or 3).
    assert_eq!(infinity(Decimal::new(1.0, 3), 0), "0.0097\u{221e}");
    assert_eq!(
        infinity(Decimal::new(1.0, 1_234_567), 0),
        "4,005.022\u{221e}"
    );
}

#[test]
fn under_1000_and_sign_unaffected_by_notation() {
    // Both notations fall through to the shared under-1000 / very-small paths.
    for n in [Notation::Scientific, Notation::Engineering] {
        assert_eq!(format(&Decimal::from_float(42.0), &opts(n, 2)), "42");
        assert_eq!(format(&Decimal::from_float(-42.0), &opts(n, 2)), "-42");
        assert_eq!(format(&Decimal::new(-1.5, 10), &opts(n, 2)), {
            if n == Notation::Scientific {
                "-1.50e10"
            } else {
                "-15.00e9"
            }
        });
    }
}
