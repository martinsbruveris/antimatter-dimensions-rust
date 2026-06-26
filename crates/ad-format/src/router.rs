//! The exponent-based routing that drives every notation — a port of the base
//! `Notation.format` method.
//!
//! `format` dispatches on the value's base-10 exponent into the very-small,
//! under-1000, infinite, and big-number cases, handling sign uniformly so each
//! [`NotationStrategy`](crate::notations::NotationStrategy) only implements the
//! positive big-number case.

use break_infinity::Decimal;

use crate::notations::strategy;
use crate::options::FormatOptions;

/// Rendered for values at or above `opts.inf_threshold`.
const INFINITE: &str = "Infinite";

/// Format `value` according to `opts`.
///
/// Pure and presentation-only: it reads nothing but its arguments.
pub fn format(value: &Decimal, opts: &FormatOptions) -> String {
    let strat = strategy(opts.notation);
    let exponent = value.exponent();

    // Very small: switch to small formatting at 1e-300 (precision loss below that).
    if exponent < -300 {
        return if value.sign() < 0.0 {
            format!(
                "-{}",
                strat.format_very_small(&value.abs(), opts.places_under_1000)
            )
        } else {
            strat.format_very_small(value, opts.places_under_1000)
        };
    }

    // Under 1000: plain fixed-point on the f64 value.
    if exponent < 3 {
        let number = value.to_f64();
        return if number < 0.0 {
            format!(
                "-{}",
                strat.format_under_1000(-number, opts.places_under_1000)
            )
        } else {
            strat.format_under_1000(number, opts.places_under_1000)
        };
    }

    // Infinite: caller-supplied threshold (e.g. NUMBER_MAX_VALUE pre-break).
    if let Some(threshold) = &opts.inf_threshold {
        if value.abs() >= *threshold {
            return if value.sign() < 0.0 {
                format!("-{INFINITE}")
            } else {
                INFINITE.to_string()
            };
        }
    }

    // Big number: hand the positive value to the notation strategy.
    if value.sign() < 0.0 {
        format!(
            "-{}",
            strat.format_decimal(&value.abs(), opts.places, opts.places_exponent, opts)
        )
    } else {
        strat.format_decimal(value, opts.places, opts.places_exponent, opts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::Notation;

    fn under_1000_opts(places_under_1000: i32) -> FormatOptions {
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
        // Below 1e-300 the f64 value underflows to 0, matching the JS fallback.
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
}
