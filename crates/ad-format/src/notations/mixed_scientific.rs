//! Mixed scientific notation: Standard below 1e33, Scientific above.

use break_infinity::Decimal;

use super::{NotationStrategy, Standard};
use crate::mantissa::{format_mantissa, format_mantissa_with_exponent, MantissaSpec};
use crate::options::FormatOptions;

pub(crate) struct MixedScientific;

impl NotationStrategy for MixedScientific {
    fn name(&self) -> &'static str {
        "Mixed scientific"
    }

    fn format_decimal(&self, value: &Decimal, opts: &FormatOptions) -> String {
        // Below 1e33 the game shows Standard's letter abbreviations; at or above it
        // switches to plain scientific. The scientific branch is Scientific's body
        // but routed through *our* `format_exponent`, so a recursively-formatted
        // exponent (>= the recursion threshold) renders Standard-style too — matching
        // the game, which binds `this.formatExponent` to the Mixed instance.
        if value.exponent() < 33 {
            return Standard.format_decimal(value, opts);
        }
        format_mantissa_with_exponent(
            value,
            &MantissaSpec {
                steps: 1,
                separator: "e",
                force_positive_exponent: false,
            },
            |m| format_mantissa(m, opts.places),
            |exp| self.format_exponent(exp, opts),
        )
    }
}
