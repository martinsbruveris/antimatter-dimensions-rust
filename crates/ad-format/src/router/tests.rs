use super::*;
use crate::options::Notation;

fn under_1000_opts(places_under_1000: u32) -> FormatOptions {
    FormatOptions {
        places_under_1000,
        ..FormatOptions::default()
    }
}

#[test]
fn zero_uses_under_1000_path() {
    assert_eq!(format(&Decimal::ZERO, &under_1000_opts(0)), "0");
    assert_eq!(format(&Decimal::ZERO, &under_1000_opts(2)), "0.00");
}

#[test]
fn small_positive_is_fixed_point() {
    let v = Decimal::from_float(12.5);
    assert_eq!(format(&v, &under_1000_opts(2)), "12.50");
    assert_eq!(format(&v, &under_1000_opts(0)), "12");
}

#[test]
fn small_negative_gets_sign() {
    let v = Decimal::from_float(-42.0);
    assert_eq!(format(&v, &under_1000_opts(0)), "-42");
}

#[test]
fn boundary_999_is_under_1000() {
    let v = Decimal::from_float(999.0);
    assert_eq!(format(&v, &under_1000_opts(0)), "999");
}

#[test]
fn very_small_collapses_to_zero() {
    // Below 1e-300 the f64 value underflows to 0.
    let v = Decimal::new(1.0, -310);
    assert_eq!(format(&v, &under_1000_opts(0)), "0");
}

#[test]
fn infinite_threshold_caps_large_values() {
    let threshold = Decimal::pow10(308.0);
    let opts = FormatOptions {
        inf_threshold: Some(threshold),
        ..FormatOptions::new(Notation::Scientific)
    };
    let big = Decimal::new(1.0, 309);
    assert_eq!(format(&big, &opts), "Infinite");
    let neg_big = Decimal::new(-1.0, 309);
    assert_eq!(format(&neg_big, &opts), "-Infinite");
}
