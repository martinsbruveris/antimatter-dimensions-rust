//! Logarithm notation: `e` followed by the base-10 logarithm of the value.

use break_infinity::Decimal;

use super::NotationStrategy;
use crate::exponent::format_with_commas;
use crate::mantissa::format_mantissa;
use crate::options::FormatOptions;

pub(crate) struct Logarithm;

impl NotationStrategy for Logarithm {
    fn name(&self) -> &'static str {
        "Logarithm"
    }

    fn format_decimal(&self, value: &Decimal, opts: &FormatOptions) -> String {
        format!("e{}", self.format_log_exponent(value.log10(), opts))
    }
}

impl Logarithm {
    /// The base `formatExponent` specialised for Logarithm: the "exponent" is the raw
    /// (fractional) `log10`, so it is rendered with `toFixed` rather than as an
    /// integer, the precision comes from `places` (not `places_exponent`), and the
    /// recursive case re-enters `format_decimal` — re-prepending the leading `e`.
    fn format_log_exponent(&self, log10: f64, opts: &FormatOptions) -> String {
        let display = &opts.exponent_display;
        // Below `min`: render the log to `max(places, 1)` decimal places.
        if log10 < display.min as f64 {
            return format_mantissa(log10, opts.places.max(1));
        }
        // Below `max` (when enabled): comma-group the integer-rounded log.
        if display.show && log10 < display.max as f64 {
            return format_with_commas(&format_mantissa(log10, 0));
        }
        // Otherwise recurse, formatting the log itself in this notation (with both
        // place counts set to `places_exponent`, mirroring the game).
        let large = opts.places_exponent;
        let exp_opts = FormatOptions {
            places: large,
            places_exponent: large,
            ..opts.clone()
        };
        self.format_decimal(&Decimal::from_float(log10), &exp_opts)
    }
}
