//! Notation strategies and the per-notation dispatch.
//!
//! Each notation implements [`NotationStrategy`]. The base `Notation.format`
//! routing (very-small / under-1000 / negative / infinite) lives in
//! [`crate::router`]; a strategy only supplies `format_decimal` for the big-number
//! case, and may override the under-1000 / very-small fallbacks if it formats small
//! numbers specially (none of the M1 four do).

mod engineering;
mod letters;
mod scientific;
mod standard;

use break_infinity::Decimal;

use crate::exponent::{
    exponent_to_string, format_with_commas, no_special_formatting, show_commas,
};
use crate::mantissa::{
    format_mantissa_base_ten, format_mantissa_with_exponent, MantissaSpec,
};
use crate::options::{FormatOptions, Notation};

pub(crate) use engineering::Engineering;
pub(crate) use letters::Letters;
pub(crate) use scientific::Scientific;
pub(crate) use standard::Standard;

/// A single notation strategy (port of the notations library's `Notation`
/// subclasses). Implementors are zero-sized and shared as `&'static`.
pub(crate) trait NotationStrategy: Sync {
    /// Display name, matching the JS notation `name` (used for fidelity lookup).
    // Wired up by the `ad-fidelity` `format()` harness in a later step.
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Format the big-number case: `value` is positive (the router has already
    /// taken `abs` and will re-apply the sign), with base-10 exponent >= 3 and
    /// below any infinite threshold. Port of the abstract `formatDecimal`.
    fn format_decimal(
        &self,
        value: &Decimal,
        places: i32,
        places_exponent: i32,
        opts: &FormatOptions,
    ) -> String;

    /// Numbers with |exponent| < 3. Default: `value.toFixed(places)`
    /// (port of base `formatUnder1000`).
    fn format_under_1000(&self, value: f64, places: i32) -> String {
        format_mantissa_base_ten(value, places)
    }

    /// Very small values (exponent < -300). Default delegates to
    /// `format_under_1000(value.to_f64(), places)` (port of base
    /// `formatVerySmallDecimal`).
    fn format_very_small(&self, value: &Decimal, places: i32) -> String {
        self.format_under_1000(value.to_f64(), places)
    }

    /// Recursive exponent formatting (port of base `formatExponent`, with the
    /// default `specialFormat = n => n.toString()`): plain below `min`,
    /// comma-grouped below `max`, otherwise formatted in this notation.
    fn format_exponent(
        &self,
        exponent: f64,
        precision: i32,
        opts: &FormatOptions,
    ) -> String {
        if no_special_formatting(exponent, &opts.exponent_commas) {
            return exponent_to_string(exponent);
        }
        if show_commas(exponent, &opts.exponent_commas) {
            return format_with_commas(&exponent_to_string(exponent));
        }
        let large = precision.max(2);
        self.format_decimal(&Decimal::from_float(exponent), large, large, opts)
    }

    /// Shared base-10 mantissa/exponent rendering for Scientific (`steps == 1`) and
    /// Engineering (`steps == 3`).
    fn format_base_ten_exponent(
        &self,
        value: &Decimal,
        places: i32,
        places_exponent: i32,
        opts: &FormatOptions,
        steps: i32,
    ) -> String {
        format_mantissa_with_exponent(
            value,
            places,
            places_exponent,
            &MantissaSpec {
                base: 10.0,
                steps,
                separator: "e",
                force_positive_exponent: false,
                use_log_if_exponent_is_formatted: false,
            },
            format_mantissa_base_ten,
            |exp, prec| self.format_exponent(exp, prec, opts),
            &opts.exponent_commas,
        )
    }
}

/// Map a [`Notation`] to its (statically allocated) strategy.
pub(crate) fn strategy(notation: Notation) -> &'static dyn NotationStrategy {
    match notation {
        Notation::Scientific => &Scientific,
        Notation::Engineering => &Engineering,
        Notation::Standard => &Standard,
        Notation::Letters => &Letters,
    }
}
