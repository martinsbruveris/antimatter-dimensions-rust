//! The exponent-based routing that drives every notation.
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
    let sign = if value.sign() < 0.0 { "-" } else { "" };
    let exponent = value.exponent();

    // Infinite: caller-supplied threshold (e.g. NUMBER_MAX_VALUE pre-break).
    if let Some(threshold) = &opts.inf_threshold {
        if value >= threshold {
            return INFINITE.to_string();
        } else if -value >= *threshold {
            return "-".to_owned() + INFINITE;
        }
    }

    // Very small: switch to small formatting at 1e-300 (precision loss below that).
    if exponent < -300 {
        let abs = strat.format_very_small(&value.abs(), opts);
        return sign.to_owned() + &abs;
    }

    // Under 1000: plain fixed-point.
    if exponent < 3 {
        let abs = strat.format_under_1000(&value.abs(), opts);
        return sign.to_owned() + &abs;
    }

    // Big number: hand the positive value to the notation strategy.
    let abs = strat.format_decimal(&value.abs(), opts);
    sign.to_owned() + &abs
}

#[cfg(test)]
mod tests;
