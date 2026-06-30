//! Notation strategies and the per-notation dispatch.
//!
//! Each notation implements [`NotationStrategy`]. The shared routing
//! (very-small / under-1000 / negative / infinite) lives in
//! [`crate::router`]; a strategy only supplies `format_decimal` for the big-number
//! case, and may override the under-1000 / very-small fallbacks if it formats small
//! numbers specially (none of the M1 four do).

mod engineering;
mod infinity;
mod letters;
mod logarithm;
mod mixed_engineering;
mod mixed_scientific;
mod scientific;
mod standard;

use break_infinity::Decimal;

use crate::exponent::format_with_commas;
use crate::mantissa::format_mantissa;
use crate::options::{FormatOptions, Notation};

pub(crate) use engineering::Engineering;
pub(crate) use infinity::Infinity;
pub(crate) use letters::Letters;
pub(crate) use logarithm::Logarithm;
pub(crate) use mixed_engineering::MixedEngineering;
pub(crate) use mixed_scientific::MixedScientific;
pub(crate) use scientific::Scientific;
pub(crate) use standard::Standard;

/// A single notation strategy. Implementors are zero-sized and shared as `&'static`.
pub(crate) trait NotationStrategy: Sync {
    /// Display name of the notation (used for fidelity lookup).
    // Wired up by the `ad-fidelity` `format()` harness in a later step.
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Very small values (exponent < -300). We default to
    /// [`format_under_1000`](Self::format_under_1000) but some notations have special
    /// behaviour for tiny values.
    fn format_very_small(&self, value: &Decimal, opts: &FormatOptions) -> String {
        self.format_under_1000(value, opts)
    }

    /// Numbers with |exponent| < 3. We provide a default implementation, but
    /// specific strategies might overwrite it.
    fn format_under_1000(&self, value: &Decimal, opts: &FormatOptions) -> String {
        format_mantissa(value.to_f64(), opts.places_under_1000)
    }

    /// Format the big-number case: `value` is positive (the router has already
    /// taken `abs` and will re-apply the sign), with base-10 exponent >= 3 and
    /// below any infinite threshold.
    fn format_decimal(&self, value: &Decimal, opts: &FormatOptions) -> String;

    /// Recursive exponent formatting: plain below `min`, comma-grouped below `max`,
    /// otherwise formatted in this notation.
    fn format_exponent(&self, exponent: f64, opts: &FormatOptions) -> String {
        let display = &opts.exponent_display;
        let exponent_str = (exponent as i64).to_string();

        // Below `min`: render the exponent plain.
        if exponent < display.min as f64 {
            return exponent_str;
        }

        // Below `max` (when enabled): comma-group the exponent.
        if display.show && exponent < display.max as f64 {
            return format_with_commas(&exponent_str);
        }

        // The nested value reuses the surrounding options but with both place
        // counts bumped to `places_exponent.max(2)`.
        let large = opts.places_exponent.max(2);
        let exp_opts = FormatOptions {
            places: large,
            places_exponent: large,
            ..opts.clone()
        };
        self.format_decimal(&Decimal::from_float(exponent), &exp_opts)
    }
}

/// Map a [`Notation`] to its (statically allocated) strategy.
pub(crate) fn strategy(notation: Notation) -> &'static dyn NotationStrategy {
    match notation {
        Notation::Scientific => &Scientific,
        Notation::Engineering => &Engineering,
        Notation::Standard => &Standard,
        Notation::Letters => &Letters,
        Notation::MixedScientific => &MixedScientific,
        Notation::MixedEngineering => &MixedEngineering,
        Notation::Logarithm => &Logarithm,
        Notation::Infinity => &Infinity,
    }
}
